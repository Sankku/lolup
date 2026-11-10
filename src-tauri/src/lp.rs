//! Evolución de LP: el cliente solo informa el valor actual, así que se guarda localmente.

use crate::history::Rank;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub const SOLO: &str = "SOLO";
pub const FLEX: &str = "FLEX";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub ms: i64,
    pub queue: String,
    pub tier: String,
    pub division: String,
    pub lp: i64,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Point {
    pub ms: i64,
    pub tier: String,
    pub division: String,
    pub lp: i64,
    pub ladder: i64,
}

/// Posición en la escala oficial de LP, solo para ordenar el eje del gráfico. No estima habilidad.
fn ladder(snapshot: &Snapshot) -> i64 {
    const TIERS: [&str; 7] = ["IRON", "BRONZE", "SILVER", "GOLD", "PLATINUM", "EMERALD", "DIAMOND"];
    const DIVISIONS: [&str; 4] = ["IV", "III", "II", "I"];
    let Some(tier) = TIERS.iter().position(|t| *t == snapshot.tier) else {
        // Maestro, Gran Maestro y Retador comparten una sola escala de LP.
        return TIERS.len() as i64 * 400 + snapshot.lp;
    };
    let division = DIVISIONS.iter().position(|d| *d == snapshot.division).unwrap_or(0);
    tier as i64 * 400 + division as i64 * 100 + snapshot.lp
}

/// Una línea dañada no debe borrar el resto del historial.
pub fn load(path: &Path) -> Vec<Snapshot> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// Agrega las colas que cambiaron y devuelve el historial completo.
pub fn record(path: &Path, ms: i64, ranks: &[(&str, Option<&Rank>)]) -> Vec<Snapshot> {
    let mut history = load(path);
    let mut changed = Vec::new();
    for (queue, rank) in ranks {
        let Some(rank) = rank else { continue };
        let last = history.iter().rev().find(|s| s.queue == *queue);
        if last.is_some_and(|s| s.tier == rank.tier && s.division == rank.division && s.lp == rank.lp) {
            continue;
        }
        changed.push(Snapshot {
            ms,
            queue: queue.to_string(),
            tier: rank.tier.clone(),
            division: rank.division.clone(),
            lp: rank.lp,
        });
    }
    if !changed.is_empty() {
        // Si no se puede escribir se muestra igual lo ya guardado.
        let _ = append(path, &changed);
        history.extend(changed);
    }
    history
}

fn append(path: &Path, snapshots: &[Snapshot]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    for snapshot in snapshots {
        writeln!(file, "{}", serde_json::to_string(snapshot)?)?;
    }
    Ok(())
}

pub fn points(history: &[Snapshot], queue: &str) -> Vec<Point> {
    history
        .iter()
        .filter(|s| s.queue == queue)
        .map(|s| Point {
            ms: s.ms,
            tier: s.tier.clone(),
            division: s.division.clone(),
            lp: s.lp,
            ladder: ladder(s),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rank(tier: &str, division: &str, lp: i64) -> Rank {
        Rank { tier: tier.into(), division: division.into(), lp, wins: 1, losses: 1 }
    }

    fn temp_file(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("lolup-{name}-{}.jsonl", std::process::id()));
        let _ = fs::remove_file(&path);
        path
    }

    #[test]
    fn records_only_when_the_rank_changes() {
        let path = temp_file("changes");
        record(&path, 1, &[(SOLO, Some(&rank("GOLD", "II", 40))), (FLEX, None)]);
        record(&path, 2, &[(SOLO, Some(&rank("GOLD", "II", 40)))]);
        let history = record(&path, 3, &[(SOLO, Some(&rank("GOLD", "II", 58)))]);
        assert_eq!(history.iter().map(|s| s.ms).collect::<Vec<_>>(), [1, 3]);
        assert_eq!(load(&path), history);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn queues_are_tracked_separately() {
        let path = temp_file("queues");
        record(&path, 1, &[(SOLO, Some(&rank("GOLD", "II", 40))), (FLEX, Some(&rank("GOLD", "II", 40)))]);
        let history = record(&path, 2, &[(FLEX, Some(&rank("GOLD", "II", 50))), (SOLO, Some(&rank("GOLD", "II", 40)))]);
        assert_eq!(points(&history, SOLO).len(), 1);
        assert_eq!(points(&history, FLEX).len(), 2);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_damaged_line_keeps_the_rest() {
        let path = temp_file("damaged");
        fs::write(&path, "{\"ms\":1,\"queue\":\"SOLO\",\"tier\":\"GOLD\",\"division\":\"I\",\"lp\":3}\nbasura\n").unwrap();
        assert_eq!(load(&path).len(), 1);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn missing_file_is_an_empty_history() {
        assert!(load(Path::new("/no/existe/lp.jsonl")).is_empty());
    }

    #[test]
    fn ladder_orders_divisions_tiers_and_apex() {
        let value = |tier: &str, division: &str, lp| {
            let s = Snapshot { ms: 0, queue: SOLO.into(), tier: tier.into(), division: division.into(), lp };
            ladder(&s)
        };
        assert!(value("GOLD", "IV", 99) < value("GOLD", "III", 0));
        assert!(value("GOLD", "I", 99) < value("PLATINUM", "IV", 0));
        assert!(value("DIAMOND", "I", 99) < value("MASTER", "", 0));
        assert_eq!(value("GRANDMASTER", "", 874), value("MASTER", "", 874));
    }
}
