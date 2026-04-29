use rusqlite::{params, Connection, Result};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct WatchEntry {
    pub id: i64,
    pub anime_id: String,
    pub title: String,
    pub episode: String,
    pub total_episodes: Option<u32>,
    pub watched_at: String,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = Self::resolve_db_path()?;
        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS watch_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                anime_id TEXT NOT NULL,
                title TEXT NOT NULL,
                episode TEXT NOT NULL,
                total_episodes INTEGER,
                watched_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(anime_id, episode)
            );",
        )?;
        Ok(Self { conn })
    }

    fn resolve_db_path() -> Result<PathBuf> {
        for path in Self::candidate_paths() {
            if let Some(parent) = path.parent() {
                if std::fs::create_dir_all(parent).is_err() {
                    continue;
                }
            }

            if Connection::open(&path).is_ok() {
                return Ok(path);
            }
        }

        Err(rusqlite::Error::InvalidPath(
            "unable to create or open any database path".into(),
        ))
    }

    fn candidate_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        if let Some(data_dir) = dirs::data_dir() {
            paths.push(data_dir.join("ani-cli").join("history.db"));
        }

        if let Some(local_data_dir) = dirs::data_local_dir() {
            let candidate = local_data_dir.join("ani-cli").join("history.db");
            if !paths.contains(&candidate) {
                paths.push(candidate);
            }
        }

        paths.push(PathBuf::from(".").join(".ani-cli").join("history.db"));
        paths
    }

    pub fn upsert_watch(
        &self,
        anime_id: &str,
        title: &str,
        episode: &str,
        total_episodes: Option<u32>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO watch_history (anime_id, title, episode, total_episodes, watched_at)
             VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)
             ON CONFLICT(anime_id, episode) DO UPDATE SET
                title = excluded.title,
                total_episodes = excluded.total_episodes,
                watched_at = CURRENT_TIMESTAMP",
            params![anime_id, title, episode, total_episodes],
        )?;
        Ok(())
    }

    pub fn get_history(&self) -> Result<Vec<WatchEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, anime_id, title, episode, total_episodes, watched_at
             FROM watch_history
             ORDER BY watched_at DESC",
        )?;
        let entries = stmt
            .query_map([], |row| {
                Ok(WatchEntry {
                    id: row.get(0)?,
                    anime_id: row.get(1)?,
                    title: row.get(2)?,
                    episode: row.get(3)?,
                    total_episodes: row.get(4)?,
                    watched_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(entries)
    }

    pub fn get_continue_watching(&self) -> Result<Vec<WatchEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, anime_id, title, episode, total_episodes, watched_at
             FROM watch_history
             WHERE id IN (
                SELECT MAX(id) FROM watch_history GROUP BY anime_id
             )
             ORDER BY watched_at DESC",
        )?;
        let entries = stmt
            .query_map([], |row| {
                Ok(WatchEntry {
                    id: row.get(0)?,
                    anime_id: row.get(1)?,
                    title: row.get(2)?,
                    episode: row.get(3)?,
                    total_episodes: row.get(4)?,
                    watched_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(entries)
    }

    pub fn delete_entry(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM watch_history WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn delete_all(&self) -> Result<()> {
        self.conn.execute("DELETE FROM watch_history", [])?;
        Ok(())
    }
}
