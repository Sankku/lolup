//! Traduce las respuestas de la API oficial de Riot (Match-V5 y League-V4) al modelo común.

use crate::game::{Game, Loadout, Names, Objectives, Participant, Rank, int, minutes_from_seconds};
use serde_json::Value;

/// teamPosition es la posición que asigna Riot, mucho más fiable que la del cliente.
fn role(player: &Value) -> &'static str {
    match player["teamPosition"].as_str() {
        Some("TOP") => "TOP",
        Some("JUNGLE") => "JUNGLE",
        Some("MIDDLE") => "MIDDLE",
        Some("BOTTOM") => "BOTTOM",
        Some("UTILITY") => "SUPPORT",
        _ => "UNKNOWN",
    }
}

fn loadout(player: &Value, names: &Names) -> Loadout {
    let champion_id = int(&player["championId"]);
    let styles = &player["perks"]["styles"];
    let champion = names
        .get(&champion_id)
        .cloned()
        .or_else(|| player["championName"].as_str().map(str::to_string))
        .unwrap_or_else(|| format!("Campeón {champion_id}"));
    Loadout {
        champion_id,
        champion,
        level: int(&player["champLevel"]),
        items: (0..7).map(|i| int(&player[format!("item{i}")])).collect(),
        spells: vec![int(&player["summoner1Id"]), int(&player["summoner2Id"])],
        keystone: int(&styles[0]["selections"][0]["perk"]),
        sub_style: int(&styles[1]["style"]),
        kills: int(&player["kills"]),
        deaths: int(&player["deaths"]),
        assists: int(&player["assists"]),
        damage: int(&player["totalDamageDealtToChampions"]),
        gold: int(&player["goldEarned"]),
        cs: int(&player["totalMinionsKilled"]) + int(&player["neutralMinionsKilled"]),
        vision: int(&player["visionScore"]),
        role: role(player),
    }
}

fn riot_id(player: &Value) -> String {
    match (player["riotIdGameName"].as_str(), player["riotIdTagline"].as_str()) {
        (Some(name), Some(tag)) if !name.is_empty() => format!("{name}#{tag}"),
        _ => player["summonerName"].as_str().unwrap_or("Jugador").to_string(),
    }
}

fn objectives(team: &Value) -> Objectives {
    let kills = |key: &str| int(&team["objectives"][key]["kills"]);
    Objectives {
        team: int(&team["teamId"]),
        barons: kills("baron"),
        dragons: kills("dragon"),
        heralds: kills("riftHerald"),
        grubs: kills("horde"),
        towers: kills("tower"),
        inhibitors: kills("inhibitor"),
        bans: team["bans"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|b| int(&b["championId"]))
            .filter(|id| *id > 0)
            .collect(),
    }
}

/// Una partida de `/lol/match/v5/matches/{matchId}`.
pub fn game(raw: &Value, names: &Names) -> Result<Game, String> {
    let info = &raw["info"];
    let players = info["participants"].as_array().ok_or("Partida sin participantes.")?;
    let participants = players
        .iter()
        .map(|p| Participant {
            id: int(&p["participantId"]),
            team: int(&p["teamId"]),
            puuid: p["puuid"].as_str().unwrap_or("").to_string(),
            name: riot_id(p),
            win: p["win"].as_bool().unwrap_or(false),
            remake: p["gameEndedInEarlySurrender"].as_bool().unwrap_or(false),
            loadout: loadout(p, names),
        })
        .collect();
    Ok(Game {
        id: info["gameId"].as_i64().ok_or("Partida sin identificador.")?,
        // ponytail: gameDuration está en segundos desde 2021; partidas anteriores no se muestran.
        minutes: minutes_from_seconds(info["gameDuration"].as_f64().unwrap_or(0.0)),
        queue_id: int(&info["queueId"]),
        created_ms: int(&info["gameCreation"]),
        participants,
        objectives: info["teams"].as_array().into_iter().flatten().map(objectives).collect(),
    })
}

/// Cuadros de `/lol/match/v5/matches/{matchId}/timeline`.
pub fn frames(timeline: &Value) -> &[Value] {
    timeline["info"]["frames"].as_array().map(Vec::as_slice).unwrap_or_default()
}

/// Entradas de League-V4 para un jugador, filtradas por cola.
pub fn rank(entries: &Value, queue: &str) -> Option<Rank> {
    let entry = entries.as_array()?.iter().find(|e| e["queueType"] == queue)?;
    let tier = entry["tier"].as_str()?.to_string();
    let apex = matches!(tier.as_str(), "MASTER" | "GRANDMASTER" | "CHALLENGER");
    Some(Rank {
        division: if apex { String::new() } else { entry["rank"].as_str().unwrap_or("").to_string() },
        tier,
        lp: int(&entry["leaguePoints"]),
        wins: int(&entry["wins"]),
        losses: int(&entry["losses"]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::tests::{fixture, lcu_detail, names};
    use crate::game::{detail, row};
    use serde_json::json;

    fn v5_game() -> Game {
        game(&fixture(include_str!("../fixtures/match_v5_game.json")), &names()).unwrap()
    }

    fn v5_timeline() -> Value {
        fixture(include_str!("../fixtures/match_v5_timeline.json"))
    }

    #[test]
    fn same_match_gives_the_same_review_from_both_sources() {
        let mut from_v5 = detail(&v5_game(), Some(frames(&v5_timeline())), "demo-me").unwrap();
        assert!(!from_v5.build.is_empty() && !from_v5.skills.is_empty());
        from_v5.build.clear();
        from_v5.skills.clear();
        assert_eq!(from_v5, lcu_detail());
    }

    #[test]
    fn reads_build_and_skill_order() {
        let d = detail(&v5_game(), Some(frames(&v5_timeline())), "demo-me").unwrap();
        assert_eq!(d.build.iter().map(|s| s.item).collect::<Vec<_>>(), [1056, 2003, 3020, 6655]);
        assert_eq!(d.skills, [1, 3, 1, 2, 1, 4]);
    }

    #[test]
    fn row_uses_riot_positions() {
        let r = row(&v5_game(), "demo-me").unwrap();
        assert_eq!((r.player.champion.as_str(), r.player.role, r.win), ("Ahri", "MIDDLE", false));
        assert_eq!(r.player.keystone, 8112);
    }

    #[test]
    fn unknown_champion_falls_back_to_the_payload_name() {
        let raw = json!({ "info": { "gameId": 1, "participants": [
            { "participantId": 1, "teamId": 100, "puuid": "a", "championId": 999, "championName": "Nuevo" }
        ]}});
        assert_eq!(game(&raw, &names()).unwrap().participants[0].loadout.champion, "Nuevo");
    }

    #[test]
    fn reads_league_entries() {
        let entries = json!([
            { "queueType": "RANKED_FLEX_SR", "tier": "GOLD", "rank": "I", "leaguePoints": 10, "wins": 1, "losses": 0 },
            { "queueType": "RANKED_SOLO_5x5", "tier": "GRANDMASTER", "rank": "I", "leaguePoints": 874, "wins": 313, "losses": 255 }
        ]);
        let solo = rank(&entries, "RANKED_SOLO_5x5").unwrap();
        assert_eq!((solo.tier.as_str(), solo.division.as_str(), solo.lp), ("GRANDMASTER", "", 874));
        assert_eq!(rank(&entries, "RANKED_FLEX_SR").unwrap().division, "I");
        assert!(rank(&json!([]), "RANKED_SOLO_5x5").is_none());
    }
}
