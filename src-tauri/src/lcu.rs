//! Acceso de solo lectura a la API local del cliente de League.

use serde_json::{Value, json};
use std::path::PathBuf;

/// Rutas exactas o prefijos permitidos. Agregar uno exige revisar docs/BASE.md §3.
const ALLOWED_EXACT: &[&str] = &[
    "/lol-summoner/v1/current-summoner",
    "/lol-gameflow/v1/gameflow-phase",
    "/lol-champ-select/v1/session",
    "/lol-ranked/v1/current-ranked-stats",
    "/lol-champion-mastery/v1/local-player/champion-mastery",
];
const ASSETS: &str = "/lol-game-data/assets/";
const ALLOWED_PREFIXES: &[&str] = &[
    ASSETS,
    "/lol-match-history/v1/products/lol/",
    "/lol-match-history/v1/games/",
    "/lol-match-history/v1/game-timelines/",
];

pub fn is_allowed(path: &str) -> bool {
    !path.contains("..")
        && (ALLOWED_EXACT.contains(&path)
            || ALLOWED_PREFIXES.iter().any(|p| path.starts_with(p)))
}

// Sin Debug: el token no debe poder aparecer en un log.
struct Auth {
    port: u16,
    token: String,
}

/// Formato: `LeagueClient:<pid>:<port>:<token>:<protocol>`.
fn parse_lockfile(content: &str) -> Option<Auth> {
    let parts: Vec<&str> = content.trim().split(':').collect();
    if parts.len() < 5 || parts[3].is_empty() {
        return None;
    }
    Some(Auth { port: parts[2].parse().ok()?, token: parts[3].to_string() })
}

/// Recorre RiotClientInstalls.json y junta claves o valores que apunten a League.
fn league_dirs(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) if s.contains("League of Legends") => out.push(s.clone()),
        Value::Array(items) => items.iter().for_each(|v| league_dirs(v, out)),
        Value::Object(map) => {
            for (key, v) in map {
                if key.contains("League of Legends") {
                    out.push(key.clone());
                }
                league_dirs(v, out);
            }
        }
        _ => {}
    }
}

fn lockfile_candidates() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(data) = std::env::var_os("PROGRAMDATA") {
        let installs = PathBuf::from(data).join("Riot Games/RiotClientInstalls.json");
        if let Some(json) = std::fs::read_to_string(installs)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        {
            league_dirs(&json, &mut dirs);
        }
    }
    dirs.push("C:/Riot Games/League of Legends".into());
    dirs.push("/Applications/League of Legends.app/Contents/LoL".into());
    dirs.into_iter().map(|d| PathBuf::from(d).join("lockfile")).collect()
}

fn discover() -> Option<Auth> {
    lockfile_candidates()
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok().and_then(|s| parse_lockfile(&s)))
}

#[derive(Clone)]
pub struct Lcu {
    http: reqwest::Client,
    pub demo: bool,
}

impl Lcu {
    pub fn new(demo: bool) -> Self {
        // ponytail: acepta cualquier certificado, solo se usa contra 127.0.0.1.
        // Fijar el certificado raíz de Riot antes de la beta pública.
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .no_proxy()
            .build()
            .expect("cliente HTTP local");
        Self { http, demo }
    }

    pub async fn get(&self, path: &str) -> Result<Value, String> {
        if !is_allowed(path) {
            return Err(format!("Endpoint no permitido: {path}"));
        }
        // En demo, los catálogos reales sirven para ver íconos si hay un cliente abierto.
        if self.demo && !(path.starts_with(ASSETS) && discover().is_some()) {
            return demo_response(path);
        }
        let response = self.send(path).await?;
        response.json().await.map_err(|_| "Respuesta inválida del cliente.".to_string())
    }

    pub async fn get_asset_list(&self, name: &str) -> Result<Value, String> {
        self.get(&format!("{ASSETS}v1/{name}.json")).await
    }

    /// Imagen del cliente, con su tipo. Solo sirve rutas de recursos estáticos.
    pub async fn asset(&self, path: &str) -> Result<(String, Vec<u8>), String> {
        if !path.starts_with(ASSETS) || !is_allowed(path) {
            return Err("Recurso no disponible.".into());
        }
        let response = self.send(path).await?;
        let kind = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        let bytes = response.bytes().await.map_err(|_| "Recurso incompleto.")?;
        Ok((kind, bytes.to_vec()))
    }

