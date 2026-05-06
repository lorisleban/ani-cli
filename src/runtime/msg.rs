use crate::api::AnimeResult;
use crate::app::Screen;
use crate::db::WatchEntry;
use crate::domain::anime::{AnimePresenceMetadata, EpisodeUrl};
use crate::domain::jikan::{JikanAnime, JikanRecommendation};
use crossterm::event::KeyEvent;

#[derive(Debug, Clone)]
pub struct PlaybackData {
    pub url: EpisodeUrl,
    pub entry: WatchEntry,
    pub metadata: Option<AnimePresenceMetadata>,
    pub episodes: Option<Vec<String>>,
}

pub enum Msg {
    // Input
    Key(KeyEvent),
    Tick,

    // Navigation
    Navigate(Screen),
    Back,

    // Home Data
    FetchHomeData,
    HomeAiringLoaded(Result<Vec<JikanAnime>, String>, u64),
    HomeTopLoaded(Result<Vec<JikanAnime>, String>, u64),

    // Search
    PerformSearch(String),
    SearchLoaded(Result<Vec<AnimeResult>, String>, u64),

    // Detail
    FetchDetail(String),
    EpisodesLoaded(Result<(Vec<String>, String), String>, u64), // (episodes, resolved_provider_id)
    MetadataLoaded(Box<Option<JikanAnime>>, u64),
    ImageLoaded(Option<image::DynamicImage>, u64),
    RecommendationsLoaded(Result<Vec<JikanRecommendation>, String>, u64),

    // Player
    PlayEpisode(WatchEntry),
    StreamReady(Box<PlaybackData>),
    PlayerFinished,

    // UI
    Toast(String, bool), // message, is_error
    Quit,
    RefreshHistory,
    ToggleUpdatePopup(bool),
    TriggerUpdate,
    UpdateStatus(Result<crate::update::UpdateOutcome, String>),
    Batch(Vec<Msg>),
}

pub enum Cmd {
    None,
    FetchAiringToday(u64),
    FetchTopAiring(u64),
    SearchAnime(String, u64),
    FetchAnimeDetail {
        provider_id: Option<String>,
        title: String,
        session_id: u64,
    },
    FetchImage(String, u64),
    FetchRecommendations(u32, u64), // mal_id
    LaunchPlayer(WatchEntry),
    ExecutePlayer(Box<PlaybackData>),
    PerformUpdate,
    Batch(Vec<Cmd>),
}
