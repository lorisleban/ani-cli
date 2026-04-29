use rusqlite::Result;

use crate::domain::history::WatchEntry;
use crate::persistence::sqlite_history::Database;

pub trait HistoryStore {
    fn upsert_watch(
        &self,
        anime_id: &str,
        title: &str,
        episode: &str,
        total_episodes: Option<u32>,
    ) -> Result<()>;
    fn get_history(&self) -> Result<Vec<WatchEntry>>;
    fn get_continue_watching(&self) -> Result<Vec<WatchEntry>>;
    fn delete_entry(&self, id: i64) -> Result<()>;
    fn delete_all(&self) -> Result<()>;
}

impl HistoryStore for Database {
    fn upsert_watch(
        &self,
        anime_id: &str,
        title: &str,
        episode: &str,
        total_episodes: Option<u32>,
    ) -> Result<()> {
        Database::upsert_watch(self, anime_id, title, episode, total_episodes)
    }

    fn get_history(&self) -> Result<Vec<WatchEntry>> {
        Database::get_history(self)
    }

    fn get_continue_watching(&self) -> Result<Vec<WatchEntry>> {
        Database::get_continue_watching(self)
    }

    fn delete_entry(&self, id: i64) -> Result<()> {
        Database::delete_entry(self, id)
    }

    fn delete_all(&self) -> Result<()> {
        Database::delete_all(self)
    }
}
