#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod facts;
mod lcu;
mod lp;

use lcu::Lcu;
use lolup_core::game;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State, http, ipc::Channel};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const DEMO_REVIEW: &str = "Perdiste porque moriste más de lo que tu equipo pudo sostener.\n\n\
Clave 1: 8 muertes en 30,2 minutos. Apuntá a 5 o menos.\n\
Clave 2: 6,0 de farmeo por minuto. Sumá una oleada más antes de cada rotación.\n\
Clave 3: tu visión fue 19. Colocá un ward de control en cada vuelta a base.\n\n\
Próxima partida: terminar con 5 muertes o menos.\n\n\
(Texto de ejemplo: modo demo sin API key.)";

const DEMO_PLAN: &str = "Objetivo: llegar a nivel 6 con la oleada empujada.\n\
Fuerte en: castigar al rival cuando use su movilidad.\n\
Cuidado con: el jungla rival después de empujar sin visión.\n\
Recordá: mirar el minimapa cada vez que termines una oleada.";

struct Core {
    lcu: Lcu,
    ai: reqwest::Client,
    plan: Mutex<String>,
    lp_file: Mutex<()>,
}

#[derive(Serialize)]
struct Status {
    connected: bool,
    phase: String,
    demo: bool,
    message: Option<String>,
}

async fn my_puuid(lcu: &Lcu) -> Result<String, String> {
    let me = lcu.get("/lol-summoner/v1/current-summoner").await?;
    me["puuid"].as_str().map(str::to_string).ok_or("Respuesta sin jugador.".to_string())
}

async fn names(lcu: &Lcu) -> Result<facts::Names, String> {
    Ok(facts::champion_names(&lcu.get("/lol-game-data/assets/v1/champion-summary.json").await?))
}

async fn phase(lcu: &Lcu) -> Result<String, String> {
    let value = lcu.get("/lol-gameflow/v1/gameflow-phase").await?;
    Ok(value.as_str().unwrap_or("None").to_string())
}

#[tauri::command]
async fn status(core: State<'_, Core>) -> Result<Status, String> {
    let demo = core.lcu.demo;
    Ok(match phase(&core.lcu).await {
        Ok(phase) => Status { connected: true, phase, demo, message: None },
        Err(message) => Status { connected: false, phase: "None".into(), demo, message: Some(message) },
    })
}

#[derive(Serialize)]
struct Entry {
    name: String,
    icon: String,
}

type EntryMap = HashMap<i64, Entry>;

fn entries(list: &Value, icon_key: &str) -> EntryMap {
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let name = e["name"].as_str()?.to_string();
            let icon = e[icon_key].as_str().unwrap_or("").to_string();
            Some((e["id"].as_i64()?, Entry { name, icon }))
        })
        .collect()
}

#[derive(Serialize)]
struct Catalog {
    champions: EntryMap,
    items: EntryMap,
    spells: EntryMap,
    runes: EntryMap,
    queues: EntryMap,
}

