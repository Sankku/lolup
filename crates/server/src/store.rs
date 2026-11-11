//! Persistencia del servidor en SQLite.
//!
//! ponytail: un solo archivo y un solo proceso. Pasar a Postgres cuando haga falta más de una instancia.

use lolup_core::lp::Snapshot;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use std::sync::Mutex;

pub struct Store(Mutex<Connection>);

impl Store {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self::init(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn in_memory() -> Self {
        Self::init(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn init(connection: Connection) -> rusqlite::Result<Self> {
        connection.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS riot_cache (
                 key TEXT PRIMARY KEY,
                 body TEXT NOT NULL,
                 fetched_ms INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS lp_snapshots (
                 puuid TEXT NOT NULL,
                 queue TEXT NOT NULL,
                 ms INTEGER NOT NULL,
                 tier TEXT NOT NULL,
                 division TEXT NOT NULL,
                 lp INTEGER NOT NULL,
                 PRIMARY KEY (puuid, queue, ms)
             );
             CREATE TABLE IF NOT EXISTS sessions (
                 token_hash TEXT PRIMARY KEY,
                 sub TEXT NOT NULL,
                 name TEXT NOT NULL,
                 expires_ms INTEGER NOT NULL
             );",
        )?;
        Ok(Self(Mutex::new(connection)))
    }

    /// Las partidas y líneas de tiempo no cambian: se guardan una vez y no vencen.
    pub fn cached(&self, key: &str) -> Option<String> {
        let db = self.0.lock().unwrap();
        db.query_row("SELECT body FROM riot_cache WHERE key = ?1", [key], |row| row.get(0)).optional().ok().flatten()
    }

    pub fn put(&self, key: &str, body: &str, ms: i64) {
        let db = self.0.lock().unwrap();
        // Perder la caché solo cuesta otra consulta a Riot, no datos.
        let _ = db.execute(
            "INSERT OR REPLACE INTO riot_cache (key, body, fetched_ms) VALUES (?1, ?2, ?3)",
            params![key, body, ms],
        );
    }

    pub fn lp_history(&self, puuid: &str) -> Vec<Snapshot> {
        let db = self.0.lock().unwrap();
        let Ok(mut query) =
            db.prepare("SELECT ms, queue, tier, division, lp FROM lp_snapshots WHERE puuid = ?1 ORDER BY ms")
        else {
            return Vec::new();
        };
        query
            .query_map([puuid], |row| {
                Ok(Snapshot { ms: row.get(0)?, queue: row.get(1)?, tier: row.get(2)?, division: row.get(3)?, lp: row.get(4)? })
            })
            .map(|rows| rows.filter_map(Result::ok).collect())
            .unwrap_or_default()
    }

    pub fn add_lp(&self, puuid: &str, snapshots: &[Snapshot]) -> rusqlite::Result<()> {
        let db = self.0.lock().unwrap();
        for s in snapshots {
            db.execute(
                "INSERT OR IGNORE INTO lp_snapshots (puuid, queue, ms, tier, division, lp) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![puuid, s.queue, s.ms, s.tier, s.division, s.lp],
            )?;
        }
        Ok(())
    }

    /// Guarda una sesión por el hash de su token: el token en sí solo vive en la cookie.
    pub fn add_session(&self, token_hash: &str, sub: &str, name: &str, expires_ms: i64) -> rusqlite::Result<()> {
        let db = self.0.lock().unwrap();
        db.execute(
            "INSERT OR REPLACE INTO sessions (token_hash, sub, name, expires_ms) VALUES (?1, ?2, ?3, ?4)",
            params![token_hash, sub, name, expires_ms],
        )?;
        Ok(())
    }

    /// Devuelve `(sub, name)` si la sesión existe y no venció. Aprovecha para borrar las vencidas.
    pub fn session(&self, token_hash: &str, now_ms: i64) -> Option<(String, String)> {
        let db = self.0.lock().unwrap();
        let _ = db.execute("DELETE FROM sessions WHERE expires_ms <= ?1", [now_ms]);
        db.query_row("SELECT sub, name FROM sessions WHERE token_hash = ?1", [token_hash], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()
            .ok()
            .flatten()
    }

    pub fn remove_session(&self, token_hash: &str) {
        let db = self.0.lock().unwrap();
        let _ = db.execute("DELETE FROM sessions WHERE token_hash = ?1", [token_hash]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(ms: i64, lp: i64) -> Snapshot {
        Snapshot { ms, queue: "SOLO".into(), tier: "GOLD".into(), division: "II".into(), lp }
    }

    #[test]
    fn caches_riot_responses() {
        let store = Store::in_memory();
        assert_eq!(store.cached("match:LA2_1"), None);
        store.put("match:LA2_1", "{\"a\":1}", 5);
        assert_eq!(store.cached("match:LA2_1").as_deref(), Some("{\"a\":1}"));
    }

    #[test]
    fn keeps_lp_history_per_player_in_order() {
        let store = Store::in_memory();
        store.add_lp("b", &[snapshot(2, 50), snapshot(1, 40)]).unwrap();
        store.add_lp("other", &[snapshot(1, 99)]).unwrap();
        store.add_lp("b", &[snapshot(2, 50)]).unwrap();
        let history = store.lp_history("b");
        assert_eq!(history.iter().map(|s| (s.ms, s.lp)).collect::<Vec<_>>(), [(1, 40), (2, 50)]);
    }

    #[test]
    fn sessions_expire() {
        let store = Store::in_memory();
        store.add_session("h", "sub1", "Jugador", 100).unwrap();
        assert_eq!(store.session("h", 50), Some(("sub1".into(), "Jugador".into())));
        assert_eq!(store.session("otro", 50), None);
        assert_eq!(store.session("h", 100), None);
        store.add_session("h2", "sub1", "Jugador", 100).unwrap();
        store.remove_session("h2");
        assert_eq!(store.session("h2", 50), None);
    }

    #[test]
    fn survives_reopening_the_file() {
        let path = std::env::temp_dir().join(format!("lolup-store-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        Store::open(&path).unwrap().add_lp("p", &[snapshot(1, 10)]).unwrap();
        assert_eq!(Store::open(&path).unwrap().lp_history("p").len(), 1);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
    }
}
