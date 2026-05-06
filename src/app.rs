use std::time::Instant;

use crate::api::{AnimeResult, EpisodeUrl, Mode};
use crate::db::{Database, NewWatchSession, WatchEntry};
use crate::discord::{
    session_started_at_unix, DiscordPresence, PlayerActivityMonitor, PresencePlayback,
};
use crate::domain::anime::AnimePresenceMetadata;
use crate::domain::jikan::JikanAnime;
use crate::domain::jikan::JikanGenre;
use crate::domain::jikan::JikanRecommendation;
use crate::domain::jikan::JikanSeasonInfo;
use crate::player::{self, PlayerType};
use crate::providers::jikan::JikanClient;
use crate::theme::Theme;

#[derive(Debug, Clone, Default)]
pub struct AppOptions {
    pub player_type: Option<PlayerType>,
    pub mode: Option<Mode>,
    pub discord_client_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Home,
    Search,
    AnimeDetail,
    WatchHistory,
    NowPlaying,
    Help,
    SeasonBrowse,
    Schedule,
    TopAnime,
    GenreBrowse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeFocus {
    Queue,
    Airing,
    Trending,
}

pub struct Toast {
    pub message: String,
    pub is_error: bool,
    pub born: Instant,
}

pub struct App {
    pub screen: Screen,
    pub screen_stack: Vec<Screen>,
    pub running: bool,

    pub theme: Theme,

    pub home_selected: usize,
    pub home_focus: HomeFocus,
    pub home_airing_selected: usize,
    pub home_airing_offset: usize,

    // Search
    pub search_input: String,
    pub search_results: Vec<AnimeResult>,
    pub search_selected: usize,
    pub search_loading: bool,
    pub search_dirty: bool,
    pub search_debounce_deadline: Option<Instant>,

    // Detail
    pub selected_anime: Option<AnimeResult>,
    pub episodes: Vec<String>,
    pub episode_selected: usize,
    pub episodes_loading: bool,
    pub jikan_anime: Option<JikanAnime>,
    pub jikan_loading: bool,
    pub synopsis_scroll: usize,
    pub cover_art: Option<crate::ui::cover_image::CoverArt>,
    pub cover_art_loading: bool,
    pub image_picker: Option<ratatui_image::picker::Picker>,

    // Now playing
    pub current_episode: Option<String>,
    pub playing_title: Option<String>,
    pub episode_url: Option<EpisodeUrl>,
    pub active_watch_session_id: Option<i64>,
    pub active_presence_token: Option<u64>,
    pub active_presence_metadata: Option<AnimePresenceMetadata>,

    // History
    pub history: Vec<WatchEntry>,
    pub history_selected: usize,
    pub continue_watching: Vec<WatchEntry>,

    // Status
    pub toasts: Vec<Toast>,
    pub spinner_tick: usize,
    pub splash_tick: usize,
    pub loading: bool,
    pub target_episode: Option<String>,

    // Vim-style key buffer (e.g. `g`+`h`)
    pub key_seq: Option<(char, Instant)>,

    // Player / mode
    pub player_type: PlayerType,
    pub mode: Mode,
    pub quality: String,
    pub discord_presence: Option<DiscordPresence>,

    pub db: Database,
    pub jikan: JikanClient,

    // Season browse
    pub season_anime: Vec<JikanAnime>,
    pub season_selected: usize,
    pub season_loading: bool,
    pub season_year: Option<i32>,
    pub season_name: Option<String>,
    pub season_page: u32,
    pub season_has_next: bool,
    pub season_list: Vec<JikanSeasonInfo>,
    pub season_filter_type: Option<String>,

    // Schedule
    pub schedule_anime: Vec<JikanAnime>,
    pub schedule_selected: usize,
    pub schedule_loading: bool,
    pub schedule_day: String,

    // Home enrichment
    pub airing_today: Vec<JikanAnime>,
    pub airing_today_loading: bool,
    pub airing_today_last_fetch: Option<Instant>,
    pub home_season_label: Option<String>,
    pub home_season_count: Option<usize>,
    pub home_season_last_fetch: Option<Instant>,

    // Top anime
    pub top_anime: Vec<JikanAnime>,
    pub top_selected: usize,
    pub top_loading: bool,
    pub top_page: u32,
    pub top_has_next: bool,
    pub top_filter_type: Option<String>,
    pub top_filter_rating: Option<String>,
    pub top_filter_sfw: bool,

