#[derive(Debug, Clone)]
pub struct WatchEntry {
    pub id: i64,
    pub anime_id: String,
    pub title: String,
    pub episode: String,
    pub total_episodes: Option<u32>,
    pub watched_at: String,
}
