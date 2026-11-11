//! Servidor de la web: busca jugadores con la API de Riot y sirve la interfaz.

mod auth;
mod riot;
mod store;

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::future::join_all;
use lolup_core::game::{self, Detail, Mastery, MatchRow, Names, Rank, Summary};
use lolup_core::{lp, match_v5};
use riot::{ApiError, Riot};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use store::Store;
use tokio::sync::OnceCell;
use tower_http::services::{ServeDir, ServeFile};

const DDRAGON: &str = "https://ddragon.leagueoflegends.com";
const CUSTOM_QUEUE: i64 = 0;
const HISTORY_SIZE: &str = "20";

struct AppState {
    riot: Riot,
    http: reqwest::Client,
    catalog: OnceCell<Arc<WebCatalog>>,
    store: Store,
    limits: auth::Limits,
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

type Shared = Arc<AppState>;

struct Failure(ApiError);

impl From<ApiError> for Failure {
    fn from(error: ApiError) -> Self {
        Failure(error)
    }
}

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        (self.0.status(), Json(json!({ "error": self.0.message() }))).into_response()
    }
}

fn bad(message: &str) -> Failure {
    Failure(ApiError::Other(message.into()))
}

#[derive(Serialize)]
struct Entry {
    name: String,
    icon: String,
}

#[derive(Serialize)]
struct CatalogView {
    champions: HashMap<i64, Entry>,
    items: HashMap<i64, Entry>,
    spells: HashMap<i64, Entry>,
    runes: HashMap<i64, Entry>,
    queues: HashMap<i64, Entry>,
}

struct WebCatalog {
    version: String,
    names: Names,
    view: CatalogView,
}

/// Nombres cortos para las colas habituales; el resto usa la descripción de Riot.
const QUEUE_NAMES: &[(i64, &str)] = &[
    (400, "Normal (reclutamiento)"),
    (420, "Ranked Solo/Duo"),
    (430, "Normal (a ciegas)"),
    (440, "Ranked Flex"),
    (450, "ARAM"),
    (490, "Partida rápida"),
    (1700, "Arena"),
    (1900, "URF"),
];

fn build_catalog(version: &str, champions: &Value, items: &Value, spells: &Value, runes: &Value, queues: &Value) -> WebCatalog {
    let cdn = format!("{DDRAGON}/cdn/{version}/img");
    let keyed = |data: &Value, folder: &str, key_field: Option<&str>| -> HashMap<i64, Entry> {
        data["data"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(key, e)| {
                let id = match key_field {
                    Some(field) => e[field].as_str()?.parse().ok()?,
                    None => key.parse().ok()?,
                };
                let icon = format!("{cdn}/{folder}/{}", e["image"]["full"].as_str()?);
                Some((id, Entry { name: e["name"].as_str()?.to_string(), icon }))
            })
            .collect()
    };
    let champion_view = keyed(champions, "champion", Some("key"));
    let names = champion_view.iter().map(|(id, e)| (*id, e.name.clone())).collect();

    let mut rune_view = HashMap::new();
    for style in runes.as_array().into_iter().flatten() {
        let mut add = |e: &Value| {
            if let (Some(id), Some(name), Some(icon)) = (e["id"].as_i64(), e["name"].as_str(), e["icon"].as_str()) {
                rune_view.insert(id, Entry { name: name.into(), icon: format!("{DDRAGON}/cdn/img/{icon}") });
            }
        };
        add(style);
        for rune in style["slots"].as_array().into_iter().flatten().flat_map(|s| s["runes"].as_array().into_iter().flatten()) {
            add(rune);
        }
    }

    let queue_view = queues
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|q| {
            let id = q["queueId"].as_i64()?;
            let short = QUEUE_NAMES.iter().find(|(qid, _)| *qid == id).map(|(_, n)| n.to_string());
            let name = short.or_else(|| q["description"].as_str().map(|d| d.trim_end_matches(" games").to_string()))?;
            Some((id, Entry { name, icon: String::new() }))
        })
        .collect();

    WebCatalog {
        version: version.to_string(),
        names,
        view: CatalogView {
            champions: champion_view,
            items: keyed(items, "item", None),
            spells: keyed(spells, "spell", Some("key")),
            runes: rune_view,
            queues: queue_view,
        },
    }
}