    // Genre browse
    pub genres: Vec<JikanGenre>,
    pub genre_selected: usize,
    pub genre_loading: bool,
    pub genre_picked: Option<JikanGenre>,
    pub genre_anime: Vec<JikanAnime>,
    pub genre_anime_selected: usize,
    pub genre_anime_loading: bool,
    pub genre_anime_page: u32,
    pub genre_anime_has_next: bool,

    // Recommendations (on detail screen)
    pub recommendations: Vec<JikanRecommendation>,
    pub recommendations_loading: bool,
    pub recommendations_selected: usize,
    pub show_recommendations: bool,

    // Update status
    pub update_available: Option<crate::update::UpdateInfo>,
    pub update_check_in_progress: bool,
    pub update_popup_visible: bool,
    pub update_requested: bool,
    pub update_in_progress: bool,
    pub update_check_manual: bool,
    pub update_notes_requested: bool,

    // Concurrency control
    pub current_session_id: u64,
}

impl App {
    pub fn new() -> Self {
        Self::with_options(AppOptions::default())
    }

    pub fn with_options(options: AppOptions) -> Self {
        Self::with_options_and_picker(options, None)
    }

    pub fn with_options_and_picker(
        options: AppOptions,
        picker: Option<ratatui_image::picker::Picker>,
    ) -> Self {
        let db = Database::new().expect("Failed to initialize database");
        let history = db.get_history().unwrap_or_default();
        let continue_watching = db.get_continue_watching().unwrap_or_default();
        let jikan = JikanClient::new(Database::new().expect("Failed to initialize Jikan database"));

        Self {
            screen: Screen::Home,
            screen_stack: Vec::new(),
            running: true,
            theme: Theme::lantern(),
            home_selected: 0,
            home_focus: HomeFocus::Queue,
            home_airing_selected: 0,
            home_airing_offset: 0,
            search_input: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
            search_loading: false,
            search_dirty: false,
            search_debounce_deadline: None,
            selected_anime: None,
            episodes: Vec::new(),
            episode_selected: 0,
            episodes_loading: false,
            jikan_anime: None,
            jikan_loading: false,
            synopsis_scroll: 0,
            cover_art: None,
            cover_art_loading: false,
            image_picker: picker,
            current_episode: None,
            playing_title: None,
            episode_url: None,
            active_watch_session_id: None,
            active_presence_token: None,
            active_presence_metadata: None,
            history,
            history_selected: 0,
            continue_watching,
            toasts: Vec::new(),
            spinner_tick: 0,
            splash_tick: 0,
            loading: false,
            target_episode: None,
            key_seq: None,
            player_type: options.player_type.unwrap_or_else(PlayerType::detect),
            mode: options.mode.unwrap_or(Mode::Sub),
            quality: "best".to_string(),
            discord_presence: options.discord_client_id.map(DiscordPresence::new),
            db,
            jikan,
            season_anime: Vec::new(),
            season_selected: 0,
            season_loading: false,
            season_year: None,
            season_name: None,
            season_page: 1,
            season_has_next: false,
            season_list: Vec::new(),
            season_filter_type: None,
            schedule_anime: Vec::new(),
            schedule_selected: 0,
            schedule_loading: false,
            schedule_day: "monday".to_string(),
            airing_today: Vec::new(),
            airing_today_loading: false,
            airing_today_last_fetch: None,
            home_season_label: None,
            home_season_count: None,
            home_season_last_fetch: None,
            top_anime: Vec::new(),
            top_selected: 0,
            top_loading: false,
            top_page: 1,
            top_has_next: false,
            top_filter_type: None,
            top_filter_rating: None,
            top_filter_sfw: false,
            genres: Vec::new(),
            genre_selected: 0,
            genre_loading: false,
            genre_picked: None,
            genre_anime: Vec::new(),
            genre_anime_selected: 0,
            genre_anime_loading: false,
            genre_anime_page: 1,
            genre_anime_has_next: false,
            recommendations: Vec::new(),
            recommendations_loading: false,
            recommendations_selected: 0,
            show_recommendations: false,
            update_available: None,
            update_check_in_progress: false,
            update_popup_visible: false,
            update_requested: false,
            update_in_progress: false,
            update_check_manual: false,
            update_notes_requested: false,
            current_session_id: 0,
        }
    }

    pub fn navigate(&mut self, screen: Screen) {
        if self.screen != screen {
            self.screen_stack.push(self.screen.clone());
            self.screen = screen;
        }
    }

    pub fn go_back(&mut self) {
        if let Some(prev) = self.screen_stack.pop() {
            self.screen = prev;
        } else {
            self.screen = Screen::Home;
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            Mode::Sub => Mode::Dub,
            Mode::Dub => Mode::Sub,
        };
    }

