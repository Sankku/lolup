//! Modelo común de una partida y los cálculos que se muestran en escritorio y web.

use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

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

pub(crate) fn int(value: &Value) -> i64 {
    value.as_i64().unwrap_or(0)
}

pub(crate) fn minute_of(milliseconds: i64) -> f64 {
    (milliseconds as f64 / 6_000.0).round() / 10.0
}

pub(crate) fn minutes_from_seconds(seconds: f64) -> f64 {
    (seconds / 6.0).round() / 10.0
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

/// Un jugador tal como lo entregan las fuentes, antes de armar las vistas.
#[derive(Debug, Clone)]
pub struct Participant {
    pub id: i64,
    pub team: i64,
    pub puuid: String,
    pub name: String,
    pub win: bool,
    pub remake: bool,
    pub loadout: Loadout,
}

#[derive(Debug, Clone, Default)]
pub struct Objectives {
    pub team: i64,
    pub barons: i64,
    pub dragons: i64,
    pub heralds: i64,
    pub grubs: i64,
    pub towers: i64,
    pub inhibitors: i64,
    pub bans: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub id: i64,
    pub minutes: f64,
    pub queue_id: i64,
    pub created_ms: i64,
    pub participants: Vec<Participant>,
    pub objectives: Vec<Objectives>,
}

impl Game {
    fn player(&self, puuid: &str) -> Option<&Participant> {
        self.participants.iter().find(|p| p.puuid == puuid)
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

pub fn row(game: &Game, puuid: &str) -> Option<MatchRow> {
    let me = game.player(puuid)?;
    Some(MatchRow {
        game_id: game.id,
        win: me.win,
        remake: me.remake,
        minutes: game.minutes,
        queue_id: game.queue_id,
        created_ms: game.created_ms,
        player: me.loadout.clone(),
    })
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

/// Un resumen general y uno por cola, para filtrar sin volver a consultar la fuente.
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

#[derive(Serialize, Debug, PartialEq)]
pub struct BuildStep {
    pub minute: f64,
    pub item: i64,
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
    /// Vacíos cuando la fuente no trae compras ni subidas de nivel, como el cliente local.
    pub build: Vec<BuildStep>,
    pub skills: Vec<i64>,
}

const CHECKPOINT_MINUTES: [i64; 3] = [10, 15, 20];

fn timeline_events(frames: &[Value]) -> impl Iterator<Item = &Value> {
    frames.iter().flat_map(|f| f["events"].as_array().into_iter().flatten())
}

/// Diferencia de oro desde el punto de vista del equipo del jugador.
fn gold_lead(frames: &[Value], team_of: &HashMap<i64, i64>, my_team: i64) -> Vec<GoldPoint> {
    frames
        .iter()
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

fn frame_stats(frame: &Value, participant: i64) -> Option<FrameStats> {
    let stats = frame["participantFrames"].get(participant.to_string())?;
    Some(FrameStats {
        gold: int(&stats["totalGold"]),
        cs: int(&stats["minionsKilled"]) + int(&stats["jungleMinionsKilled"]),
        level: int(&stats["level"]),
        xp: int(&stats["xp"]),
    })
}

fn checkpoints(frames: &[Value], me: i64, rival: Option<i64>) -> Vec<Checkpoint> {
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
fn events(frames: &[Value], me: i64, my_team: i64, team_of: &HashMap<i64, i64>) -> Vec<Event> {
    let mut list = Vec::new();
    for event in timeline_events(frames) {
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

/// Compras y habilidades del jugador. Una compra deshecha se quita.
fn build_and_skills(frames: &[Value], me: i64) -> (Vec<BuildStep>, Vec<i64>) {
    let (mut build, mut skills) = (Vec::new(), Vec::new());
    for event in timeline_events(frames).filter(|e| int(&e["participantId"]) == me) {
        match event["type"].as_str() {
            Some("ITEM_PURCHASED") => build.push(BuildStep {
                minute: minute_of(int(&event["timestamp"])),
                item: int(&event["itemId"]),
            }),
            Some("ITEM_UNDO") => {
                let undone = int(&event["beforeId"]);
                if let Some(index) = build.iter().rposition(|step| step.item == undone) {
                    build.remove(index);
                }
            }
            // Las evoluciones de habilidades no son un punto de habilidad.
            Some("SKILL_LEVEL_UP") if event["levelUpType"] != "EVOLVE" => skills.push(int(&event["skillSlot"])),
            _ => {}
        }
    }
    (build, skills)
}

pub fn detail(game: &Game, frames: Option<&[Value]>, puuid: &str) -> Result<Detail, String> {
    let me = game.player(puuid).ok_or("No se encontró al jugador en la partida.")?;
    let team_of: HashMap<i64, i64> = game.participants.iter().map(|p| (p.id, p.team)).collect();

    // Solo hay rival si la posición es inequívoca en ambos equipos.
    let role = me.loadout.role;
    let same_role = |mine: bool| -> Vec<&Participant> {
        game.participants.iter().filter(|p| p.loadout.role == role && (p.team == me.team) == mine).collect()
    };
    let rival = match (role, same_role(true).len(), same_role(false).as_slice()) {
        ("UNKNOWN", _, _) => None,
        (_, 1, [player]) => Some(*player),
        _ => None,
    };

    let mut objectives = game.objectives.clone();
    if objectives.is_empty() {
        let mut teams: Vec<i64> = game.participants.iter().map(|p| p.team).collect();
        teams.dedup();
        objectives = teams.into_iter().map(|team| Objectives { team, ..Default::default() }).collect();
    }
    objectives.sort_by_key(|o| o.team != me.team);

    let teams = objectives
        .into_iter()
        .map(|o| {
            let roster: Vec<&Participant> = game.participants.iter().filter(|p| p.team == o.team).collect();
            Team {
                win: roster.first().is_some_and(|p| p.win),
                kills: roster.iter().map(|p| p.loadout.kills).sum(),
                gold: roster.iter().map(|p| p.loadout.gold).sum(),
                barons: o.barons,
                dragons: o.dragons,
                heralds: o.heralds,
                grubs: o.grubs,
                towers: o.towers,
                inhibitors: o.inhibitors,
                bans: o.bans,
                players: roster
                    .iter()
                    .map(|p| Player { name: p.name.clone(), me: p.puuid == puuid, stats: p.loadout.clone() })
                    .collect(),
            }
        })
        .collect();

    let frames = frames.unwrap_or_default();
    let (build, skills) = build_and_skills(frames, me.id);
    Ok(Detail {
        game_id: game.id,
        minutes: game.minutes,
        queue_id: game.queue_id,
        created_ms: game.created_ms,
        teams,
        gold_lead: gold_lead(frames, &team_of, me.team),
        rival: rival.map(|p| p.loadout.champion.clone()),
        checkpoints: checkpoints(frames, me.id, rival.map(|p| p.id)),
        events: events(frames, me.id, me.team, &team_of),
        build,
        skills,
    })
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Rank {
    pub tier: String,
    pub division: String,
    pub lp: i64,
    pub wins: i64,
    pub losses: i64,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Mastery {
    pub champion_id: i64,
    pub champion: String,
    pub level: i64,
    pub points: i64,
}

/// El cliente local y Champion-Mastery-V4 usan los mismos nombres de campo.
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
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    pub fn fixture(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    pub fn names() -> Names {
        champion_names(&fixture(include_str!("../fixtures/champions.json")))
    }

    fn rows() -> Vec<MatchRow> {
        let history = fixture(include_str!("../fixtures/history.json"));
        crate::lcu::history(&history, &names()).iter().filter_map(|g| row(g, "demo-me")).collect()
    }

    pub fn lcu_detail() -> Detail {
        let game = crate::lcu::game(&fixture(include_str!("../fixtures/game.json")), &names()).unwrap();
        let timeline = fixture(include_str!("../fixtures/timeline.json"));
        detail(&game, Some(crate::lcu::frames(&timeline)), "demo-me").unwrap()
    }

    #[test]
    fn lists_only_rows_with_my_participant() {
        let rows = rows();
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].player.champion, "Ahri");
        assert_eq!(rows[0].player.items.len(), 7);
        let history = fixture(include_str!("../fixtures/history.json"));
        assert!(crate::lcu::history(&history, &names()).iter().all(|g| row(g, "otro").is_none()));
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
        let d = lcu_detail();
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
        let d = lcu_detail();
        assert_eq!(d.gold_lead.first().unwrap(), &GoldPoint { minute: 0.0, lead: 0 });
        assert_eq!(d.gold_lead[1], GoldPoint { minute: 5.0, lead: 500 });
        assert_eq!(d.gold_lead.last().unwrap(), &GoldPoint { minute: 30.0, lead: -7000 });
    }

    #[test]
    fn detail_survives_a_missing_timeline() {
        let game = crate::lcu::game(&fixture(include_str!("../fixtures/game.json")), &names()).unwrap();
        let d = detail(&game, None, "demo-me").unwrap();
        assert!(d.gold_lead.is_empty() && d.checkpoints.is_empty() && d.events.is_empty() && d.build.is_empty());
    }

    #[test]
    fn checkpoints_compare_me_with_my_lane_rival() {
        let d = lcu_detail();
        assert_eq!(d.rival.as_deref(), Some("Yasuo"));
        assert_eq!(d.checkpoints.iter().map(|c| c.minute).collect::<Vec<_>>(), [10, 15, 20]);
        let ten = &d.checkpoints[0];
        assert_eq!((ten.mine.gold, ten.mine.cs), (12000, 85));
        assert_eq!(ten.rival.as_ref().unwrap().gold, 13000);
    }

    #[test]
    fn no_rival_when_the_role_is_ambiguous() {
        let mut raw = fixture(include_str!("../fixtures/game.json"));
        raw["participants"][7]["timeline"] = json!({ "lane": "MIDDLE", "role": "DUO" });
        let game = crate::lcu::game(&raw, &names()).unwrap();
        let timeline = fixture(include_str!("../fixtures/timeline.json"));
        let d = detail(&game, Some(crate::lcu::frames(&timeline)), "demo-me").unwrap();
        assert_eq!(d.rival, None);
        assert!(d.checkpoints.iter().all(|c| c.rival.is_none()));
    }

    #[test]
    fn events_attribute_objectives_to_the_right_team() {
        let events = lcu_detail().events;
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
    fn build_drops_undone_purchases_and_skips_evolutions() {
        let frames = vec![json!({ "timestamp": 0, "events": [
            { "type": "ITEM_PURCHASED", "participantId": 1, "itemId": 1056, "timestamp": 20_000 },
            { "type": "ITEM_PURCHASED", "participantId": 2, "itemId": 1055, "timestamp": 20_000 },
            { "type": "ITEM_PURCHASED", "participantId": 1, "itemId": 2003, "timestamp": 21_000 },
            { "type": "ITEM_UNDO", "participantId": 1, "beforeId": 2003, "afterId": 0, "timestamp": 22_000 },
            { "type": "ITEM_PURCHASED", "participantId": 1, "itemId": 3020, "timestamp": 480_000 },
            { "type": "SKILL_LEVEL_UP", "participantId": 1, "skillSlot": 1, "levelUpType": "NORMAL" },
            { "type": "SKILL_LEVEL_UP", "participantId": 1, "skillSlot": 3, "levelUpType": "NORMAL" },
            { "type": "SKILL_LEVEL_UP", "participantId": 1, "skillSlot": 1, "levelUpType": "EVOLVE" }
        ]})];
        let (build, skills) = build_and_skills(&frames, 1);
        assert_eq!(build, [BuildStep { minute: 0.3, item: 1056 }, BuildStep { minute: 8.0, item: 3020 }]);
        assert_eq!(skills, [1, 3]);
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
