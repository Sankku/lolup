//! Cuentas de LolUp: ingreso y alta con pantalla propia.
//!
//! La cuenta vive en el servicio de cuentas configurado con `ACCOUNTS_ISSUER`, `ACCOUNTS_CLIENT_ID` y
//! `ACCOUNTS_CLIENT_SECRET` (login directo, servidor a servidor). Acá solo queda una sesión: un token
//! aleatorio en una cookie `HttpOnly` y su hash en SQLite. La contraseña pasa por este proceso y nunca
//! se guarda ni se registra. Sin esas variables la web queda abierta, como en desarrollo.

use crate::riot::ApiError;
use crate::{AppState, Failure, Shared, now_ms};
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const COOKIE: &str = "lolup_sesion";
const SESSION_DAYS: i64 = 30;

pub struct Accounts {
    http: reqwest::Client,
    base: String,
    client_id: String,
    secret: String,
}

impl Accounts {
    pub fn from_env() -> Option<Self> {
        let var = |name: &str| std::env::var(name).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        Some(Self {
            http: reqwest::Client::builder().timeout(std::time::Duration::from_secs(15)).build().ok()?,
            base: var("ACCOUNTS_ISSUER")?.trim_end_matches('/').to_string(),
            client_id: var("ACCOUNTS_CLIENT_ID")?,
            secret: var("ACCOUNTS_CLIENT_SECRET")?,
        })
    }

    async fn call(&self, action: &str, body: Value) -> Result<Identity, (StatusCode, String)> {
        let unavailable = || (StatusCode::BAD_GATEWAY, "No pudimos verificar tu cuenta. Probá de nuevo en un rato.".to_string());
        let response = self
            .http
            .post(format!("{}/api/directo/{action}", self.base))
            .basic_auth(&self.client_id, Some(&self.secret))
            .json(&body)
            .send()
            .await
            .map_err(|_| unavailable())?;
        let status = response.status();
        let value: Value = response.json().await.map_err(|_| unavailable())?;
        if status.is_success() {
            return serde_json::from_value(value).map_err(|_| unavailable());
        }
        // Errores del jugador (datos, duplicados, límites): el mensaje ya viene listo para mostrar.
        match (status.as_u16(), value["message"].as_str()) {
            (400 | 401 | 403 | 409 | 429, Some(message)) => {
                Err((StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_REQUEST), message.to_string()))
            }
            _ => {
                eprintln!("cuentas: {action} respondió {status}");
                Err(unavailable())
            }
        }
    }
}

#[derive(Deserialize)]
struct Identity {
    sub: String,
    name: String,
}

#[derive(Serialize)]
struct User {
    name: String,
}

fn random_token() -> Option<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).ok()?;
    Some(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn hash(token: &str) -> String {
    Sha256::digest(token.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn cookie_token(headers: &HeaderMap) -> Option<String> {
    headers.get_all(header::COOKIE).iter().filter_map(|v| v.to_str().ok()).flat_map(|v| v.split(';')).find_map(|pair| {
        let (name, value) = pair.trim().split_once('=')?;
        (name == COOKIE && !value.is_empty()).then(|| value.to_string())
    })
}

fn session_cookie(token: &str, max_age: i64) -> HeaderValue {
    let value = format!("{COOKIE}={token}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age={max_age}");
    HeaderValue::from_str(&value).expect("cookie ASCII")
}

/// `(sub, nombre)` de la sesión de este pedido, si hay una vigente.
fn current(state: &AppState, headers: &HeaderMap) -> Option<(String, String)> {
    state.store.session(&hash(&cookie_token(headers)?), now_ms())
}

/// Con cuentas configuradas, las búsquedas piden sesión. Sin ellas la web queda abierta.
pub fn require(state: &AppState, headers: &HeaderMap) -> Result<(), Failure> {
    match state.accounts {
        Some(_) if current(state, headers).is_none() => Err(Failure(ApiError::Login)),
        _ => Ok(()),
    }
}

fn reject(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

pub async fn me(State(state): State<Shared>, headers: HeaderMap) -> Json<Value> {
    let user = current(&state, &headers).map(|(_, name)| User { name });
    Json(json!({ "enabled": state.accounts.is_some(), "user": user }))
}

#[derive(Deserialize)]
pub struct LoginForm {
    login: String,
    password: String,
}

#[derive(Deserialize)]
pub struct RegisterForm {
    name: String,
    email: String,
    password: String,
}

fn client_ip(headers: &HeaderMap) -> String {
    // Solo se llega por el túnel de Cloudflare, que pone la IP real del visitante.
    headers.get("cf-connecting-ip").and_then(|v| v.to_str().ok()).unwrap_or("desconocida").to_string()
}

async fn start(state: &AppState, headers: &HeaderMap, action: &str, mut body: Value) -> Response {
    let Some(accounts) = &state.accounts else {
        return reject(StatusCode::NOT_FOUND, "Las cuentas no están habilitadas en este servidor.");
    };
    body["ip"] = json!(client_ip(headers));
    let identity = match accounts.call(action, body).await {
        Ok(identity) => identity,
        Err((status, message)) => return reject(status, &message),
    };
    let Some(token) = random_token() else {
        return reject(StatusCode::INTERNAL_SERVER_ERROR, "No se pudo iniciar la sesión.");
    };
    let max_age = SESSION_DAYS * 24 * 3600;
    if state.store.add_session(&hash(&token), &identity.sub, &identity.name, now_ms() + max_age * 1000).is_err() {
        return reject(StatusCode::INTERNAL_SERVER_ERROR, "No se pudo iniciar la sesión.");
    }
    let status = if action == "crear" { StatusCode::CREATED } else { StatusCode::OK };
    let mut response = (status, Json(json!({ "user": User { name: identity.name } }))).into_response();
    response.headers_mut().insert(header::SET_COOKIE, session_cookie(&token, max_age));
    response
}

pub async fn login(State(state): State<Shared>, headers: HeaderMap, Json(form): Json<LoginForm>) -> Response {
    start(&state, &headers, "entrar", json!({ "login": form.login, "password": form.password })).await
}

pub async fn register(State(state): State<Shared>, headers: HeaderMap, Json(form): Json<RegisterForm>) -> Response {
    let body = json!({ "name": form.name, "email": form.email, "password": form.password });
    start(&state, &headers, "crear", body).await
}

pub async fn logout(State(state): State<Shared>, headers: HeaderMap) -> Response {
    if let Some(token) = cookie_token(&headers) {
        state.store.remove_session(&hash(&token));
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(header::SET_COOKIE, session_cookie("", 0));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_session_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, HeaderValue::from_static("otra=1; lolup_sesion=abc123; x=y"));
        assert_eq!(cookie_token(&headers).as_deref(), Some("abc123"));
        headers.insert(header::COOKIE, HeaderValue::from_static("lolup_sesion="));
        assert_eq!(cookie_token(&headers), None);
    }

    #[test]
    fn tokens_are_random_and_stored_hashed() {
        let (a, b) = (random_token().unwrap(), random_token().unwrap());
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert_eq!(hash(&a).len(), 64);
        assert_ne!(hash(&a), a);
        assert_eq!(hash(&a), hash(&a));
    }
}