async fn fetch_json(http: &reqwest::Client, url: &str) -> Result<Value, ApiError> {
    let response = http.get(url).send().await.map_err(|_| ApiError::Other("No se pudo cargar Data Dragon.".into()))?;
    response.json().await.map_err(|_| ApiError::Other("Data Dragon respondió algo inválido.".into()))
}

async fn catalog(state: &AppState) -> Result<Arc<WebCatalog>, ApiError> {
    state
        .catalog
        .get_or_try_init(|| async {
            let versions = fetch_json(&state.http, &format!("{DDRAGON}/api/versions.json")).await?;
            let version = versions[0].as_str().ok_or(ApiError::Other("Sin versión de Data Dragon.".into()))?.to_string();
            let data = |file: &str| format!("{DDRAGON}/cdn/{version}/data/es_AR/{file}.json");
            let urls = [data("champion"), data("item"), data("summoner"), data("runesReforged")];
            let (champions, items, spells, runes, queues) = futures_util::try_join!(
                fetch_json(&state.http, &urls[0]),
                fetch_json(&state.http, &urls[1]),
                fetch_json(&state.http, &urls[2]),
                fetch_json(&state.http, &urls[3]),
                fetch_json(&state.http, "https://static.developer.riotgames.com/docs/lol/queues.json"),
            )?;
            Ok(Arc::new(build_catalog(&version, &champions, &items, &spells, &runes, &queues)))
        })
        .await
        .cloned()
}

async fn get_catalog(State(state): State<Shared>) -> Result<Json<Value>, Failure> {
    let catalog = catalog(&state).await?;
    serde_json::to_value(&catalog.view).map(Json).map_err(|_| bad("No se pudo armar el catálogo."))
}

/// Las partidas no cambian: se piden una sola vez por proceso.
async fn cached(state: &AppState, key: String, host: &str, segments: Vec<&str>) -> Result<Arc<Value>, ApiError> {
    if let Some(hit) = state.store.cached(&key).and_then(|body| serde_json::from_str(&body).ok()) {
        return Ok(Arc::new(hit));
    }
    let value = state.riot.get(host, &segments, &[]).await?;
    state.store.put(&key, &value.to_string(), now_ms());
    Ok(Arc::new(value))
}