    pub fn schedule_search(&mut self, delay_ms: u64) {
        self.search_dirty = true;
        self.search_debounce_deadline =
            Some(Instant::now() + std::time::Duration::from_millis(delay_ms));
    }

    pub fn cancel_search_schedule(&mut self) {
        self.search_dirty = false;
        self.search_debounce_deadline = None;
    }

    pub fn toast(&mut self, msg: impl Into<String>, is_error: bool) {
        self.toasts.push(Toast {
            message: msg.into(),
            is_error,
            born: Instant::now(),
        });
        // cap
        if self.toasts.len() > 6 {
            let drop = self.toasts.len() - 6;
            self.toasts.drain(0..drop);
        }
    }

    pub fn refresh_history(&mut self) {
        self.history = self.db.get_history().unwrap_or_default();
        self.continue_watching = self.db.get_continue_watching().unwrap_or_default();
        if self.home_selected >= self.continue_watching.len() {
            self.home_selected = self.continue_watching.len().saturating_sub(1);
        }
    }

    pub fn play_episode(&mut self) -> Result<(), String> {
        let anime = self
            .selected_anime
            .as_ref()
            .ok_or("No anime selected")?
            .clone();
        let ep = self
            .episodes
            .get(self.episode_selected)
            .ok_or("No episode selected")?
            .clone();

        let title = format!("{} - Episode {}", anime.title, ep);
        self.current_episode = Some(ep.clone());
        self.playing_title = Some(title.clone());

        self.stop_active_watch_session();
        let session_id = self
            .db
            .start_watch_session(NewWatchSession {
                anime_id: &anime.id,
                title: &anime.title,
                episode: &ep,
                total_episodes: Some(anime.episode_count),
                player: self.player_type.name(),
                mode: self.mode.as_str(),
                quality: &self.quality,
            })
            .map_err(|e| format!("history: {}", e))?;
        self.active_watch_session_id = Some(session_id);

        if let Some(ref url_info) = self.episode_url {
            let launch = match player::launch_player(
                self.player_type,
                &url_info.url,
                &title,
                url_info.referer.as_deref(),
                url_info.subtitle.as_deref(),
                self.discord_presence.is_some(),
            ) {
                Ok(launch) => launch,
                Err(err) => {
                    self.stop_active_watch_session();
                    return Err(err);
                }
            };

            if let Some(discord_presence) = self.discord_presence.as_ref() {
                let token = discord_presence.next_token();
                let monitor = launch.activity_monitor.map(|monitor| match monitor {
                    player::PlayerActivityMonitor::Mpv { endpoint } => {
                        PlayerActivityMonitor::Mpv { endpoint }
                    }
                });
                discord_presence.start_playback(
                    PresencePlayback {
                        token,
                        anime_title: anime.title.clone(),
                        episode: ep.clone(),
                        total_episodes: Some(anime.episode_count),
                        player: self.player_type,
                        mode: self.mode.as_str().to_string(),
                        quality: url_info.quality.clone(),
                        started_at_unix: session_started_at_unix(),
                        metadata: self.active_presence_metadata.clone(),
                    },
                    monitor,
                );
                self.active_presence_token = Some(token);
            }
        }

        self.refresh_history();
        self.navigate(Screen::NowPlaying);
        Ok(())
    }

    pub fn stop_active_watch_session(&mut self) {
        if let Some(session_id) = self.active_watch_session_id.take() {
            let _ = self.db.stop_watch_session(session_id);
            self.refresh_history();
        }
        if let Some(token) = self.active_presence_token.take() {
            if let Some(discord_presence) = self.discord_presence.as_ref() {
                discord_presence.stop(token);
            }
        }
    }

    pub fn next_episode(&mut self) -> bool {
        if self.episode_selected + 1 < self.episodes.len() {
            self.episode_selected += 1;
            true
        } else {
            false
        }
    }
    pub fn previous_episode(&mut self) -> bool {
        if self.episode_selected > 0 {
            self.episode_selected -= 1;
            true
        } else {
            false
        }
    }

    pub fn tick_spinner(&mut self) {
        self.spinner_tick = self.spinner_tick.wrapping_add(1);
        self.splash_tick = self.splash_tick.saturating_add(1);
        // expire toasts older than 2.5s
        self.toasts.retain(|t| t.born.elapsed().as_millis() < 2500);
        // expire stale key sequence (>600ms)
        if let Some((_, born)) = self.key_seq {
            if born.elapsed().as_millis() > 600 {
                self.key_seq = None;
            }
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