#[tauri::command]
async fn catalog(core: State<'_, Core>) -> Result<Catalog, String> {
    let asset = |name: &'static str| core.lcu.get_asset_list(name);
    let (champions, items, spells, perks, styles, queues) = futures_util::try_join!(
        asset("champion-summary"),
        asset("items"),
        asset("summoner-spells"),
        asset("perks"),
        asset("perkstyles"),
        asset("queues"),
    )?;
    let mut runes = entries(&perks, "iconPath");
    runes.extend(entries(&styles["styles"], "iconPath"));
    Ok(Catalog {
        champions: entries(&champions, "squarePortraitPath"),
        items: entries(&items, "iconPath"),
        spells: entries(&spells, "iconPath"),
        runes,
        queues: entries(&queues, "iconPath"),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Profile {
    name: String,
    tag: String,
    level: i64,
    icon_id: i64,
    solo: Option<game::Rank>,
    flex: Option<game::Rank>,
    mastery: Vec<game::Mastery>,
    lp_history: Vec<lp::Point>,
}

fn lp_snapshots(app: &AppHandle, core: &Core, me: &Value, solo: Option<&game::Rank>, flex: Option<&game::Rank>) -> Vec<lp::Snapshot> {
    if core.lcu.demo {
        return serde_json::from_str(include_str!("../../crates/core/fixtures/lp.json")).unwrap_or_default();
    }
    let puuid: String = me["puuid"].as_str().unwrap_or("").chars().filter(|c| c.is_alphanumeric() || *c == '-').collect();
    let Ok(dir) = app.path().app_data_dir() else {
        return Vec::new();
    };
    let path = dir.join(format!("lp-{puuid}.jsonl"));
    let ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
    let _guard = core.lp_file.lock().unwrap();
    lp::record(&path, ms, &[(lp::SOLO, solo), (lp::FLEX, flex)])
}

#[tauri::command]
async fn profile(app: AppHandle, core: State<'_, Core>) -> Result<Profile, String> {
    let me = core.lcu.get("/lol-summoner/v1/current-summoner").await?;
    let ranked = core.lcu.get("/lol-ranked/v1/current-ranked-stats").await?;
    let mastery = core.lcu.get("/lol-champion-mastery/v1/local-player/champion-mastery").await?;
    let names = names(&core.lcu).await?;
    let solo = lolup_core::lcu::rank(&ranked, "RANKED_SOLO_5x5");
    let flex = lolup_core::lcu::rank(&ranked, "RANKED_FLEX_SR");
    let snapshots = lp_snapshots(&app, &core, &me, solo.as_ref(), flex.as_ref());
    Ok(Profile {
        name: me["gameName"].as_str().unwrap_or("Invocador").to_string(),
        tag: me["tagLine"].as_str().unwrap_or("").to_string(),
        level: me["summonerLevel"].as_i64().unwrap_or(0),
        icon_id: me["profileIconId"].as_i64().unwrap_or(0),
        solo,
        flex,
        mastery: game::top_mastery(&mastery, &names, 6),
        lp_history: lp::points(&snapshots, lp::SOLO),
    })
}

#[derive(Serialize)]
struct Matches {
    rows: Vec<game::MatchRow>,
    summaries: Vec<game::Summary>,
}

#[tauri::command]
async fn matches(core: State<'_, Core>) -> Result<Matches, String> {
    let puuid = my_puuid(&core.lcu).await?;
    let path = format!("/lol-match-history/v1/products/lol/{puuid}/matches?begIndex=0&endIndex=19");
    let list = core.lcu.get(&path).await?;
    let games = lolup_core::lcu::history(&list, &names(&core.lcu).await?);
    let rows: Vec<game::MatchRow> = games.iter().filter_map(|g| game::row(g, &puuid)).collect();
    let summaries = game::summaries(&rows);
    Ok(Matches { rows, summaries })
}

#[tauri::command]
async fn match_detail(core: State<'_, Core>, game_id: i64) -> Result<game::Detail, String> {
    let puuid = my_puuid(&core.lcu).await?;
    let raw = core.lcu.get(&format!("/lol-match-history/v1/games/{game_id}")).await?;
    // Sin línea de tiempo la partida se muestra igual, solo falta el gráfico.
    let timeline = core.lcu.get(&format!("/lol-match-history/v1/game-timelines/{game_id}")).await.ok();
    let parsed = lolup_core::lcu::game(&raw, &names(&core.lcu).await?)?;
    game::detail(&parsed, timeline.as_ref().map(lolup_core::lcu::frames), &puuid)
}

fn use_demo_ai(core: &Core) -> bool {
    core.lcu.demo && std::env::var_os("ANTHROPIC_API_KEY").is_none()
}

#[tauri::command]
async fn review(core: State<'_, Core>, game_id: i64, on_text: Channel<String>) -> Result<(), String> {
    let send = |text: &str| {
        let _ = on_text.send(text.to_string());
    };
    if use_demo_ai(&core) {
        return ai::demo_stream(DEMO_REVIEW, send).await.map(|_| ());
    }
    let puuid = my_puuid(&core.lcu).await?;
    let game = core.lcu.get(&format!("/lol-match-history/v1/games/{game_id}")).await?;
    let card = facts::fact_card(&facts::game_facts(&game, &puuid, &names(&core.lcu).await?)?);
    ai::stream(&core.ai, ai::REVIEW_PROMPT, &card, send).await.map(|_| ())
}

#[tauri::command]
async fn generate_plan(app: AppHandle, core: State<'_, Core>, on_text: Channel<String>) -> Result<(), String> {
    if !facts::plan_allowed(&phase(&core.lcu).await?) {
        return Err("El plan solo se genera durante la selección de campeón.".into());
    }
    let send = |text: &str| {
        let _ = on_text.send(text.to_string());
    };
    let plan = if use_demo_ai(&core) {
        ai::demo_stream(DEMO_PLAN, send).await?
    } else {
        let session = core.lcu.get("/lol-champ-select/v1/session").await?;
        let card = facts::draft_card(&session, &names(&core.lcu).await?)?;
        ai::stream(&core.ai, ai::PLAN_PROMPT, &card, send).await?
    };
    *core.plan.lock().unwrap() = plan.clone();
    app.emit("plan", plan).map_err(|e| e.to_string())
}

#[tauri::command]
fn current_plan(core: State<'_, Core>) -> String {
    core.plan.lock().unwrap().clone()
}

fn flip_overlay(app: &AppHandle) -> tauri::Result<()> {
    let Some(overlay) = app.get_webview_window("overlay") else {
        return Ok(());
    };
    if overlay.is_visible()? { overlay.hide() } else { overlay.show() }
}

#[tauri::command]
fn toggle_overlay(app: AppHandle) -> Result<(), String> {
    flip_overlay(&app).map_err(|e| e.to_string())
}

fn main() {
    let demo = std::env::var_os("LOLUP_DEMO").is_some();
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state == ShortcutState::Pressed {
                        let _ = flip_overlay(app);
                    }
                })
                .build(),
        )
        .register_asynchronous_uri_scheme_protocol("lcu", |ctx, request, responder| {
            let lcu = ctx.app_handle().state::<Core>().lcu.clone();
            // ponytail: las rutas del cliente solo traen letras, barras y espacios.
            let path = request.uri().path().replace("%2F", "/").replace("%20", " ");
            let path = format!("/{}", path.trim_start_matches('/'));
            tauri::async_runtime::spawn(async move {
                let response = match lcu.asset(&path).await {
                    Ok((kind, bytes)) => http::Response::builder().header("content-type", kind).body(bytes),
                    Err(_) => http::Response::builder().status(404).body(Vec::new()),
                };
                responder.respond(response.expect("respuesta HTTP válida"));
            });
        })
        .manage(Core { lcu: Lcu::new(demo), ai: reqwest::Client::new(), plan: Mutex::default(), lp_file: Mutex::default() })
        .invoke_handler(tauri::generate_handler![
            status,
            catalog,
            profile,
            matches,
            match_detail,
            review,
            generate_plan,
            current_plan,
            toggle_overlay
        ])
        .setup(|app| {
            let overlay = app.get_webview_window("overlay").expect("ventana overlay");
            // El overlay nunca captura clics: el juego los recibe todos.
            overlay.set_ignore_cursor_events(true)?;
            if let Some(monitor) = overlay.primary_monitor()? {
                let width = overlay.outer_size()?.width as i32;
                overlay.set_position(tauri::PhysicalPosition::new(monitor.size().width as i32 - width - 24, 96))?;
            }
            app.global_shortcut().register("ctrl+alt+d")?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error al iniciar LolUp");
}