#[derive(Deserialize)]
struct ProfileQuery {
    platform: String,
    name: String,
    tag: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileView {
    name: String,
    tag: String,
    level: i64,
    icon_id: i64,
    icon_url: String,
    solo: Option<Rank>,
    flex: Option<Rank>,
    mastery: Vec<Mastery>,
    lp_history: Vec<lp::Point>,
}

#[derive(Serialize)]
struct MatchesView {
    rows: Vec<MatchRow>,
    summaries: Vec<Summary>,
}

#[derive(Serialize)]
struct ProfileResponse {
    puuid: String,
    platform: String,
    profile: ProfileView,
    matches: MatchesView,
}

fn checked_platform(platform: &str) -> Result<(&'static str, &'static str), Failure> {
    let region = riot::region(platform).ok_or_else(|| bad("Región desconocida."))?;
    let account = riot::account_region(platform).ok_or_else(|| bad("Región desconocida."))?;
    Ok((region, account))
}

fn checked_riot_id(name: &str, tag: &str) -> Result<(), Failure> {
    let name_ok = (3..=16).contains(&name.chars().count());
    let tag_ok = (2..=5).contains(&tag.chars().count()) && tag.chars().all(char::is_alphanumeric);
    if name_ok && tag_ok { Ok(()) } else { Err(bad("El Riot ID tiene que tener la forma Nombre#TAG.")) }
}

async fn profile(
    State(state): State<Shared>,
    headers: HeaderMap,
    Query(q): Query<ProfileQuery>,
) -> Result<Json<ProfileResponse>, Failure> {
    auth::require(&state, &headers)?;
    let platform = q.platform.to_lowercase();
    let (region, account_host) = checked_platform(&platform)?;
    let (name, tag) = (q.name.trim(), q.tag.trim().trim_start_matches('#'));
    checked_riot_id(name, tag)?;

    let catalog = catalog(&state).await?;
    let account = state.riot.get(account_host, &["riot", "account", "v1", "accounts", "by-riot-id", name, tag], &[]).await?;
    let puuid = account["puuid"].as_str().ok_or(ApiError::NotFound)?.to_string();

    let p = puuid.as_str();
    let summoner_path = ["lol", "summoner", "v4", "summoners", "by-puuid", p];
    let league_path = ["lol", "league", "v4", "entries", "by-puuid", p];
    let mastery_path = ["lol", "champion-mastery", "v4", "champion-masteries", "by-puuid", p, "top"];
    let ids_path = ["lol", "match", "v5", "matches", "by-puuid", p, "ids"];
    let (summoner, entries, mastery, ids) = futures_util::try_join!(
        state.riot.get(&platform, &summoner_path, &[]),
        state.riot.get(&platform, &league_path, &[]),
        state.riot.get(&platform, &mastery_path, &[("count", "6")]),
        state.riot.get(region, &ids_path, &[("start", "0"), ("count", HISTORY_SIZE)]),
    )?;

    let match_ids: Vec<String> = ids.as_array().into_iter().flatten().filter_map(|id| id.as_str().map(str::to_string)).collect();
    let fetched = join_all(
        match_ids.iter().map(|id| cached(&state, format!("match:{id}"), region, vec!["lol", "match", "v5", "matches", id])),
    )
    .await;
    // Las personalizadas solo se muestran con el permiso del jugador, según la política de Riot.
    let games: Vec<game::Game> = fetched
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|raw| match_v5::game(&raw, &catalog.names).ok())
        .filter(|g| g.queue_id != CUSTOM_QUEUE)
        .collect();
    let rows: Vec<MatchRow> = games.iter().filter_map(|g| game::row(g, &puuid)).collect();
    let summaries = game::summaries(&rows);
    let solo = match_v5::rank(&entries, "RANKED_SOLO_5x5");
    let flex = match_v5::rank(&entries, "RANKED_FLEX_SR");
    // ponytail: el historial se guarda por PUUID. Revisar al pasar a la key de producción por si Riot
    // entrega otro identificador para la misma cuenta.
    let mut history = state.store.lp_history(&puuid);
    let new = lp::changes(&history, now_ms(), &[(lp::SOLO, solo.as_ref()), (lp::FLEX, flex.as_ref())]);
    if state.store.add_lp(&puuid, &new).is_ok() {
        history.extend(new);
    }
    let icon_id = summoner["profileIconId"].as_i64().unwrap_or(0);

    Ok(Json(ProfileResponse {
        profile: ProfileView {
            name: account["gameName"].as_str().unwrap_or(name).to_string(),
            tag: account["tagLine"].as_str().unwrap_or(tag).to_string(),
            level: summoner["summonerLevel"].as_i64().unwrap_or(0),
            icon_id,
            icon_url: format!("{DDRAGON}/cdn/{}/img/profileicon/{icon_id}.png", catalog.version),
            solo,
            flex,
            mastery: game::top_mastery(&mastery, &catalog.names, 6),
            lp_history: lp::points(&history, lp::SOLO),
        },
        matches: MatchesView { rows, summaries },
        puuid,
        platform,
    }))
}

#[derive(Deserialize)]
struct MatchQuery {
    platform: String,
    id: i64,
    puuid: String,
}

async fn match_detail(State(state): State<Shared>, headers: HeaderMap, Query(q): Query<MatchQuery>) -> Result<Json<Detail>, Failure> {
    auth::require(&state, &headers)?;
    let platform = q.platform.to_lowercase();
    let (region, _) = checked_platform(&platform)?;
    let match_id = format!("{}_{}", platform.to_uppercase(), q.id);
    let catalog = catalog(&state).await?;
    let raw = cached(&state, format!("match:{match_id}"), region, vec!["lol", "match", "v5", "matches", &match_id]).await?;
    let parsed = match_v5::game(&raw, &catalog.names).map_err(|e| bad(&e))?;
    if parsed.queue_id == CUSTOM_QUEUE {
        return Err(Failure(ApiError::NotFound));
    }
    // Sin línea de tiempo la partida se muestra igual, solo falta el gráfico.
    let timeline = cached(&state, format!("timeline:{match_id}"), region, vec!["lol", "match", "v5", "matches", &match_id, "timeline"]).await.ok();
    let detail = game::detail(&parsed, timeline.as_deref().map(match_v5::frames), &q.puuid).map_err(|e| bad(&e))?;
    Ok(Json(detail))
}

