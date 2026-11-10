//! Perfil, historial y marcador completo, normalizados para la interfaz.

use crate::facts::{Names, name};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

fn int(value: &Value) -> i64 {
    value.as_i64().unwrap_or(0)
}

fn stat(player: &Value, key: &str) -> i64 {
    int(&player["stats"][key])
}

fn my_index(game: &Value, puuid: &str) -> Option<usize> {
    game["participantIdentities"]
        .as_array()?
        .iter()
        .position(|i| i["player"]["puuid"].as_str() == Some(puuid))
}

const SMITE: i64 = 11;

/// ponytail: lane y role del cliente son aproximados: en partidas reales marca dos junglas.
/// Castigo es la señal fiable de jungla. Reemplazar por teamPosition si aparece.
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

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Loadout {
    pub champion_id: i64,
    pub champion: String,
    pub level: i64,
    pub items: Vec<i64>,
    pub spells: Vec<i64>,
    pub keystone: i64,
    pub sub_style: i64,
    pub kills: i64,
    pub deaths: i64,
    pub assists: i64,
    pub damage: i64,
    pub gold: i64,
    pub cs: i64,
    pub vision: i64,
    pub role: &'static str,
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

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MatchRow {
    pub game_id: i64,
    pub win: bool,
    pub remake: bool,
    pub minutes: f64,
    pub queue_id: i64,
    pub created_ms: i64,
    #[serde(flatten)]
    pub player: Loadout,
}

fn minutes(game: &Value) -> f64 {
    (game["gameDuration"].as_f64().unwrap_or(0.0) / 6.0).round() / 10.0
}

pub fn match_rows(history: &Value, puuid: &str, names: &Names) -> Vec<MatchRow> {
    history["games"]["games"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|game| {
            let me = game["participants"].get(my_index(game, puuid)?)?;
            Some(MatchRow {
                game_id: game["gameId"].as_i64()?,
                win: me["stats"]["win"].as_bool()?,
                remake: me["stats"]["gameEndedInEarlySurrender"].as_bool().unwrap_or(false),
                minutes: minutes(game),
                queue_id: int(&game["queueId"]),
                created_ms: int(&game["gameCreation"]),
                player: loadout(me, names),
            })
        })
        .collect()
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChampionLine {
    pub champion_id: i64,
    pub champion: String,
    pub games: i64,
    pub wins: i64,
    pub kda: f64,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RoleLine {
    pub role: &'static str,
    pub games: i64,
    pub wins: i64,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub queue_id: Option<i64>,
    pub games: i64,
    pub wins: i64,
    pub kills: f64,
    pub deaths: f64,
    pub assists: f64,
    pub kda: f64,
    pub champions: Vec<ChampionLine>,
    pub roles: Vec<RoleLine>,
}

fn ratio(kills: i64, deaths: i64, assists: i64) -> f64 {
    ((kills + assists) as f64 * 10.0 / deaths.max(1) as f64).round() / 10.0
}

fn one_decimal(total: i64, games: i64) -> f64 {
    if games == 0 { 0.0 } else { (total as f64 * 10.0 / games as f64).round() / 10.0 }
}

/// Las partidas canceladas por remake no cuentan en los promedios.
fn summarize(queue_id: Option<i64>, rows: &[&MatchRow]) -> Summary {
    let played: Vec<&&MatchRow> = rows.iter().filter(|r| !r.remake).collect();
    let games = played.len() as i64;
    let sum = |pick: fn(&Loadout) -> i64| played.iter().map(|r| pick(&r.player)).sum::<i64>();
    let (kills, deaths, assists) = (sum(|p| p.kills), sum(|p| p.deaths), sum(|p| p.assists));

    let mut champions: HashMap<i64, Vec<&MatchRow>> = HashMap::new();
    let mut roles: BTreeMap<&'static str, (i64, i64)> = BTreeMap::new();
    for row in &played {
        champions.entry(row.player.champion_id).or_default().push(row);
        let entry = roles.entry(row.player.role).or_default();
        entry.0 += 1;
        entry.1 += row.win as i64;
    }

    let mut champion_lines: Vec<ChampionLine> = champions
        .into_iter()
        .map(|(champion_id, games)| {
            let total = |pick: fn(&Loadout) -> i64| games.iter().map(|r| pick(&r.player)).sum::<i64>();
            ChampionLine {
                champion_id,
                champion: games[0].player.champion.clone(),
                games: games.len() as i64,
                wins: games.iter().filter(|r| r.win).count() as i64,
                kda: ratio(total(|p| p.kills), total(|p| p.deaths), total(|p| p.assists)),
            }
        })
        .collect();
    champion_lines.sort_by_key(|c| (-c.games, -c.wins, c.champion_id));

    Summary {
        queue_id,
        games,
        wins: played.iter().filter(|r| r.win).count() as i64,
        kills: one_decimal(kills, games),
        deaths: one_decimal(deaths, games),
        assists: one_decimal(assists, games),
        kda: ratio(kills, deaths, assists),
        champions: champion_lines,
        roles: roles
            .into_iter()
            .filter(|(role, _)| *role != "UNKNOWN")
            .map(|(role, (games, wins))| RoleLine { role, games, wins })
            .collect(),
    }
}

/// Un resumen general y uno por cola, para filtrar sin volver a consultar el cliente.
pub fn summaries(rows: &[MatchRow]) -> Vec<Summary> {
    let all: Vec<&MatchRow> = rows.iter().collect();
    let mut queues: Vec<i64> = rows.iter().map(|r| r.queue_id).collect();
    queues.sort();
    queues.dedup();
    let mut result = vec![summarize(None, &all)];
    for queue in queues {
        let matching: Vec<&MatchRow> = rows.iter().filter(|r| r.queue_id == queue).collect();
        result.push(summarize(Some(queue), &matching));
    }
    result
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub name: String,
    pub me: bool,
    #[serde(flatten)]
    pub stats: Loadout,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    pub win: bool,
    pub kills: i64,
    pub gold: i64,
    pub barons: i64,
    pub dragons: i64,
    pub heralds: i64,
    pub grubs: i64,
    pub towers: i64,
    pub inhibitors: i64,
    pub bans: Vec<i64>,
    pub players: Vec<Player>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct GoldPoint {
    pub minute: f64,
    pub lead: i64,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    pub game_id: i64,
    pub minutes: f64,
    pub queue_id: i64,
    pub created_ms: i64,
    pub teams: Vec<Team>,
    pub gold_lead: Vec<GoldPoint>,
    pub rival: Option<String>,
    pub checkpoints: Vec<Checkpoint>,
    pub events: Vec<Event>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct FrameStats {
    pub gold: i64,
    pub cs: i64,
    pub level: i64,
    pub xp: i64,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Checkpoint {
    pub minute: i64,
    pub mine: FrameStats,
    pub rival: Option<FrameStats>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Event {
    pub minute: f64,
    pub kind: &'static str,
    pub mine: bool,
    pub detail: String,
}

const CHECKPOINT_MINUTES: [i64; 3] = [10, 15, 20];

fn minute_of(milliseconds: i64) -> f64 {
    (milliseconds as f64 / 6_000.0).round() / 10.0
}

fn frame_stats(frame: &Value, participant: i64) -> Option<FrameStats> {
    let stats = frame["participantFrames"].get(participant.to_string())?;
    Some(FrameStats {
        gold: int(&stats["totalGold"]),
        cs: int(&stats["minionsKilled"]) + int(&stats["jungleMinionsKilled"]),
        level: int(&stats["level"]),
        xp: int(&stats["xp"]),
    })
}

fn checkpoints(timeline: &Value, me: i64, rival: Option<i64>) -> Vec<Checkpoint> {
    let frames = timeline["frames"].as_array().map(Vec::as_slice).unwrap_or_default();
    CHECKPOINT_MINUTES
        .iter()
        .filter_map(|minute| {
            let frame = frames.iter().find(|f| (int(&f["timestamp"]) - minute * 60_000).abs() <= 30_000)?;
            Some(Checkpoint {
                minute: *minute,
                mine: frame_stats(frame, me)?,
                rival: rival.and_then(|id| frame_stats(frame, id)),
            })
        })
        .collect()
}

/// Asesinatos propios y objetivos de ambos equipos, en orden de partida.
fn events(timeline: &Value, me: i64, my_team: i64, team_of: &HashMap<i64, i64>) -> Vec<Event> {
    let frames = timeline["frames"].as_array().map(Vec::as_slice).unwrap_or_default();
    let mut list = Vec::new();
    for event in frames.iter().flat_map(|f| f["events"].as_array().into_iter().flatten()) {
        let minute = minute_of(int(&event["timestamp"]));
        let killer = int(&event["killerId"]);
        let by_my_team = team_of.get(&killer) == Some(&my_team);
        let mut push = |kind, mine, detail: &str| list.push(Event { minute, kind, mine, detail: detail.to_string() });
        match event["type"].as_str() {
            Some("CHAMPION_KILL") => {
                let assisted = event["assistingParticipantIds"].as_array().is_some_and(|ids| ids.iter().any(|id| int(id) == me));
                if killer == me {
                    push("kill", true, "");
                } else if int(&event["victimId"]) == me {
                    push("death", false, "");
                } else if assisted {
                    push("assist", true, "");
                }
            }
            // Sin asesino no se puede atribuir a un equipo.
            Some("ELITE_MONSTER_KILL") if team_of.contains_key(&killer) => match event["monsterType"].as_str() {
                Some("DRAGON") => push("dragon", by_my_team, event["monsterSubType"].as_str().unwrap_or("")),
                Some("BARON_NASHOR") => push("baron", by_my_team, ""),
                Some("RIFTHERALD") => push("herald", by_my_team, ""),
                Some("HORDE") => push("grubs", by_my_team, ""),
                _ => {}
            },
            // teamId es el dueño del edificio: verificado contra towerKills en partidas reales.
            Some("BUILDING_KILL") => {
                let kind = if event["buildingType"] == "INHIBITOR_BUILDING" { "inhibitor" } else { "tower" };
                push(kind, int(&event["teamId"]) != my_team, "");
            }
            _ => {}
        }
    }
    list
}

fn player_name(identity: &Value) -> String {
    let player = &identity["player"];
    match (player["gameName"].as_str(), player["tagLine"].as_str()) {
        (Some(name), Some(tag)) if !name.is_empty() => format!("{name}#{tag}"),
        _ => player["summonerName"].as_str().unwrap_or("Jugador").to_string(),
    }
}

/// Diferencia de oro desde el punto de vista del equipo del jugador.
fn gold_lead(timeline: &Value, team_of: &HashMap<i64, i64>, my_team: i64) -> Vec<GoldPoint> {
    timeline["frames"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|frame| {
            let lead = frame["participantFrames"]
                .as_object()
                .into_iter()
                .flat_map(|players| players.values())
                .map(|p| {
                    let gold = int(&p["totalGold"]);
                    if team_of.get(&int(&p["participantId"])) == Some(&my_team) { gold } else { -gold }
                })
                .sum();
            // El último cuadro cae a mitad de minuto: truncarlo dibuja un salto vertical.
            GoldPoint { minute: minute_of(int(&frame["timestamp"])), lead }
        })
        .collect()
}

pub fn match_detail(game: &Value, timeline: Option<&Value>, puuid: &str, names: &Names) -> Result<Detail, String> {
    let my_index = my_index(game, puuid).ok_or("No se encontró al jugador en la partida.")?;
    let players = game["participants"].as_array().ok_or("Partida sin participantes.")?;
    let identities = game["participantIdentities"].as_array().ok_or("Partida sin jugadores.")?;
    let me = players.get(my_index).ok_or("Partida incompleta.")?;
    let my_team = int(&me["teamId"]);
    let me_id = int(&me["participantId"]);
    let team_of: HashMap<i64, i64> =
        players.iter().map(|p| (int(&p["participantId"]), int(&p["teamId"]))).collect();

    // Solo hay rival si la posición es inequívoca en ambos equipos.
    let my_role = role(me);
    let same_role = |team_is_mine: bool| -> Vec<&Value> {
        players.iter().filter(|p| role(p) == my_role && (int(&p["teamId"]) == my_team) == team_is_mine).collect()
    };
    let rival = match (my_role, same_role(true).len(), same_role(false).as_slice()) {
        ("UNKNOWN", _, _) => None,
        (_, 1, [player]) => Some(*player),
        _ => None,
    };

    let mut teams: Vec<&Value> = game["teams"].as_array().ok_or("Partida sin equipos.")?.iter().collect();
    teams.sort_by_key(|team| int(&team["teamId"]) != my_team);

    let teams = teams
        .into_iter()
        .map(|team| {
            let id = int(&team["teamId"]);
            let roster: Vec<(usize, &Value)> =
                players.iter().enumerate().filter(|(_, p)| int(&p["teamId"]) == id).collect();
            let list: Vec<Player> = roster
                .iter()
                .map(|(index, p)| Player {
                    name: identities.get(*index).map(player_name).unwrap_or_else(|| "Jugador".into()),
                    me: *index == my_index,
                    stats: loadout(p, names),
                })
                .collect();
            Team {
                win: roster.first().is_some_and(|(_, p)| p["stats"]["win"].as_bool() == Some(true)),
                kills: list.iter().map(|p| p.stats.kills).sum(),
                gold: list.iter().map(|p| p.stats.gold).sum(),
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
                players: list,
            }
        })
        .collect();

    Ok(Detail {
        game_id: int(&game["gameId"]),
        minutes: minutes(game),
        queue_id: int(&game["queueId"]),
        created_ms: int(&game["gameCreation"]),
        teams,
        gold_lead: timeline.map(|t| gold_lead(t, &team_of, my_team)).unwrap_or_default(),
        rival: rival.map(|p| name(names, int(&p["championId"]))),
        checkpoints: timeline
            .map(|t| checkpoints(t, me_id, rival.map(|p| int(&p["participantId"]))))
            .unwrap_or_default(),
        events: timeline.map(|t| events(t, me_id, my_team, &team_of)).unwrap_or_default(),
    })
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Rank {
    pub tier: String,
    pub division: String,
    pub lp: i64,
    pub wins: i64,
    pub losses: i64,
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

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Mastery {
    pub champion_id: i64,
    pub champion: String,
    pub level: i64,
    pub points: i64,
}

pub fn top_mastery(list: &Value, names: &Names, limit: usize) -> Vec<Mastery> {
    let mut rows: Vec<Mastery> = list
        .as_array()
        .into_iter()
        .flatten()
        .map(|m| Mastery {
            champion_id: int(&m["championId"]),
            champion: name(names, int(&m["championId"])),
            level: int(&m["championLevel"]),
            points: int(&m["championPoints"]),
        })
        .collect();
    rows.sort_by_key(|m| -m.points);
    rows.truncate(limit);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::champion_names;
    use serde_json::json;

    fn fixture(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    fn names() -> Names {
        champion_names(&fixture(include_str!("../fixtures/champions.json")))
    }

    fn rows() -> Vec<MatchRow> {
        match_rows(&fixture(include_str!("../fixtures/history.json")), "demo-me", &names())
    }

    fn detail() -> Detail {
        let game = fixture(include_str!("../fixtures/game.json"));
        let timeline = fixture(include_str!("../fixtures/timeline.json"));
        match_detail(&game, Some(&timeline), "demo-me", &names()).unwrap()
    }

    #[test]
    fn lists_only_rows_with_my_participant() {
        let rows = rows();
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].player.champion, "Ahri");
        assert_eq!(rows[0].player.items.len(), 7);
        let history = fixture(include_str!("../fixtures/history.json"));
        assert!(match_rows(&history, "otro", &names()).is_empty());
    }

    #[test]
    fn summaries_group_by_champion_and_queue() {
        let all = summaries(&rows());
        assert_eq!(all[0].queue_id, None);
        assert_eq!((all[0].games, all[0].wins), (5, 3));
        assert_eq!(all[0].champions[0].champion, "Ahri");
        assert_eq!((all[0].champions[0].games, all[0].champions[0].wins), (3, 2));
        assert_eq!(all[0].kda, 2.5); // (34 + 34) / 27
        assert_eq!(all.len(), 3);
        assert_eq!(all[1].queue_id, Some(420));
        assert_eq!(all[1].games + all[2].games, 5);
    }

    #[test]
    fn remakes_do_not_count() {
        let mut list = rows();
        list[0].remake = true;
        let all = summaries(&list);
        assert_eq!(all[0].games, 4);
        assert!(all[0].champions.iter().all(|c| c.games <= 2));
    }

    #[test]
    fn empty_history_does_not_divide_by_zero() {
        let all = summaries(&[]);
        assert_eq!(all.len(), 1);
        assert_eq!((all[0].games, all[0].kda, all[0].kills), (0, 0.0, 0.0));
    }

    #[test]
    fn detail_puts_my_team_first_and_marks_me() {
        let d = detail();
        assert_eq!(d.teams[0].players.iter().filter(|p| p.me).count(), 1);
        assert_eq!(d.teams[1].players.iter().filter(|p| p.me).count(), 0);
        assert!(!d.teams[0].win);
        assert!(d.teams[1].win);
        assert_eq!((d.teams[0].kills, d.teams[1].kills), (21, 34));
        assert_eq!(d.teams[1].towers, 9);
        assert_eq!(d.teams[0].players[0].name, "EjemploUno#LAS");
        assert!(d.teams[0].bans.iter().all(|id| *id > 0));
    }

    #[test]
    fn gold_lead_is_from_my_team_point_of_view() {
        let d = detail();
        assert_eq!(d.gold_lead.first().unwrap(), &GoldPoint { minute: 0.0, lead: 0 });
        assert_eq!(d.gold_lead[1], GoldPoint { minute: 5.0, lead: 500 });
        assert_eq!(d.gold_lead.last().unwrap(), &GoldPoint { minute: 30.0, lead: -7000 });
    }

    #[test]
    fn detail_survives_a_missing_timeline() {
        let game = fixture(include_str!("../fixtures/game.json"));
        let d = match_detail(&game, None, "demo-me", &names()).unwrap();
        assert!(d.gold_lead.is_empty());
        assert!(d.checkpoints.is_empty());
        assert!(d.events.is_empty());
    }

    #[test]
    fn checkpoints_compare_me_with_my_lane_rival() {
        let d = detail();
        assert_eq!(d.rival.as_deref(), Some("Yasuo"));
        assert_eq!(d.checkpoints.iter().map(|c| c.minute).collect::<Vec<_>>(), [10, 15, 20]);
        let ten = &d.checkpoints[0];
        assert_eq!((ten.mine.gold, ten.mine.cs), (12000, 85));
        assert_eq!(ten.rival.as_ref().unwrap().gold, 13000);
    }

    #[test]
    fn no_rival_when_the_role_is_ambiguous() {
        let mut game = fixture(include_str!("../fixtures/game.json"));
        game["participants"][7]["timeline"] = json!({ "lane": "MIDDLE", "role": "DUO" });
        let timeline = fixture(include_str!("../fixtures/timeline.json"));
        let d = match_detail(&game, Some(&timeline), "demo-me", &names()).unwrap();
        assert_eq!(d.rival, None);
        assert!(d.checkpoints.iter().all(|c| c.rival.is_none()));
    }

    #[test]
    fn events_attribute_objectives_to_the_right_team() {
        let events = detail().events;
        let kinds: Vec<(&str, bool)> = events.iter().map(|e| (e.kind, e.mine)).collect();
        assert_eq!(
            kinds,
            [
                ("kill", true),
                ("death", false),
                ("assist", true),
                ("dragon", false),
                ("tower", true),
                ("tower", false),
                ("inhibitor", false),
            ]
        );
        assert_eq!(events[3].detail, "FIRE_DRAGON");
    }

    #[test]
    fn jungler_needs_smite() {
        let with = json!({ "spell1Id": 4, "spell2Id": 11, "timeline": { "lane": "TOP", "role": "SOLO" } });
        let without = json!({ "spell1Id": 4, "spell2Id": 14, "timeline": { "lane": "JUNGLE", "role": "NONE" } });
        assert_eq!(role(&with), "JUNGLE");
        assert_eq!(role(&without), "UNKNOWN");
    }

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
    fn role_prefers_support_and_tolerates_unknown() {
        assert_eq!(role(&json!({ "timeline": { "lane": "NONE", "role": "SUPPORT" } })), "SUPPORT");
        assert_eq!(role(&json!({ "timeline": { "lane": "MIDDLE", "role": "SOLO" } })), "MIDDLE");
        assert_eq!(role(&json!({ "timeline": { "lane": "NONE", "role": "SOLO" } })), "UNKNOWN");
    }

    #[test]
    fn mastery_is_sorted_and_limited() {
        let list = json!([
            { "championId": 103, "championLevel": 7, "championPoints": 10 },
            { "championId": 157, "championLevel": 9, "championPoints": 90 },
            { "championId": 64, "championLevel": 5, "championPoints": 50 }
        ]);
        let top = top_mastery(&list, &names(), 2);
        assert_eq!(top.iter().map(|m| m.points).collect::<Vec<_>>(), [90, 50]);
    }
}
