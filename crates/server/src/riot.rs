//! Cliente de la API de Riot. La key solo vive en este proceso y nunca se registra.

use reqwest::{StatusCode, Url};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Plataforma → región para Match-V5. Account-V1 no tiene "sea" y usa "asia".
const PLATFORMS: &[(&str, &str)] = &[
    ("na1", "americas"),
    ("br1", "americas"),
    ("la1", "americas"),
    ("la2", "americas"),
    ("euw1", "europe"),
    ("eun1", "europe"),
    ("tr1", "europe"),
    ("ru", "europe"),
    ("me1", "europe"),
    ("kr", "asia"),
    ("jp1", "asia"),
    ("oc1", "sea"),
    ("sg2", "sea"),
    ("tw2", "sea"),
    ("vn2", "sea"),
];

pub fn region(platform: &str) -> Option<&'static str> {
    PLATFORMS.iter().find(|(p, _)| *p == platform).map(|(_, r)| *r)
}

pub fn account_region(platform: &str) -> Option<&'static str> {
    region(platform).map(|r| if r == "sea" { "asia" } else { r })
}

#[derive(Debug, PartialEq)]
pub enum ApiError {
    NotFound,
    Key,
    Limit,
    /// El servidor arrancó sin `RIOT_API_KEY`: la interfaz carga, pero no puede buscar.
    MissingKey,
    /// Hay cuentas configuradas y el pedido no trae una sesión vigente.
    Login,
    Other(String),
}

impl ApiError {
    pub fn status(&self) -> StatusCode {
        match self {
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::Key => StatusCode::BAD_GATEWAY,
            ApiError::Limit => StatusCode::TOO_MANY_REQUESTS,
            ApiError::MissingKey => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::Login => StatusCode::UNAUTHORIZED,
            ApiError::Other(_) => StatusCode::BAD_GATEWAY,
        }
    }

    pub fn message(&self) -> String {
        match self {
            ApiError::NotFound => "No se encontró ese jugador o esa partida.".into(),
            ApiError::Key => "La key de Riot no es válida o venció. Generá una nueva en el portal.".into(),
            ApiError::Limit => "Se alcanzó el límite de consultas a Riot. Probá de nuevo en un rato.".into(),
            ApiError::MissingKey => "El servidor todavía no tiene una key de Riot configurada.".into(),
            ApiError::Login => "Iniciá sesión para buscar jugadores.".into(),
            ApiError::Other(message) => message.clone(),
        }
    }
}

pub fn url(host: &str, segments: &[&str], query: &[(&str, &str)]) -> Url {
    let mut url = Url::parse(&format!("https://{host}.api.riotgames.com")).expect("host válido");
    url.path_segments_mut().expect("URL con ruta").extend(segments);
    if !query.is_empty() {
        url.query_pairs_mut().extend_pairs(query);
    }
    url
}

pub struct Riot {
    http: reqwest::Client,
    key: Option<String>,
    // ponytail: límite fijo de concurrencia. Reemplazar por una cola por región y método (W4).
    slots: Semaphore,
}

impl Riot {
    pub fn new(key: Option<String>) -> Self {
        Self { http: reqwest::Client::new(), key, slots: Semaphore::new(5) }
    }

    pub async fn get(&self, host: &str, segments: &[&str], query: &[(&str, &str)]) -> Result<Value, ApiError> {
        let key = self.key.as_deref().ok_or(ApiError::MissingKey)?;
        let url = url(host, segments, query);
        let _slot = self.slots.acquire().await.map_err(|_| ApiError::Other("Servidor cerrándose.".into()))?;
        for _ in 0..3 {
            let response = self
                .http
                .get(url.clone())
                .header("X-Riot-Token", key)
                .send()
                .await
                .map_err(|_| ApiError::Other("No se pudo conectar con Riot.".into()))?;
            match response.status() {
                status if status.is_success() => {
                    return response.json().await.map_err(|_| ApiError::Other("Respuesta inválida de Riot.".into()));
                }
                StatusCode::NOT_FOUND => return Err(ApiError::NotFound),
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => return Err(ApiError::Key),
                StatusCode::TOO_MANY_REQUESTS => {
                    let wait = response
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(2);
                    if wait > 10 {
                        return Err(ApiError::Limit);
                    }
                    tokio::time::sleep(Duration::from_secs(wait)).await;
                }
                status => return Err(ApiError::Other(format!("Riot respondió {}.", status.as_u16()))),
            }
        }
        Err(ApiError::Limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_platforms_to_regions() {
        assert_eq!(region("la2"), Some("americas"));
        assert_eq!(region("euw1"), Some("europe"));
        assert_eq!(region("sg2"), Some("sea"));
        assert_eq!(account_region("sg2"), Some("asia"));
        assert_eq!(region("../x"), None);
    }

    #[test]
    fn encodes_riot_ids_in_the_path() {
        let built = url("americas", &["riot", "account", "v1", "accounts", "by-riot-id", "9z Larian Mid", "N/A?"], &[]);
        assert_eq!(
            built.as_str(),
            "https://americas.api.riotgames.com/riot/account/v1/accounts/by-riot-id/9z%20Larian%20Mid/N%2FA%3F"
        );
        let query = url("la2", &["lol", "x"], &[("start", "0"), ("count", "20")]);
        assert_eq!(query.query(), Some("start=0&count=20"));
    }
}
