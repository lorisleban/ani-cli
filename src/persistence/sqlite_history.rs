use std::path::PathBuf;

use rusqlite::{params, Connection, Result};

use crate::domain::history::WatchEntry;

const USER_VERSION: i64 = 1;
const STOP_REWIND_SECONDS: i64 = 30;

pub struct Database {
    conn: Connection,
}

pub struct NewWatchSession<'a> {
    pub anime_id: &'a str,
    pub title: &'a str,
    pub episode: &'a str,
    pub total_episodes: Option<u32>,
    pub player: &'a str,
    pub mode: &'a str,
    pub quality: &'a str,
}

struct WatchEventInput<'a> {
    show_id: i64,
    episode_id: Option<i64>,
    event_type: &'a str,
    player: &'a str,
    mode: &'a str,
    quality: &'a str,
    session_id: Option<i64>,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = Self::resolve_db_path()?;
        let conn = Connection::open(&db_path)?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS watch_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                anime_id TEXT NOT NULL,
                title TEXT NOT NULL,
                episode TEXT NOT NULL,
                total_episodes INTEGER,
                watched_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(anime_id, episode)
            );

            CREATE TABLE IF NOT EXISTS shows (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider TEXT NOT NULL DEFAULT 'allanime',
                provider_show_id TEXT NOT NULL,
                title TEXT NOT NULL,
                total_episodes INTEGER,
                mal_id INTEGER,
                image_url TEXT,
                metadata_updated_at DATETIME,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(provider, provider_show_id)
            );

            CREATE TABLE IF NOT EXISTS episodes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                show_id INTEGER NOT NULL REFERENCES shows(id) ON DELETE CASCADE,
                episode TEXT NOT NULL,
                episode_number REAL,
                provider_episode_id TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(show_id, episode)
            );

            CREATE TABLE IF NOT EXISTS watch_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                show_id INTEGER NOT NULL REFERENCES shows(id) ON DELETE CASCADE,
                episode_id INTEGER REFERENCES episodes(id) ON DELETE SET NULL,
                event_type TEXT NOT NULL,
                event_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                player TEXT,
                mode TEXT,
                quality TEXT,
                session_id INTEGER,
                sync_dirty INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS watch_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                show_id INTEGER NOT NULL REFERENCES shows(id) ON DELETE CASCADE,
                episode_id INTEGER NOT NULL REFERENCES episodes(id) ON DELETE CASCADE,
                started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                stopped_at DATETIME,
                stop_rewind_seconds INTEGER NOT NULL DEFAULT 30,
                player TEXT,
                mode TEXT,
                quality TEXT,
                sync_dirty INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS watch_state (
                show_id INTEGER PRIMARY KEY REFERENCES shows(id) ON DELETE CASCADE,
                episode_id INTEGER REFERENCES episodes(id) ON DELETE SET NULL,
                status TEXT NOT NULL DEFAULT 'watching',
                last_started_at DATETIME,
                last_stopped_at DATETIME,
                last_watched_at DATETIME,
                watch_count INTEGER NOT NULL DEFAULT 0,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                sync_dirty INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS library_items (
                show_id INTEGER PRIMARY KEY REFERENCES shows(id) ON DELETE CASCADE,
                status TEXT NOT NULL DEFAULT 'watching',
                favorite INTEGER NOT NULL DEFAULT 0,
                rating INTEGER,
                notes TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                sync_dirty INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS metadata_cache (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL,
                external_id TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                fetched_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                expires_at DATETIME,
                UNIQUE(source, external_id)
            );

            CREATE TABLE IF NOT EXISTS sync_records (
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                remote_id TEXT,
                dirty INTEGER NOT NULL DEFAULT 1,
                deleted INTEGER NOT NULL DEFAULT 0,
                local_updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                remote_updated_at DATETIME,
                version INTEGER NOT NULL DEFAULT 1,
                PRIMARY KEY(entity_type, entity_id)
            );",
        )?;
        self.backfill_local_data()?;
        self.conn
            .pragma_update(None, "user_version", USER_VERSION)?;
        Ok(())
    }

    fn backfill_local_data(&self) -> Result<()> {
        let history = self.get_history()?;
        for entry in history {
            let show_id = self.upsert_show(&entry.anime_id, &entry.title, entry.total_episodes)?;
            let episode_id = self.upsert_episode(show_id, &entry.episode)?;
            self.conn.execute(
                "INSERT INTO watch_state (
                    show_id, episode_id, status, last_watched_at, watch_count, updated_at, sync_dirty
                 )
                 VALUES (?1, ?2, 'watching', ?3, 1, ?3, 0)
                 ON CONFLICT(show_id) DO UPDATE SET
                    episode_id = excluded.episode_id,
                    last_watched_at = CASE
                        WHEN watch_state.last_watched_at IS NULL
                            OR excluded.last_watched_at > watch_state.last_watched_at
                        THEN excluded.last_watched_at
                        ELSE watch_state.last_watched_at
                    END,
                    updated_at = CASE
                        WHEN excluded.updated_at > watch_state.updated_at
                        THEN excluded.updated_at
                        ELSE watch_state.updated_at
                    END",
                params![show_id, episode_id, entry.watched_at],
            )?;
        }
        Ok(())
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

    pub fn start_watch_session(&self, input: NewWatchSession<'_>) -> Result<i64> {
        self.upsert_watch(
            input.anime_id,
            input.title,
            input.episode,
            input.total_episodes,
        )?;
        let show_id = self.upsert_show(input.anime_id, input.title, input.total_episodes)?;
        let episode_id = self.upsert_episode(show_id, input.episode)?;

        self.conn.execute(
            "INSERT INTO watch_sessions (
                show_id, episode_id, started_at, player, mode, quality, stop_rewind_seconds
             )
             VALUES (?1, ?2, CURRENT_TIMESTAMP, ?3, ?4, ?5, ?6)",
            params![
                show_id,
                episode_id,
                input.player,
                input.mode,
                input.quality,
                STOP_REWIND_SECONDS
            ],
        )?;
        let session_id = self.conn.last_insert_rowid();

        self.insert_watch_event(WatchEventInput {
            show_id,
            episode_id: Some(episode_id),
            event_type: "started",
            player: input.player,
            mode: input.mode,
            quality: input.quality,
            session_id: Some(session_id),
        })?;
        self.conn.execute(
            "INSERT INTO watch_state (
                show_id, episode_id, status, last_started_at, last_watched_at, watch_count, updated_at
             )
             VALUES (?1, ?2, 'watching', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 1, CURRENT_TIMESTAMP)
             ON CONFLICT(show_id) DO UPDATE SET
                episode_id = excluded.episode_id,
                status = 'watching',
                last_started_at = CURRENT_TIMESTAMP,
                last_watched_at = CURRENT_TIMESTAMP,
                watch_count = watch_state.watch_count + 1,
                updated_at = CURRENT_TIMESTAMP,
                sync_dirty = 1",
            params![show_id, episode_id],
        )?;
        self.mark_sync_dirty("show", show_id)?;
        self.mark_sync_dirty("episode", episode_id)?;
        self.mark_sync_dirty("watch_state", show_id)?;
        self.mark_sync_dirty("watch_session", session_id)?;

        Ok(session_id)
    }

    pub fn stop_watch_session(&self, session_id: i64) -> Result<()> {
        let changed = self.conn.execute(
            "UPDATE watch_sessions
             SET stopped_at = CASE
                    WHEN datetime('now', '-' || stop_rewind_seconds || ' seconds') < started_at
                    THEN started_at
                    ELSE datetime('now', '-' || stop_rewind_seconds || ' seconds')
                 END,
                 sync_dirty = 1
             WHERE id = ?1 AND stopped_at IS NULL",
            params![session_id],
        )?;
        if changed == 0 {
            return Ok(());
        }

        let session = self.conn.query_row(
            "SELECT show_id, episode_id, player, mode, quality, stopped_at
             FROM watch_sessions
             WHERE id = ?1",
            params![session_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            },
        )?;

        let (show_id, episode_id, player, mode, quality, stopped_at) = session;
        if let Some(stopped_at) = stopped_at {
            self.conn.execute(
                "INSERT INTO watch_events (
                    show_id, episode_id, event_type, event_at, player, mode, quality, session_id
                 )
                 VALUES (?1, ?2, 'stopped', ?3, ?4, ?5, ?6, ?7)",
                params![show_id, episode_id, stopped_at, player, mode, quality, session_id],
            )?;
            let event_id = self.conn.last_insert_rowid();
            self.conn.execute(
                "UPDATE watch_state
                 SET last_stopped_at = ?2,
                     updated_at = CURRENT_TIMESTAMP,
                     sync_dirty = 1
                 WHERE show_id = ?1",
                params![show_id, stopped_at],
            )?;
            self.mark_sync_dirty("watch_event", event_id)?;
            self.mark_sync_dirty("watch_state", show_id)?;
            self.mark_sync_dirty("watch_session", session_id)?;
        }
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
        self.conn.execute("DELETE FROM watch_events", [])?;
        self.conn.execute("DELETE FROM watch_sessions", [])?;
        self.conn.execute("DELETE FROM watch_state", [])?;
        Ok(())
    }

    fn upsert_show(&self, anime_id: &str, title: &str, total_episodes: Option<u32>) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO shows (provider, provider_show_id, title, total_episodes)
             VALUES ('allanime', ?1, ?2, ?3)
             ON CONFLICT(provider, provider_show_id) DO UPDATE SET
                title = excluded.title,
                total_episodes = excluded.total_episodes,
                updated_at = CURRENT_TIMESTAMP",
            params![anime_id, title, total_episodes],
        )?;
        self.conn.query_row(
            "SELECT id FROM shows WHERE provider = 'allanime' AND provider_show_id = ?1",
            params![anime_id],
            |row| row.get(0),
        )
    }

    fn upsert_episode(&self, show_id: i64, episode: &str) -> Result<i64> {
        let episode_number = episode.parse::<f64>().ok();
        self.conn.execute(
            "INSERT INTO episodes (show_id, episode, episode_number)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(show_id, episode) DO UPDATE SET
                episode_number = excluded.episode_number,
                updated_at = CURRENT_TIMESTAMP",
            params![show_id, episode, episode_number],
        )?;
        self.conn.query_row(
            "SELECT id FROM episodes WHERE show_id = ?1 AND episode = ?2",
            params![show_id, episode],
            |row| row.get(0),
        )
    }

    fn insert_watch_event(&self, input: WatchEventInput<'_>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO watch_events (
                show_id, episode_id, event_type, player, mode, quality, session_id
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                input.show_id,
                input.episode_id,
                input.event_type,
                input.player,
                input.mode,
                input.quality,
                input.session_id
            ],
        )?;
        let event_id = self.conn.last_insert_rowid();
        self.mark_sync_dirty("watch_event", event_id)
    }

    fn mark_sync_dirty(&self, entity_type: &str, entity_id: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sync_records (
                entity_type, entity_id, dirty, deleted, local_updated_at, version
             )
             VALUES (?1, ?2, 1, 0, CURRENT_TIMESTAMP, 1)
             ON CONFLICT(entity_type, entity_id) DO UPDATE SET
                dirty = 1,
                deleted = 0,
                local_updated_at = CURRENT_TIMESTAMP,
                version = version + 1",
            params![entity_type, entity_id.to_string()],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Database, NewWatchSession};
    use rusqlite::Connection;

    impl Database {
        fn in_memory() -> rusqlite::Result<Self> {
            let db = Self {
                conn: Connection::open_in_memory()?,
            };
            db.migrate()?;
            Ok(db)
        }
    }

    #[test]
    fn records_watch_session_state_and_rewound_stop() {
        let db = Database::in_memory().expect("database");

        let session_id = db
            .start_watch_session(NewWatchSession {
                anime_id: "show-1",
                title: "Test Show",
                episode: "1",
                total_episodes: Some(12),
                player: "mpv",
                mode: "sub",
                quality: "best",
            })
            .expect("start session");
        db.stop_watch_session(session_id).expect("stop session");

        let stopped_at: Option<String> = db
            .conn
            .query_row(
                "SELECT stopped_at FROM watch_sessions WHERE id = ?1",
                [session_id],
                |row| row.get(0),
            )
            .expect("session stopped_at");
        let stopped_events: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM watch_events WHERE event_type = 'stopped'",
                [],
                |row| row.get(0),
            )
            .expect("stopped event count");
        let watch_count: i64 = db
            .conn
            .query_row("SELECT watch_count FROM watch_state", [], |row| row.get(0))
            .expect("watch state");

        assert!(stopped_at.is_some());
        assert_eq!(stopped_events, 1);
        assert_eq!(watch_count, 1);
    }

    #[test]
    fn stopping_session_twice_does_not_duplicate_stop_event() {
        let db = Database::in_memory().expect("database");
        let session_id = db
            .start_watch_session(NewWatchSession {
                anime_id: "show-1",
                title: "Test Show",
                episode: "1",
                total_episodes: Some(12),
                player: "mpv",
                mode: "sub",
                quality: "best",
            })
            .expect("start session");

        db.stop_watch_session(session_id).expect("first stop");
        db.stop_watch_session(session_id).expect("second stop");

        let stopped_events: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM watch_events WHERE event_type = 'stopped'",
                [],
                |row| row.get(0),
            )
            .expect("stopped event count");

        assert_eq!(stopped_events, 1);
    }
}