/// La key se lee del entorno o de un archivo `.env` que no se sube al repo.
fn riot_key() -> Option<String> {
    std::env::var("RIOT_API_KEY").ok().filter(|k| !k.is_empty()).or_else(|| {
        std::fs::read_to_string(".env").ok()?.lines().find_map(|line| {
            let value = line.trim().strip_prefix("RIOT_API_KEY=")?.trim().trim_matches('"');
            (!value.is_empty()).then(|| value.to_string())
        })
    })
}

#[tokio::main]
async fn main() {
    let key = riot_key();
    if key.is_none() {
        // Arranca igual: la interfaz carga y cada búsqueda explica qué falta, en vez de reiniciarse en bucle.
        eprintln!("Falta RIOT_API_KEY en el entorno o en el archivo .env: las búsquedas van a fallar hasta cargarla.");
    }
    let db_path = std::env::var("LOLUP_DB").unwrap_or_else(|_| "data/lolup.db".into());
    let store = Store::open(std::path::Path::new(&db_path)).unwrap_or_else(|error| {
        eprintln!("No se pudo abrir la base {db_path}: {error}");
        std::process::exit(1);
    });
    let limits = auth::Limits::default();
    let state = Arc::new(AppState { riot: Riot::new(key), http: reqwest::Client::new(), catalog: OnceCell::new(), store, limits });
    let ui = ServeDir::new("dist").fallback(ServeFile::new("dist/index.html"));
    let app = Router::new()
        .route("/api/catalog", get(get_catalog))
        .route("/api/profile", get(profile))
        .route("/api/match", get(match_detail))
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/logout", post(auth::logout))
        .fallback_service(ui)
        .with_state(state);
    // Solo local mientras se use la key personal: Riot no permite exponerla al público.
    // En el contenedor se bindea a 0.0.0.0 sin publicar puertos: solo entra el túnel, detrás de Access.
    let addr = std::env::var("LOLUP_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap_or_else(|error| {
        eprintln!("No se pudo escuchar en {addr}: {error}");
        std::process::exit(1);
    });
    println!("LolUp web en http://{addr}");
    axum::serve(listener, app).await.expect("servidor");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_riot_ids() {
        assert!(checked_riot_id("9z Larian Mid", "NACHY").is_ok());
        assert!(checked_riot_id("ab", "LAS").is_err());
        assert!(checked_riot_id("Jugador", "L").is_err());
        assert!(checked_riot_id("Jugador", "LA/S").is_err());
        assert!(checked_platform("la2").is_ok());
        assert!(checked_platform("xx").is_err());
    }

    #[test]
    fn builds_catalog_from_data_dragon() {
        let champions = json!({ "data": { "Ahri": { "key": "103", "name": "Ahri", "image": { "full": "Ahri.png" } } } });
        let items = json!({ "data": { "3020": { "name": "Botas", "image": { "full": "3020.png" } } } });
        let spells = json!({ "data": { "SummonerFlash": { "key": "4", "name": "Destello", "image": { "full": "SummonerFlash.png" } } } });
        let runes = json!([{ "id": 8100, "name": "Dominación", "icon": "perk-images/Styles/7200_Domination.png",
            "slots": [{ "runes": [{ "id": 8112, "name": "Electrocutar", "icon": "perk-images/Styles/Domination/Electrocute/Electrocute.png" }] }] }]);
        let queues = json!([{ "queueId": 420, "description": "5v5 Ranked Solo games" }, { "queueId": 900, "description": "ARURF games" }]);
        let c = build_catalog("16.22.1", &champions, &items, &spells, &runes, &queues);
        assert_eq!(c.names.get(&103).map(String::as_str), Some("Ahri"));
        assert_eq!(c.view.champions[&103].icon, "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Ahri.png");
        assert_eq!(c.view.items[&3020].icon, "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/item/3020.png");
        assert_eq!(c.view.spells[&4].name, "Destello");
        assert_eq!(c.view.runes[&8112].icon, "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/Domination/Electrocute/Electrocute.png");
        assert!(c.view.runes.contains_key(&8100));
        assert_eq!(c.view.queues[&420].name, "Ranked Solo/Duo");
        assert_eq!(c.view.queues[&900].name, "ARURF");
    }
}
