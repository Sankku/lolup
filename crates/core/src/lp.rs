//! Evolución de LP. Riot solo informa el valor actual, así que cada versión guarda sus registros.

use crate::game::Rank;
use serde::{Deserialize, Serialize};

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

/// Registros nuevos: solo las colas cuyo rango cambió desde el último guardado.
pub fn changes(history: &[Snapshot], ms: i64, ranks: &[(&str, Option<&Rank>)]) -> Vec<Snapshot> {
    ranks
        .iter()
        .filter_map(|(queue, rank)| {
            let rank = rank.as_ref()?;
            let last = history.iter().rev().find(|s| s.queue == *queue);
            if last.is_some_and(|s| s.tier == rank.tier && s.division == rank.division && s.lp == rank.lp) {
                return None;
            }
            Some(Snapshot {
                ms,
                queue: queue.to_string(),
                tier: rank.tier.clone(),
                division: rank.division.clone(),
                lp: rank.lp,
            })
        })
        .collect()
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

    #[test]
    fn records_only_queues_that_changed() {
        let mut history = changes(&[], 1, &[(SOLO, Some(&rank("GOLD", "II", 40))), (FLEX, None)]);
        assert_eq!(history.len(), 1);
        assert!(changes(&history, 2, &[(SOLO, Some(&rank("GOLD", "II", 40)))]).is_empty());
        history.extend(changes(&history, 3, &[(SOLO, Some(&rank("GOLD", "II", 58))), (FLEX, Some(&rank("GOLD", "I", 5)))]));
        assert_eq!(history.iter().map(|s| (s.ms, s.queue.as_str())).collect::<Vec<_>>(), [(1, SOLO), (3, SOLO), (3, FLEX)]);
        assert_eq!(points(&history, SOLO).len(), 2);
        assert_eq!(points(&history, FLEX).len(), 1);
    }

    #[test]
    fn ladder_orders_divisions_tiers_and_apex() {
        let value = |tier: &str, division: &str, lp| {
            ladder(&Snapshot { ms: 0, queue: SOLO.into(), tier: tier.into(), division: division.into(), lp })
        };
        assert!(value("GOLD", "IV", 99) < value("GOLD", "III", 0));
        assert!(value("GOLD", "I", 99) < value("PLATINUM", "IV", 0));
        assert!(value("DIAMOND", "I", 99) < value("MASTER", "", 0));
        assert_eq!(value("GRANDMASTER", "", 874), value("MASTER", "", 874));
    }
}
