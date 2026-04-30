#[derive(Debug, Clone)]
pub struct WatchEntry {
    pub id: i64,
    pub anime_id: String,
    pub title: String,
    pub episode: String,
    pub total_episodes: Option<u32>,
    pub watched_at: String,
}

#[derive(Debug, Clone)]
pub struct WatchSession {
    pub id: i64,
    pub anime_id: String,
    pub episode: String,
    pub started_at: String,
    pub stopped_at: Option<String>,
}
