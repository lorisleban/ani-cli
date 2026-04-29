use std::time::Instant;

use crate::api::{AnimeResult, EpisodeUrl, Mode};
use crate::db::{Database, WatchEntry};
use crate::player::{self, PlayerType};
use crate::theme::Theme;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Home,
    Search,
    AnimeDetail,
    WatchHistory,
    NowPlaying,
    Help,
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

    // Now playing
    pub current_episode: Option<String>,
    pub playing_title: Option<String>,
    pub episode_url: Option<EpisodeUrl>,

    // History
    pub history: Vec<WatchEntry>,
    pub history_selected: usize,
    pub continue_watching: Vec<WatchEntry>,

    // Status
    pub toasts: Vec<Toast>,
    pub spinner_tick: usize,
    pub splash_tick: usize,
    pub loading: bool,

    // Vim-style key buffer (e.g. `g`+`h`)
    pub key_seq: Option<(char, Instant)>,

    // Player / mode
    pub player_type: PlayerType,
    pub mode: Mode,
    pub quality: String,

    pub db: Database,
}

impl App {
    pub fn new() -> Self {
        let db = Database::new().expect("Failed to initialize database");
        let history = db.get_history().unwrap_or_default();
        let continue_watching = db.get_continue_watching().unwrap_or_default();

        Self {
            screen: Screen::Home,
            screen_stack: Vec::new(),
            running: true,
            theme: Theme::lantern(),
            home_selected: 0,
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
            current_episode: None,
            playing_title: None,
            episode_url: None,
            history,
            history_selected: 0,
            continue_watching,
            toasts: Vec::new(),
            spinner_tick: 0,
            splash_tick: 0,
            loading: false,
            key_seq: None,
            player_type: PlayerType::detect(),
            mode: Mode::Sub,
            quality: "best".to_string(),
            db,
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
        let anime = self.selected_anime.as_ref().ok_or("No anime selected")?;
        let ep = self
            .episodes
            .get(self.episode_selected)
            .ok_or("No episode selected")?
            .clone();

        let title = format!("{} — Episode {}", anime.title, ep);
        self.current_episode = Some(ep.clone());
        self.playing_title = Some(title.clone());

        self.db
            .upsert_watch(&anime.id, &anime.title, &ep, Some(anime.episode_count))
            .map_err(|e| format!("history: {}", e))?;
        self.refresh_history();

        if let Some(ref url_info) = self.episode_url {
            player::launch_player(
                self.player_type,
                &url_info.url,
                &title,
                url_info.referer.as_deref(),
                url_info.subtitle.as_deref(),
            )?;
        }

        self.navigate(Screen::NowPlaying);
        Ok(())
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
