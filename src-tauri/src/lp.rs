//! En escritorio la evolución de LP se guarda en un archivo local por cuenta.

use lolup_core::game::Rank;
pub use lolup_core::lp::{FLEX, Point, SOLO, Snapshot, points};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

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
    let changed = lolup_core::lp::changes(&history, ms, ranks);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rank(lp: i64) -> Rank {
        Rank { tier: "GOLD".into(), division: "II".into(), lp, wins: 1, losses: 1 }
    }

    fn temp_file(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("lolup-{name}-{}.jsonl", std::process::id()));
        let _ = fs::remove_file(&path);
        path
    }

    #[test]
    fn persists_only_changes() {
        let path = temp_file("changes");
        record(&path, 1, &[(SOLO, Some(&rank(40))), (FLEX, None)]);
        record(&path, 2, &[(SOLO, Some(&rank(40)))]);
        let history = record(&path, 3, &[(SOLO, Some(&rank(58)))]);
        assert_eq!(history.iter().map(|s| s.ms).collect::<Vec<_>>(), [1, 3]);
        assert_eq!(load(&path), history);
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
}
