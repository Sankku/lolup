//! Cuentas de LolUp: alta, ingreso y sesiones, todo en la base propia del servidor.
//!
//! Las contraseñas se guardan con PBKDF2-HMAC-SHA256 (600.000 iteraciones, sal aleatoria). La sesión es
//! un token aleatorio en una cookie `HttpOnly`; en la base solo queda su hash. Las búsquedas piden sesión.
//!
//! ponytail: cuentas independientes por decisión del dueño (2026-11-10). Más adelante pueden pasar a la
//! cuenta de la red con el login directo de accounts (atidorfa/accounts#10) sin cambiar esta pantalla.

use crate::riot::ApiError;
use crate::store::Taken;
use crate::{AppState, Failure, Shared, now_ms};
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;

const COOKIE: &str = "lolup_sesion";
const SESSION_DAYS: i64 = 30;
const ITERATIONS: u32 = 600_000;
const BAD_CREDENTIALS: &str = "Usuario o contraseña incorrectos.";
const MINUTE: i64 = 60_000;

// --- Contraseñas ---------------------------------------------------------------------------------

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    (text.len() % 2 == 0)
        .then(|| (0..text.len()).step_by(2).map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok()).collect::<Option<Vec<u8>>>())
        .flatten()
}

/// PBKDF2-HMAC-SHA256 (RFC 8018). Los estados con las claves ya mezcladas se clonan en cada vuelta.
fn pbkdf2(password: &[u8], salt: &[u8], iterations: u32, out: &mut [u8]) {
    let mut key = [0u8; 64];
    if password.len() > 64 {
        key[..32].copy_from_slice(&Sha256::digest(password));
    } else {
        key[..password.len()].copy_from_slice(password);
    }
    let mut inner = Sha256::new();
    inner.update(key.map(|b| b ^ 0x36));
    let mut outer = Sha256::new();
    outer.update(key.map(|b| b ^ 0x5c));
    let hmac = |parts: &[&[u8]]| -> [u8; 32] {
        let mut i = inner.clone();
        for part in parts {
            i.update(part);
        }
        let mut o = outer.clone();
        o.update(i.finalize());
        o.finalize().into()
    };
    for (index, chunk) in out.chunks_mut(32).enumerate() {
        let mut u = hmac(&[salt, &(index as u32 + 1).to_be_bytes()]);
        let mut t = u;
        for _ in 1..iterations {
            u = hmac(&[&u]);
            t.iter_mut().zip(u).for_each(|(a, b)| *a ^= b);
        }
        chunk.copy_from_slice(&t[..chunk.len()]);
    }
}

fn hash_password(password: &str) -> Option<String> {
    let mut salt = [0u8; 16];
    getrandom::fill(&mut salt).ok()?;
    let mut out = [0u8; 32];
    pbkdf2(password.as_bytes(), &salt, ITERATIONS, &mut out);
    Some(format!("pbkdf2_sha256${ITERATIONS}${}${}", hex(&salt), hex(&out)))
}

fn verify_password(password: &str, stored: &str) -> bool {
    let parts: Vec<&str> = stored.split('$').collect();
    let (scheme, iterations, salt, expected) = match parts.as_slice() {
        [a, b, c, d] => (*a, *b, *c, *d),
        _ => return false,
    };
    let (Ok(iterations), Some(salt), Some(expected)) = (iterations.parse::<u32>(), unhex(salt), unhex(expected)) else {
        return false;
    };
    if scheme != "pbkdf2_sha256" || iterations == 0 || expected.is_empty() {
        return false;
    }
    let mut out = vec![0u8; expected.len()];
    pbkdf2(password.as_bytes(), &salt, iterations, &mut out);
    out.iter().zip(&expected).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
}

/// Para un usuario que no existe se hace el mismo trabajo, así el tiempo no delata qué cuentas hay.
const DUMMY_HASH: &str = "pbkdf2_sha256$600000$00000000000000000000000000000000$0000000000000000000000000000000000000000000000000000000000000000";

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    tokio::task::spawn_blocking(work).await.ok()
}

// --- Límites de intentos -------------------------------------------------------------------------

/// Ventanas deslizantes en memoria: se pierden al reiniciar, lo que solo da un respiro al que ataca.
#[derive(Default)]
pub struct Limits(Mutex<HashMap<String, Vec<i64>>>);

