//! Convierte el JSON del cliente en fichas de hechos sin datos de identidad.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

pub type Names = HashMap<i64, String>;

pub fn champion_names(summary: &Value) -> Names {
    summary
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| Some((c["id"].as_i64()?, c["name"].as_str()?.to_string())))
        .collect()
}

pub fn name(names: &Names, id: i64) -> String {
    names.get(&id).cloned().unwrap_or_else(|| format!("Campeón {id}"))
}

fn stat(participant: &Value, key: &str) -> i64 {
    participant["stats"][key].as_i64().unwrap_or(0)
}

fn cs(participant: &Value) -> i64 {
    stat(participant, "totalMinionsKilled") + stat(participant, "neutralMinionsKilled")
}

fn percent(part: i64, total: i64) -> f64 {
    if total == 0 { 0.0 } else { round1(part as f64 * 100.0 / total as f64) }
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

/// Índice del jugador. Las dos listas del cliente vienen en el mismo orden.
fn my_index(game: &Value, puuid: &str) -> Option<usize> {
    game["participantIdentities"]
        .as_array()?
        .iter()
        .position(|i| i["player"]["puuid"].as_str() == Some(puuid))
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Facts {
    pub champion: String,
    pub win: bool,
    pub minutes: f64,
    pub kills: i64,
    pub deaths: i64,
    pub assists: i64,
    pub kill_participation: f64,
    pub damage: i64,
    pub damage_share: f64,
    pub taken_share: f64,
    pub gold: i64,
    pub gold_share: f64,
    pub cs: i64,
    pub cs_per_minute: f64,
    pub vision: i64,
    pub team_kills: i64,
    pub enemy_kills: i64,
    pub allies: Vec<String>,
    pub enemies: Vec<String>,
}

pub fn game_facts(game: &Value, puuid: &str, names: &Names) -> Result<Facts, String> {
    let index = my_index(game, puuid).ok_or("No se encontró al jugador en la partida.")?;
    let players = game["participants"].as_array().ok_or("Partida sin participantes.")?;
    let me = players.get(index).ok_or("Partida incompleta.")?;
    let team = me["teamId"].as_i64();
    let (mine, theirs): (Vec<&Value>, Vec<&Value>) =
        players.iter().partition(|p| p["teamId"].as_i64() == team);

    let sum = |side: &[&Value], key: &str| side.iter().map(|p| stat(p, key)).sum::<i64>();
    let champs = |side: Vec<&Value>| {
        side.iter()
            .filter(|p| !std::ptr::eq(**p, me))
            .map(|p| name(names, p["championId"].as_i64().unwrap_or(0)))
            .collect::<Vec<_>>()
    };

    let minutes = game["gameDuration"].as_f64().unwrap_or(0.0) / 60.0;
    let (kills, assists) = (stat(me, "kills"), stat(me, "assists"));
    let team_kills = sum(&mine, "kills");
    let damage = stat(me, "totalDamageDealtToChampions");
    let gold = stat(me, "goldEarned");
    let creeps = cs(me);

    Ok(Facts {
        champion: name(names, me["championId"].as_i64().unwrap_or(0)),
        win: me["stats"]["win"].as_bool().unwrap_or(false),
        minutes: round1(minutes),
        kills,
        deaths: stat(me, "deaths"),
        assists,
        kill_participation: percent(kills + assists, team_kills),
        damage,
        damage_share: percent(damage, sum(&mine, "totalDamageDealtToChampions")),
        taken_share: percent(stat(me, "totalDamageTaken"), sum(&mine, "totalDamageTaken")),
        gold,
        gold_share: percent(gold, sum(&mine, "goldEarned")),
        cs: creeps,
        cs_per_minute: if minutes > 0.0 { round1(creeps as f64 / minutes) } else { 0.0 },
        vision: stat(me, "visionScore"),
        team_kills,
        enemy_kills: sum(&theirs, "kills"),
        allies: champs(mine),
        enemies: champs(theirs),
    })
}

/// Cada dato va junto a su dueño, para que el modelo no los confunda.
pub fn fact_card(f: &Facts) -> String {
    format!(
        "Resultado: {result} en {min} minutos.\n\
         Tu campeón: {champ}.\n\
         Tu equipo además tenía: {allies}.\n\
         El equipo rival tenía: {enemies}.\n\
         Tus eliminaciones/muertes/asistencias: {k}/{d}/{a}.\n\
         Tu participación en asesinatos de tu equipo: {kp}%.\n\
         Tu daño a campeones: {dmg}, que es {ds}% del daño de tu equipo.\n\
         Tu daño recibido es {ts}% del recibido por tu equipo.\n\
         Tu oro: {gold}, que es {gs}% del oro de tu equipo.\n\
         Tus súbditos y monstruos: {cs} ({csm} por minuto).\n\
         Tu puntaje de visión: {vision}.\n\
         Asesinatos totales de tu equipo: {tk}. Asesinatos totales del equipo rival: {ek}.",
        result = if f.win { "victoria" } else { "derrota" },
        min = f.minutes,
        champ = f.champion,
        allies = f.allies.join(", "),
        enemies = f.enemies.join(", "),
        k = f.kills,
        d = f.deaths,
        a = f.assists,
        kp = f.kill_participation,
        dmg = f.damage,
        ds = f.damage_share,
        ts = f.taken_share,
        gold = f.gold,
        gs = f.gold_share,
        cs = f.cs,
        csm = f.cs_per_minute,
        vision = f.vision,
        tk = f.team_kills,
        ek = f.enemy_kills,
    )
}

/// El plan debe ser previo a la partida: nunca se genera con la partida en curso.
pub fn plan_allowed(phase: &str) -> bool {
    phase == "ChampSelect"
}

pub fn draft_card(session: &Value, names: &Names) -> Result<String, String> {
    let my_cell = session["localPlayerCellId"].as_i64().ok_or("Sin sesión de selección.")?;
    let team = session["myTeam"].as_array().ok_or("Sin sesión de selección.")?;
    let me = team
        .iter()
        .find(|p| p["cellId"].as_i64() == Some(my_cell))
        .ok_or("Jugador no encontrado en la selección.")?;
    let my_champ = me["championId"].as_i64().filter(|id| *id > 0).ok_or("Elegí un campeón primero.")?;

    // Solo campeones y posiciones visibles. Nunca jugadores.
    let picked = |players: &Value, skip: Option<i64>| {
        players
            .as_array()
            .into_iter()
            .flatten()
            .filter(|p| p["cellId"].as_i64() != skip)
            .filter_map(|p| p["championId"].as_i64().filter(|id| *id > 0))
            .map(|id| name(names, id))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let or_unknown = |s: String| if s.is_empty() { "todavía no visible".to_string() } else { s };

    Ok(format!(
        "Tu campeón: {}.\nTu posición: {}.\nCampeones de tu equipo: {}.\nCampeones del equipo rival: {}.",
        name(names, my_champ),
        me["assignedPosition"].as_str().filter(|s| !s.is_empty()).unwrap_or("sin asignar"),
        or_unknown(picked(&session["myTeam"], Some(my_cell))),
        or_unknown(picked(&session["theirTeam"], None)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    fn names() -> Names {
        champion_names(&fixture(include_str!("../fixtures/champions.json")))
    }

    fn game() -> Value {
        fixture(include_str!("../fixtures/game.json"))
    }

    #[test]
    fn computes_my_facts_from_a_full_game() {
        let f = game_facts(&game(), "demo-me", &names()).unwrap();
        assert_eq!((f.champion.as_str(), f.win), ("Ahri", false));
        assert_eq!((f.kills, f.deaths, f.assists), (4, 8, 6));
        assert_eq!((f.team_kills, f.enemy_kills), (21, 34));
        assert_eq!(f.kill_participation, 47.6);
        assert_eq!(f.damage_share, 25.7);
        assert_eq!(f.cs, 181);
        assert_eq!(f.cs_per_minute, 6.0);
        assert_eq!(f.allies.len(), 4);
        assert_eq!(f.enemies.len(), 5);
        assert!(!f.allies.contains(&"Ahri".to_string()));
    }

    #[test]
    fn fact_card_never_contains_player_identity() {
        let f = game_facts(&game(), "demo-me", &names()).unwrap();
        let card = fact_card(&f);
        for secret in ["demo-me", "demo-ally", "demo-enemy", "EjemploUno", "EjemploRival"] {
            assert!(!card.contains(secret), "{secret}");
        }
        assert!(card.contains("47.6%"));
    }

    #[test]
    fn empty_game_does_not_divide_by_zero() {
        let game = json!({
            "gameDuration": 0,
            "participantIdentities": [{ "player": { "puuid": "a" } }],
            "participants": [{ "teamId": 100, "championId": 1, "stats": {} }]
        });
        let f = game_facts(&game, "a", &Names::new()).unwrap();
        assert_eq!((f.kill_participation, f.cs_per_minute), (0.0, 0.0));
        assert_eq!(f.champion, "Campeón 1");
        assert!(game_facts(&game, "otro", &Names::new()).is_err());
    }

    #[test]
    fn plan_only_runs_in_champion_select() {
        assert!(plan_allowed("ChampSelect"));
        for phase in ["InProgress", "GameStart", "Lobby", "EndOfGame", "None"] {
            assert!(!plan_allowed(phase), "{phase}");
        }
    }

    #[test]
    fn draft_card_uses_champions_only() {
        let session = fixture(include_str!("../fixtures/session.json"));
        let card = draft_card(&session, &names()).unwrap();
        assert!(card.contains("Tu campeón: Ahri."));
        assert!(card.contains("Tu posición: MIDDLE."));
        assert!(!card.contains("demo-"));
        let not_picked = json!({
            "localPlayerCellId": 0,
            "myTeam": [{ "cellId": 0, "championId": 0 }],
            "theirTeam": []
        });
        assert!(draft_card(&not_picked, &names()).is_err());
    }
}