    async fn send(&self, path: &str) -> Result<reqwest::Response, String> {
        let auth = discover().ok_or("League no está abierto.")?;
        let response = self
            .http
            .get(format!("https://127.0.0.1:{}{path}", auth.port))
            .basic_auth("riot", Some(auth.token))
            .send()
            .await
            .map_err(|_| "No se pudo conectar con el cliente de League.")?;
        if !response.status().is_success() {
            return Err(format!("El cliente respondió {}.", response.status().as_u16()));
        }
        Ok(response)
    }
}

fn demo_response(path: &str) -> Result<Value, String> {
    let text = match path {
        "/lol-summoner/v1/current-summoner" => {
            return Ok(json!({
                "puuid": "demo-me", "gameName": "EjemploUno", "tagLine": "LAS",
                "summonerLevel": 312, "profileIconId": 1
            }));
        }
        "/lol-gameflow/v1/gameflow-phase" => return Ok(json!("ChampSelect")),
        "/lol-champ-select/v1/session" => include_str!("../../crates/core/fixtures/session.json"),
        "/lol-ranked/v1/current-ranked-stats" => include_str!("../../crates/core/fixtures/ranked.json"),
        "/lol-champion-mastery/v1/local-player/champion-mastery" => include_str!("../../crates/core/fixtures/mastery.json"),
        "/lol-game-data/assets/v1/champion-summary.json" => include_str!("../../crates/core/fixtures/champions.json"),
        "/lol-game-data/assets/v1/perkstyles.json" => return Ok(json!({ "styles": [] })),
        p if p.starts_with(ASSETS) => return Ok(json!([])),
        p if p.starts_with("/lol-match-history/v1/game-timelines/") => include_str!("../../crates/core/fixtures/timeline.json"),
        p if p.starts_with("/lol-match-history/v1/products/") => include_str!("../../crates/core/fixtures/history.json"),
        p if p.starts_with("/lol-match-history/v1/games/") => include_str!("../../crates/core/fixtures/game.json"),
        _ => return Err("Sin datos de ejemplo.".into()),
    };
    serde_json::from_str(text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_only_the_read_endpoints_we_need() {
        assert!(is_allowed("/lol-gameflow/v1/gameflow-phase"));
        assert!(is_allowed("/lol-match-history/v1/games/42"));
        assert!(is_allowed("/lol-ranked/v1/current-ranked-stats"));
        assert!(is_allowed("/lol-game-data/assets/v1/items.json"));
        for denied in [
            "/lol-game-data/assets/../../riotclient/auth-token",
            "/lol-summoner/v1/summoners/1",
            "/lol-matchmaking/v1/ready-check/accept",
            "/lol-chat/v1/conversations",
            "/lol-champ-select/v1/session/actions/1",
            "/lol-match-history/v1/games/../../lol-chat/v1/me",
            "/riotclient/auth-token",
        ] {
            assert!(!is_allowed(denied), "{denied}");
        }
    }

    #[test]
    fn parses_lockfile() {
        let auth = parse_lockfile("LeagueClient:123:54321:secret:https\n").unwrap();
        assert_eq!((auth.port, auth.token.as_str()), (54321, "secret"));
        assert!(parse_lockfile("LeagueClient:123:nope:secret:https").is_none());
        assert!(parse_lockfile("LeagueClient:123:54321::https").is_none());
        assert!(parse_lockfile("").is_none());
    }

    #[test]
    fn finds_install_dirs_in_keys_and_values() {
        let installs = json!({
            "associated_client": { "D:/Games/League of Legends/": "D:/Riot/Client.exe" },
            "patchlines": ["E:/League of Legends"],
            "other": "C:/Valorant"
        });
        let mut dirs = Vec::new();
        league_dirs(&installs, &mut dirs);
        dirs.sort();
        assert_eq!(dirs, ["D:/Games/League of Legends/", "E:/League of Legends"]);
    }

    #[test]
    fn fixtures_are_valid() {
        for path in [
            "/lol-champ-select/v1/session",
            "/lol-game-data/assets/v1/champion-summary.json",
            "/lol-match-history/v1/products/lol/x/matches",
            "/lol-match-history/v1/games/1",
            "/lol-match-history/v1/game-timelines/1",
            "/lol-ranked/v1/current-ranked-stats",
            "/lol-champion-mastery/v1/local-player/champion-mastery",
        ] {
            demo_response(path).unwrap();
        }
    }
}