impl Limits {
    fn recent(&self, key: &str, window_ms: i64, now: i64) -> usize {
        let mut map = self.0.lock().unwrap();
        if map.len() > 10_000 {
            map.retain(|_, hits| hits.last().is_some_and(|&t| now - t < 60 * MINUTE));
        }
        let hits = map.entry(key.to_string()).or_default();
        hits.retain(|&t| now - t < window_ms);
        hits.len()
    }

    fn add(&self, key: &str, now: i64) {
        self.0.lock().unwrap().entry(key.to_string()).or_default().push(now);
    }

    /// Cuenta el intento y dice si ya pasó el máximo de la ventana.
    fn take(&self, key: &str, max: usize, window_ms: i64, now: i64) -> bool {
        let over = self.recent(key, window_ms, now) >= max;
        if !over {
            self.add(key, now);
        }
        over
    }

    fn clear(&self, key: &str) {
        self.0.lock().unwrap().remove(key);
    }
}

// --- Validación ----------------------------------------------------------------------------------

fn validate(name: &str, email: &str, password: &str) -> Result<(), &'static str> {
    let name_ok = (3..=20).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !name_ok {
        return Err("El usuario debe tener de 3 a 20 caracteres (letras, números, _ o -).");
    }
    let email_ok = email.len() <= 254
        && !email.chars().any(char::is_whitespace)
        && email.split_once('@').is_some_and(|(user, domain)| {
            !user.is_empty() && !domain.contains('@') && domain.split('.').count() >= 2 && domain.split('.').all(|p| !p.is_empty())
        });
    if !email_ok {
        return Err("El email no parece válido.");
    }
    if !(8..=128).contains(&password.chars().count()) {
        return Err("La contraseña debe tener entre 8 y 128 caracteres.");
    }
    Ok(())
}

// --- Sesiones ------------------------------------------------------------------------------------

fn random_token() -> Option<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).ok()?;
    Some(hex(&bytes))
}

