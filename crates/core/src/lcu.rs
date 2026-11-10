//! Traduce las respuestas del cliente local de League al modelo común.

use crate::game::{Game, Loadout, Names, Objectives, Participant, Rank, int, minutes_from_seconds, name};
use serde_json::Value;

const SMITE: i64 = 11;

fn stat(player: &Value, key: &str) -> i64 {
    int(&player["stats"][key])
}

/// ponytail: lane y role del cliente son aproximados: en partidas reales marca dos junglas.
/// Castigo es la señal fiable de jungla. La web usa teamPosition de Match-V5.
fn role(player: &Value) -> &'static str {
    if player["spell1Id"].as_i64() == Some(SMITE) || player["spell2Id"].as_i64() == Some(SMITE) {
        return "JUNGLE";
    }
    let timeline = &player["timeline"];
    if timeline["role"] == "SUPPORT" {
        return "SUPPORT";
    }
    match timeline["lane"].as_str() {
        Some("TOP") => "TOP",
        Some("MIDDLE") | Some("MID") => "MIDDLE",
        Some("BOTTOM") | Some("BOT") => "BOTTOM",
        _ => "UNKNOWN",
    }
}

fn loadout(player: &Value, names: &Names) -> Loadout {
    let champion_id = int(&player["championId"]);
    Loadout {
        champion_id,
        champion: name(names, champion_id),
        level: stat(player, "champLevel"),
        items: (0..7).map(|i| stat(player, &format!("item{i}"))).collect(),
        spells: vec![int(&player["spell1Id"]), int(&player["spell2Id"])],
        keystone: stat(player, "perk0"),
        sub_style: stat(player, "perkSubStyle"),
        kills: stat(player, "kills"),
        deaths: stat(player, "deaths"),
        assists: stat(player, "assists"),
        damage: stat(player, "totalDamageDealtToChampions"),
        gold: stat(player, "goldEarned"),
        cs: stat(player, "totalMinionsKilled") + stat(player, "neutralMinionsKilled"),
        vision: stat(player, "visionScore"),
        role: role(player),
    }
}

fn player_name(identity: &Value) -> String {
    let player = &identity["player"];
    match (player["gameName"].as_str(), player["tagLine"].as_str()) {
        (Some(name), Some(tag)) if !name.is_empty() => format!("{name}#{tag}"),
        _ => player["summonerName"].as_str().unwrap_or("Jugador").to_string(),
    }
}

fn objectives(team: &Value) -> Objectives {
    Objectives {
        team: int(&team["teamId"]),
        barons: int(&team["baronKills"]),
        dragons: int(&team["dragonKills"]),
        heralds: int(&team["riftHeraldKills"]),
        grubs: int(&team["hordeKills"]),
        towers: int(&team["towerKills"]),
        inhibitors: int(&team["inhibitorKills"]),
        bans: team["bans"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|b| int(&b["championId"]))
            .filter(|id| *id > 0)
            .collect(),
    }
}

/// Una partida de `/lol-match-history/v1/games/{id}`, o un elemento del historial.
/// Las dos listas del cliente vienen en el mismo orden: verificado en partidas reales.
pub fn game(raw: &Value, names: &Names) -> Result<Game, String> {
    let players = raw["participants"].as_array().ok_or("Partida sin participantes.")?;
    let identities = raw["participantIdentities"].as_array().ok_or("Partida sin jugadores.")?;
    let participants = players
        .iter()
        .zip(identities)
        .enumerate()
        .map(|(index, (p, identity))| Participant {
            id: p["participantId"].as_i64().unwrap_or(index as i64 + 1),
            team: int(&p["teamId"]),
            puuid: identity["player"]["puuid"].as_str().unwrap_or("").to_string(),
            name: player_name(identity),
            win: p["stats"]["win"].as_bool().unwrap_or(false),
            remake: p["stats"]["gameEndedInEarlySurrender"].as_bool().unwrap_or(false),
            loadout: loadout(p, names),
        })
        .collect();
    Ok(Game {
        id: raw["gameId"].as_i64().ok_or("Partida sin identificador.")?,
        minutes: minutes_from_seconds(raw["gameDuration"].as_f64().unwrap_or(0.0)),
        queue_id: int(&raw["queueId"]),
        created_ms: int(&raw["gameCreation"]),
        participants,
        objectives: raw["teams"].as_array().into_iter().flatten().map(objectives).collect(),
    })
}

/// El historial solo trae al jugador en cada partida; alcanza para la lista y los resúmenes.
pub fn history(raw: &Value, names: &Names) -> Vec<Game> {
    raw["games"]["games"].as_array().into_iter().flatten().filter_map(|g| game(g, names).ok()).collect()
}

pub fn frames(timeline: &Value) -> &[Value] {
    timeline["frames"].as_array().map(Vec::as_slice).unwrap_or_default()
}

pub fn rank(ranked: &Value, queue: &str) -> Option<Rank> {
    let entry = &ranked["queueMap"][queue];
    let tier = entry["tier"].as_str().filter(|t| !t.is_empty() && *t != "NONE" && *t != "UNRANKED")?;
    Some(Rank {
        tier: tier.to_string(),
        division: entry["division"].as_str().filter(|d| *d != "NA").unwrap_or("").to_string(),
        lp: int(&entry["leaguePoints"]),
        wins: int(&entry["wins"]),
        losses: int(&entry["losses"]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_ranks_and_hides_unranked() {
        let ranked = json!({ "queueMap": {
            "RANKED_SOLO_5x5": { "tier": "GOLD", "division": "II", "leaguePoints": 40, "wins": 10, "losses": 8 },
            "RANKED_FLEX_SR": { "tier": "MASTER", "division": "NA", "leaguePoints": 5 },
            "OTHER": { "tier": "" }
        }});
        assert_eq!(rank(&ranked, "RANKED_SOLO_5x5").unwrap().division, "II");
        assert_eq!(rank(&ranked, "RANKED_FLEX_SR").unwrap().division, "");
        assert!(rank(&ranked, "OTHER").is_none());
        assert!(rank(&ranked, "MISSING").is_none());
    }

    #[test]
    fn role_prefers_smite_then_support() {
        let jungle = json!({ "spell1Id": 4, "spell2Id": 11, "timeline": { "lane": "TOP", "role": "SOLO" } });
        let fake_jungle = json!({ "spell1Id": 4, "spell2Id": 14, "timeline": { "lane": "JUNGLE", "role": "NONE" } });
        let support = json!({ "timeline": { "lane": "NONE", "role": "SUPPORT" } });
        let unknown = json!({ "timeline": { "lane": "NONE", "role": "SOLO" } });
        assert_eq!(role(&jungle), "JUNGLE");
        assert_eq!(role(&fake_jungle), "UNKNOWN");
        assert_eq!(role(&support), "SUPPORT");
        assert_eq!(role(&unknown), "UNKNOWN");
    }
}