fn token_hash(token: &str) -> String {
    hex(&Sha256::digest(token.as_bytes()))
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

/// `(id, nombre)` de la sesión de este pedido, si hay una vigente.
fn current(state: &AppState, headers: &HeaderMap) -> Option<(String, String)> {
    state.store.session(&token_hash(&cookie_token(headers)?), now_ms())
}

/// Las búsquedas piden sesión.
pub fn require(state: &AppState, headers: &HeaderMap) -> Result<(), Failure> {
    current(state, headers).map(|_| ()).ok_or(Failure(ApiError::Login))
}

fn reject(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

fn signed_in(state: &AppState, id: i64, name: &str, status: StatusCode) -> Response {
    let Some(token) = random_token() else {
        return reject(StatusCode::INTERNAL_SERVER_ERROR, "No se pudo iniciar la sesión.");
    };
    let max_age = SESSION_DAYS * 24 * 3600;
    if state.store.add_session(&token_hash(&token), &id.to_string(), name, now_ms() + max_age * 1000).is_err() {
        return reject(StatusCode::INTERNAL_SERVER_ERROR, "No se pudo iniciar la sesión.");
    }
    let mut response = (status, Json(json!({ "user": { "name": name } }))).into_response();
    response.headers_mut().insert(header::SET_COOKIE, session_cookie(&token, max_age));
    response
}

fn client_ip(headers: &HeaderMap) -> String {
    // Solo se llega por el túnel de Cloudflare, que pone la IP real del visitante.
    headers.get("cf-connecting-ip").and_then(|v| v.to_str().ok()).unwrap_or("local").to_string()
}

// --- Rutas ---------------------------------------------------------------------------------------

pub async fn me(State(state): State<Shared>, headers: HeaderMap) -> Json<serde_json::Value> {
    let user = current(&state, &headers).map(|(_, name)| json!({ "name": name }));
    Json(json!({ "enabled": true, "user": user }))
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

pub async fn login(State(state): State<Shared>, headers: HeaderMap, Json(form): Json<LoginForm>) -> Response {
    let now = now_ms();
    if state.limits.take(&format!("login:{}", client_ip(&headers)), 10, MINUTE, now) {
        return reject(StatusCode::TOO_MANY_REQUESTS, "Demasiados intentos. Probá de nuevo en un minuto.");
    }
    let login = form.login.trim().to_lowercase();
    if login.is_empty() || form.password.is_empty() || login.len() > 254 || form.password.len() > 1024 {
        return reject(StatusCode::UNAUTHORIZED, BAD_CREDENTIALS);
    }
    let failures = format!("fail:{login}");
    if state.limits.recent(&failures, 15 * MINUTE, now) >= 5 {
        return reject(StatusCode::TOO_MANY_REQUESTS, "Demasiados intentos fallidos para esta cuenta. Probá en 15 minutos.");
    }
    let user = state.store.find_user(&login);
    let stored = user.as_ref().map_or(DUMMY_HASH.to_string(), |u| u.password_hash.clone());
    let password = form.password;
    let ok = blocking(move || verify_password(&password, &stored)).await.unwrap_or(false);
    match user {
        Some(user) if ok => {
            state.limits.clear(&failures);
            signed_in(&state, user.id, &user.name, StatusCode::OK)
        }
        _ => {
            state.limits.add(&failures, now);
            reject(StatusCode::UNAUTHORIZED, BAD_CREDENTIALS)
        }
    }
}

pub async fn register(State(state): State<Shared>, headers: HeaderMap, Json(form): Json<RegisterForm>) -> Response {
    let (name, email) = (form.name.trim().to_string(), form.email.trim().to_lowercase());
    if let Err(message) = validate(&name, &email, &form.password) {
        return reject(StatusCode::BAD_REQUEST, message);
    }
    if state.limits.take(&format!("register:{}", client_ip(&headers)), 5, 60 * MINUTE, now_ms()) {
        return reject(StatusCode::TOO_MANY_REQUESTS, "Demasiadas cuentas nuevas desde tu conexión. Probá en un rato.");
    }
    let password = form.password;
    let Some(Some(hash)) = blocking(move || hash_password(&password)).await else {
        return reject(StatusCode::INTERNAL_SERVER_ERROR, "No se pudo crear la cuenta.");
    };
    match state.store.create_user(&name, &email, &hash, now_ms()) {
        Ok(user) => signed_in(&state, user.id, &user.name, StatusCode::CREATED),
        Err(Taken::Name) => reject(StatusCode::CONFLICT, "Ya existe una cuenta con ese usuario."),
        Err(Taken::Email) => reject(StatusCode::CONFLICT, "Ya existe una cuenta con ese email."),
    }
}

pub async fn logout(State(state): State<Shared>, headers: HeaderMap) -> Response {
    if let Some(token) = cookie_token(&headers) {
        state.store.remove_session(&token_hash(&token));
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(header::SET_COOKIE, session_cookie("", 0));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn derive(password: &str, salt: &str, iterations: u32, len: usize) -> String {
        let mut out = vec![0u8; len];
        pbkdf2(password.as_bytes(), salt.as_bytes(), iterations, &mut out);
        hex(&out)
    }

    #[test]
    fn pbkdf2_matches_published_vectors() {
        assert_eq!(derive("password", "salt", 1, 32), "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b");
        assert_eq!(derive("password", "salt", 2, 32), "ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43");
        assert_eq!(derive("password", "salt", 4096, 32), "c5e478d59288c841aa530db6845c4c8d962893a001ce4e11a4963873aa98134a");
        // RFC 7914 §11: dos bloques de salida.
        assert_eq!(
            derive("passwd", "salt", 1, 64),
            "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc49ca9cccf179b645991664b39d77ef317c71b845b1e30bd509112041d3a19783"
        );
    }

    #[test]
    fn hashes_and_verifies_passwords() {
        let stored = hash_password("una clave larga").unwrap();
        assert!(stored.starts_with("pbkdf2_sha256$600000$"));
        assert!(verify_password("una clave larga", &stored));
        assert!(!verify_password("otra clave", &stored));
        assert_ne!(hash_password("una clave larga").unwrap(), stored);
        assert!(!verify_password("x", "basura"));
        assert!(!verify_password("x", DUMMY_HASH));
    }

    #[test]
    fn validates_registrations() {
        assert!(validate("Jugador_1", "j@example.com", "clave1234").is_ok());
        assert!(validate("ab", "j@example.com", "clave1234").is_err());
        assert!(validate("con espacio", "j@example.com", "clave1234").is_err());
        assert!(validate("jugador", "sin-arroba", "clave1234").is_err());
        assert!(validate("jugador", "a@b", "clave1234").is_err());
        assert!(validate("jugador", "j@example.com", "corta").is_err());
    }

    #[test]
    fn limits_attempts_per_window() {
        let limits = Limits::default();
        for _ in 0..3 {
            assert!(!limits.take("k", 3, MINUTE, 0));
        }
        assert!(limits.take("k", 3, MINUTE, 10));
        assert!(!limits.take("k", 3, MINUTE, MINUTE + 1));
        limits.clear("k");
        assert_eq!(limits.recent("k", MINUTE, 0), 0);
    }

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
        assert_ne!(token_hash(&a), a);
    }
}
