use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::terminal::AppTerminal;
use crate::api::ApiClient;
use crate::app::{App, Screen};
use crate::ui;
use crate::update::{open_release_notes, perform_update, UpdateOutcome};
use chrono::{DateTime, Utc};

const SEARCH_DEBOUNCE_MS: u64 = 180;

pub async fn run_app(
    terminal: &mut AppTerminal,
    app: &mut App,
    mut api: ApiClient,
) -> Result<(), Box<dyn std::error::Error>> {
    let tick_rate = Duration::from_millis(80);
    let mut last_tick = Instant::now();
    let mut pending_update_check = start_update_check(app);
    let mut pending_update_action: Option<tokio::task::JoinHandle<Result<UpdateOutcome, String>>> =
        None;

    loop {
        terminal.draw(|f| ui::render(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_default();

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                if handle_global(app, key) {
                    if !app.running {
                        break;
                    }
                    continue;
                }

                match app.screen {
                    Screen::Home => on_home(app, key, &api, terminal).await,
                    Screen::Search => on_search(app, key, &mut api, terminal).await,
                    Screen::AnimeDetail => on_detail(app, key, &mut api, terminal).await,
                    Screen::WatchHistory => on_history(app, key),
                    Screen::NowPlaying => on_playing(app, key, &api, terminal).await,
                    Screen::Help => on_help(app, key),
                    Screen::SeasonBrowse => on_season(app, key, &mut api, terminal).await,
                    Screen::Schedule => on_schedule(app, key, &mut api, terminal).await,
                }
            }
        }

        if !app.running {
            break;
        }

        if let Some(handle) = pending_update_check.take() {
            if handle.is_finished() {
                match handle.await {
                    Ok(result) => match result {
                        Ok(info) => apply_update_result(app, info),
                        Err(err) => {
                            app.toast(format!("update check failed: {}", err), true);
                            app.update_check_manual = false;
                        }
                    },
                    Err(err) => {
                        app.toast(format!("update check failed: {}", err), true);
                        app.update_check_manual = false;
                    }
                }
            } else {
                pending_update_check = Some(handle);
            }
        }

        if pending_update_check.is_none() {
            pending_update_check = start_update_check(app);
        }

        if app.update_requested && pending_update_action.is_none() {
            app.update_requested = false;
            app.update_in_progress = true;
            app.update_popup_visible = false;
            app.toast("updating...", false);
            pending_update_action = Some(tokio::spawn(async { perform_update().await }));
        }

        if let Some(handle) = pending_update_action.take() {
            if handle.is_finished() {
                app.update_in_progress = false;
                match handle.await {
                    Ok(outcome) => match outcome {
                        Ok(result) => {
                            app.toast(result.message, false);
                            if result.restart_required {
                                app.toast("restart required to finish update", false);
                            }
                        }
                        Err(err) => app.toast(format!("update failed: {}", err), true),
                    },
                    Err(err) => app.toast(format!("update failed: {}", err), true),
                }
            } else {
                pending_update_action = Some(handle);
            }
        }

        if app.update_notes_requested {
            app.update_notes_requested = false;
            if let Some(info) = app.update_available.clone() {
                match open_release_notes(&info.release_url) {
                    Ok(()) => app.toast("opened release notes", false),
                    Err(err) => app.toast(format!("release notes failed: {}", err), true),
                }
            }
        }

        maybe_run_debounced_search(app, &mut api, terminal).await;

        if last_tick.elapsed() >= tick_rate {
            app.tick_spinner();
            last_tick = Instant::now();
        }
    }
    Ok(())
}

fn reset_search_state(app: &mut App) {
    app.search_input.clear();
    app.search_results.clear();
    app.search_selected = 0;
    app.cancel_search_schedule();
}

async fn maybe_run_debounced_search(
    app: &mut App,
    api: &mut ApiClient,
    terminal: &mut AppTerminal,
) {
    if app.screen != Screen::Search || app.search_loading || !app.search_dirty {
        return;
    }

    let ready = app
        .search_debounce_deadline
        .map(|deadline| Instant::now() >= deadline)
        .unwrap_or(false);
    if !ready {
        return;
    }

    app.cancel_search_schedule();
    if app.search_input.len() >= 2 {
        run_search(app, api, terminal).await;
    } else {
        app.search_results.clear();
    }
}

/// Handle keys that work everywhere. Returns true if consumed.
fn handle_global(app: &mut App, key: KeyEvent) -> bool {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.running = false;
        return true;
    }

    if let Some((first, _)) = app.key_seq {
        app.key_seq = None;
        if first == 'g' {
            match key.code {
                KeyCode::Char('h') => {
                    app.navigate(Screen::Home);
                    return true;
                }
                KeyCode::Char('s') => {
                    app.season_loading = true;
                    app.season_anime.clear();
                    app.season_selected = 0;
                    app.season_page = 1;
                    app.season_year = None;
                    app.season_name = None;
                    app.navigate(Screen::SeasonBrowse);
                    return true;
                }
                KeyCode::Char('c') => {
                    app.schedule_loading = true;
                    app.schedule_anime.clear();
                    app.schedule_selected = 0;
                    app.navigate(Screen::Schedule);
                    return true;
                }
                KeyCode::Char('w') => {
                    app.refresh_history();
                    app.history_selected = 0;
                    app.navigate(Screen::WatchHistory);
                    return true;
                }
                KeyCode::Char('p') => {
                    if app.current_episode.is_some() {
                        app.navigate(Screen::NowPlaying);
                    } else {
                        app.toast("nothing playing", false);
                    }
                    return true;
                }
                KeyCode::Char('g') => {
                    match app.screen {
                        Screen::Home => app.home_selected = 0,
                        Screen::Search => app.search_selected = 0,
                        Screen::AnimeDetail => app.episode_selected = 0,
                        Screen::WatchHistory => app.history_selected = 0,
                        _ => {}
                    }
                    return true;
                }
                _ => {}
            }
        }
    }

    match key.code {
        KeyCode::Char('U') => {
            if app.update_in_progress {
                app.toast("update already running", false);
            } else if app.update_available.is_some() {
                app.update_requested = true;
            } else {
                app.toast("checking for updates...", false);
                app.update_check_in_progress = true;
                app.update_check_manual = true;
            }
            true
        }
        KeyCode::Char('R') => {
            if app.update_available.is_some() {
                app.update_notes_requested = true;
            } else {
                app.toast("no release notes available", false);
            }
            true
        }
        KeyCode::Esc if app.update_popup_visible => {
            app.update_popup_visible = false;
            true
        }
        KeyCode::Char('?') => {
            app.navigate(Screen::Help);
            true
        }
        KeyCode::Char('Q') => {
            app.running = false;
            true
        }
        KeyCode::Char('G') => {
            match app.screen {
                Screen::Home if !app.continue_watching.is_empty() => {
                    app.home_selected = app.continue_watching.len() - 1;
                }
                Screen::Search if !app.search_results.is_empty() => {
                    app.search_selected = app.search_results.len() - 1;
                }
                Screen::AnimeDetail if !app.episodes.is_empty() => {
                    app.episode_selected = app.episodes.len() - 1;
                }
                Screen::WatchHistory if !app.history.is_empty() => {
                    app.history_selected = app.history.len() - 1;
                }
                _ => {}
            }
            true
        }
        KeyCode::Char('g') => {
            app.key_seq = Some(('g', Instant::now()));
            true
        }
        KeyCode::Char('/') if app.screen != Screen::Search => {
            reset_search_state(app);
            app.navigate(Screen::Search);
            true
        }
        _ => false,
    }
}

fn start_update_check(
    app: &mut App,
) -> Option<tokio::task::JoinHandle<Result<Option<crate::update::UpdateInfo>, String>>> {
    if !app.update_check_in_progress {
        return None;
    }
    app.update_check_in_progress = false;
    Some(tokio::spawn(async {
        crate::update::check_for_update().await
    }))
}

fn apply_update_result(app: &mut App, result: Option<crate::update::UpdateInfo>) {
    let now = DateTime::<Utc>::from(std::time::SystemTime::now()).to_rfc3339();
    let _ = app.db.set_state("update_last_checked", &now);
    if let Some(info) = result {
        app.update_available = Some(info);
        app.update_popup_visible = true;
        app.toast("update ready — press U to install, R for notes", false);
    } else if app.update_check_manual {
        app.toast("no updates found", false);
    }
    app.update_check_manual = false;
}

async fn on_home(app: &mut App, key: KeyEvent, api: &ApiClient, terminal: &mut AppTerminal) {
    match key.code {
        KeyCode::Char('q') => app.running = false,
        KeyCode::Char('s') => {
            reset_search_state(app);
            app.navigate(Screen::Search);
        }
        KeyCode::Char('w') => {
            app.refresh_history();
            app.history_selected = 0;
            app.navigate(Screen::WatchHistory);
        }
        KeyCode::Char('d') => app.toggle_mode(),
        KeyCode::Up | KeyCode::Char('k') if app.home_selected > 0 => {
            app.home_selected -= 1;
        }
        KeyCode::Down | KeyCode::Char('j')
            if app.home_selected + 1 < app.continue_watching.len() =>
        {
            app.home_selected += 1;
        }
        KeyCode::Enter | KeyCode::Char('r') => {
            if let Some(entry) = app.continue_watching.get(app.home_selected).cloned() {
                resume_entry(app, &entry, api, terminal).await;
            } else {
                app.toast("nothing to resume - press s to search", false);
            }
        }
        _ => {}
    }
}

async fn resume_entry(
    app: &mut App,
    entry: &crate::db::WatchEntry,
    api: &ApiClient,
    terminal: &mut AppTerminal,
) {
    app.search_loading = true;
    let _ = terminal.draw(|f| ui::render(f, app));

    match api.search_anime(&entry.title).await {
        Ok(results) => {
            if let Some(anime) = results.into_iter().find(|r| r.id == entry.anime_id) {
                app.selected_anime = Some(anime.clone());
                app.episodes_loading = true;
                app.jikan_anime = None;
                app.jikan_loading = true;
                app.synopsis_scroll = 0;
                app.navigate(Screen::AnimeDetail);
                let _ = terminal.draw(|f| ui::render(f, app));

                let jikan_client = app.jikan.clone();
                let (ep_result, jikan_result) = tokio::join!(
                    api.episodes_list(&entry.anime_id),
                    jikan_client.fetch_jikan_anime(&anime.title, Some(anime.episode_count))
                );

                if let Ok(episodes) = ep_result {
                    app.episodes = episodes;
                    let current_ep: f64 = entry.episode.parse().unwrap_or(0.0);
                    app.episode_selected = app
                        .episodes
                        .iter()
                        .position(|ep| ep.parse::<f64>().unwrap_or(0.0) > current_ep)
                        .unwrap_or(0);
                }
                app.episodes_loading = false;
                app.jikan_anime = jikan_result.ok().flatten();
                app.jikan_loading = false;
            } else {
                app.toast("couldn't find that show anymore", true);
            }
        }
        Err(e) => app.toast(e, true),
    }
    app.search_loading = false;
}

async fn on_search(app: &mut App, key: KeyEvent, api: &mut ApiClient, terminal: &mut AppTerminal) {
    match key.code {
        KeyCode::Esc => app.go_back(),
        KeyCode::Up if app.search_selected > 0 => {
            app.search_selected -= 1;
        }
        KeyCode::Down if app.search_selected + 1 < app.search_results.len() => {
            app.search_selected += 1;
        }
        KeyCode::Backspace => {
            app.search_input.pop();
            if app.search_input.len() >= 2 {
                app.schedule_search(SEARCH_DEBOUNCE_MS);
            } else {
                app.search_results.clear();
                app.cancel_search_schedule();
            }
        }
        KeyCode::Enter => {
            if let Some(result) = app.search_results.get(app.search_selected).cloned() {
                app.selected_anime = Some(result.clone());
                app.episodes_loading = true;
                app.jikan_anime = None;
                app.jikan_loading = true;
                app.synopsis_scroll = 0;
                app.navigate(Screen::AnimeDetail);
                let _ = terminal.draw(|f| ui::render(f, app));
                let jikan_client = app.jikan.clone();
                let (ep_result, jikan_result) = tokio::join!(
                    api.episodes_list(&result.id),
                    jikan_client.fetch_jikan_anime(&result.title, Some(result.episode_count))
                );
                match ep_result {
                    Ok(eps) => {
                        app.episodes = eps;
                        app.episode_selected = 0;
                    }
                    Err(e) => app.toast(e, true),
                }
                app.episodes_loading = false;
                app.jikan_anime = jikan_result.ok().flatten();
                app.jikan_loading = false;
            }
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.search_input.push(c);
            if app.search_input.len() >= 2 {
                app.schedule_search(SEARCH_DEBOUNCE_MS);
            } else {
                app.cancel_search_schedule();
            }
        }
        _ => {}
    }
}

async fn run_search(app: &mut App, api: &mut ApiClient, terminal: &mut AppTerminal) {
    app.search_loading = true;
    api.mode = app.mode;
    let _ = terminal.draw(|f| ui::render(f, app));
    match api.search_anime(&app.search_input).await {
        Ok(results) => {
            app.search_results = results;
            app.search_selected = 0;
        }
        Err(e) => app.toast(e, true),
    }
    app.search_loading = false;
}

async fn on_detail(app: &mut App, key: KeyEvent, api: &mut ApiClient, terminal: &mut AppTerminal) {
    let cols = grid_cols(app);
    match key.code {
        KeyCode::Esc => app.go_back(),
        KeyCode::Up | KeyCode::Char('k') if app.episode_selected >= cols => {
            app.episode_selected -= cols;
        }
        KeyCode::Down | KeyCode::Char('j') if app.episode_selected + cols < app.episodes.len() => {
            app.episode_selected += cols;
        }
        KeyCode::Left | KeyCode::Char('h') if app.episode_selected > 0 => {
            app.episode_selected -= 1;
        }
        KeyCode::Right | KeyCode::Char('l') if app.episode_selected + 1 < app.episodes.len() => {
            app.episode_selected += 1;
        }
        KeyCode::Char('K') => {
            app.synopsis_scroll = app.synopsis_scroll.saturating_sub(1);
        }
        KeyCode::Char('J') => {
            app.synopsis_scroll = app.synopsis_scroll.saturating_add(1);
        }
        KeyCode::Char('d') => {
            app.toggle_mode();
            if let Some(anime) = app.selected_anime.clone() {
                app.episodes_loading = true;
                api.mode = app.mode;
                let _ = terminal.draw(|f| ui::render(f, app));
                match api.episodes_list(&anime.id).await {
                    Ok(eps) => {
                        app.episodes = eps;
                        app.episode_selected = 0;
                    }
                    Err(e) => app.toast(e, true),
                }
                app.episodes_loading = false;
            }
        }
        KeyCode::Enter | KeyCode::Char('p') => {
            play_selected(app, api, terminal).await;
        }
        _ => {}
    }
}

fn grid_cols(_app: &App) -> usize {
    8
}

async fn play_selected(app: &mut App, api: &ApiClient, terminal: &mut AppTerminal) {
    let anime = match app.selected_anime.clone() {
        Some(a) => a,
        None => return,
    };
    let ep = match app.episodes.get(app.episode_selected).cloned() {
        Some(e) => e,
        None => return,
    };
    app.loading = true;
    app.toast("fetching stream...", false);
    let _ = terminal.draw(|f| ui::render(f, app));

    let metadata_client = app.jikan.clone();
    let (stream_result, metadata_result) = tokio::join!(
        api.get_episode_url(&anime.id, &ep, &app.quality),
        metadata_client.fetch_presence_metadata(&anime.title, Some(anime.episode_count))
    );

    app.active_presence_metadata = metadata_result.ok().flatten();

    match stream_result {
        Ok(url) => {
            app.episode_url = Some(url);
            if let Err(e) = app.play_episode() {
                app.toast(e, true);
            } else {
                app.toast("playing", false);
            }
        }
        Err(e) => app.toast(format!("failed: {}", e), true),
    }
    app.loading = false;
}

fn on_history(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.go_back(),
        KeyCode::Up | KeyCode::Char('k') if app.history_selected > 0 => {
            app.history_selected -= 1;
        }
        KeyCode::Down | KeyCode::Char('j') if app.history_selected + 1 < app.history.len() => {
            app.history_selected += 1;
        }
        KeyCode::Char('x') => {
            if let Some(entry) = app.history.get(app.history_selected) {
                let id = entry.id;
                if app.db.delete_entry(id).is_ok() {
                    app.refresh_history();
                    if app.history_selected >= app.history.len() && app.history_selected > 0 {
                        app.history_selected -= 1;
                    }
                    app.toast("removed", false);
                }
            }
        }
        KeyCode::Char('X') if app.db.delete_all().is_ok() => {
            app.refresh_history();
            app.history_selected = 0;
            app.toast("history cleared", false);
        }
        _ => {}
    }
}

async fn on_playing(app: &mut App, key: KeyEvent, api: &ApiClient, terminal: &mut AppTerminal) {
    match key.code {
        KeyCode::Esc => app.navigate(Screen::AnimeDetail),
        KeyCode::Char('n') | KeyCode::Char('l') => {
            if app.next_episode() {
                play_selected(app, api, terminal).await;
            } else {
                app.toast("no more episodes", false);
            }
        }
        KeyCode::Char('p') | KeyCode::Char('h') => {
            if app.previous_episode() {
                play_selected(app, api, terminal).await;
            } else {
                app.toast("first episode already", false);
            }
        }
        KeyCode::Char('r') => {
            if let Err(e) = app.play_episode() {
                app.toast(e, true);
            } else {
                app.toast("replay", false);
            }
        }
        KeyCode::Char('s') => app.navigate(Screen::AnimeDetail),
        _ => {}
    }
}

fn on_help(app: &mut App, key: KeyEvent) {
    if matches!(
        key.code,
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')
    ) {
        app.go_back();
    }
}

async fn on_season(app: &mut App, key: KeyEvent, api: &mut ApiClient, terminal: &mut AppTerminal) {
    if app.season_loading && app.season_anime.is_empty() {
        let _ = terminal.draw(|f| ui::render(f, app));
        fetch_season_page(app).await;
    }

    match key.code {
        KeyCode::Esc => app.go_back(),
        KeyCode::Up | KeyCode::Char('k') if app.season_selected > 0 => {
            app.season_selected -= 1;
        }
        KeyCode::Down | KeyCode::Char('j') if app.season_selected + 1 < app.season_anime.len() => {
            app.season_selected += 1;
        }
        KeyCode::Char('n') => {
            if app.season_has_next {
                app.season_page += 1;
                app.season_loading = true;
                let _ = terminal.draw(|f| ui::render(f, app));
                fetch_season_page(app).await;
            }
        }
        KeyCode::Char('t') => {
            let types = [None, Some("TV"), Some("Movie"), Some("OVA"), Some("ONA")];
            let current_idx = types
                .iter()
                .position(|t| t.as_deref() == app.season_filter_type.as_deref())
                .unwrap_or(0);
            let next_idx = (current_idx + 1) % types.len();
            app.season_filter_type = types[next_idx].map(String::from);
            app.season_page = 1;
            app.season_loading = true;
            let _ = terminal.draw(|f| ui::render(f, app));
            fetch_season_page(app).await;
        }
        KeyCode::Char('[') => {
            navigate_season(app, -1).await;
            let _ = terminal.draw(|f| ui::render(f, app));
        }
        KeyCode::Char(']') => {
            navigate_season(app, 1).await;
            let _ = terminal.draw(|f| ui::render(f, app));
        }
        KeyCode::Enter => {
            if let Some(anime) = app.season_anime.get(app.season_selected).cloned() {
                enter_anime_from_jikan(app, &anime, api, terminal).await;
            }
        }
        _ => {}
    }
}

async fn fetch_season_page(app: &mut App) {
    let jikan = app.jikan.clone();
    let result = if let (Some(year), Some(season)) = (&app.season_year, &app.season_name) {
        jikan.get_season(*year, season, app.season_page).await
    } else {
        jikan.get_current_season(app.season_page).await
    };

    match result {
        Ok(page) => {
            app.season_has_next = page.pagination.has_next_page;
            let mut anime = page.data;
            if let Some(ref filter) = app.season_filter_type {
                anime.retain(|a| a.anime_type.as_deref() == Some(filter.as_str()));
            }
            if app.season_page == 1 {
                app.season_anime = anime;
            } else {
                app.season_anime.extend(anime);
            }
            if app.season_year.is_none() {
                if let Some(first) = app.season_anime.first() {
                    app.season_year = first.year;
                    app.season_name = first.season.clone();
                }
            }
        }
        Err(e) => app.toast(format!("season fetch failed: {}", e), true),
    }
    app.season_loading = false;
}

async fn navigate_season(app: &mut App, direction: i32) {
    if app.season_list.is_empty() {
        let jikan = app.jikan.clone();
        match jikan.get_season_list().await {
            Ok(list) => app.season_list = list,
            Err(e) => {
                app.toast(format!("season list failed: {}", e), true);
                return;
            }
        }
    }

    let current_year = app.season_year.unwrap_or(2026);
    let current_season = app
        .season_name
        .clone()
        .unwrap_or_else(|| "spring".to_string());
    let season_order = ["winter", "spring", "summer", "fall"];
    let current_idx = season_order
        .iter()
        .position(|s| *s == current_season.to_lowercase())
        .unwrap_or(1);

    let new_idx = current_idx as i32 + direction;
    let (new_year, new_season_idx) = if new_idx < 0 {
        (current_year - 1, 3)
    } else if new_idx >= 4 {
        (current_year + 1, 0)
    } else {
        (current_year, new_idx as usize)
    };

    app.season_year = Some(new_year);
    app.season_name = Some(season_order[new_season_idx].to_string());
    app.season_page = 1;
    app.season_selected = 0;
    app.season_loading = true;
    fetch_season_page(app).await;
}

async fn on_schedule(
    app: &mut App,
    key: KeyEvent,
    api: &mut ApiClient,
    terminal: &mut AppTerminal,
) {
    if app.schedule_loading && app.schedule_anime.is_empty() {
        let _ = terminal.draw(|f| ui::render(f, app));
        fetch_schedule_day(app).await;
    }

    let days = [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ];
    match key.code {
        KeyCode::Esc => app.go_back(),
        KeyCode::Up | KeyCode::Char('k') if app.schedule_selected > 0 => {
            app.schedule_selected -= 1;
        }
        KeyCode::Down | KeyCode::Char('j')
            if app.schedule_selected + 1 < app.schedule_anime.len() =>
        {
            app.schedule_selected += 1;
        }
        KeyCode::Left | KeyCode::Char('h') => {
            let idx = days
                .iter()
                .position(|d| *d == app.schedule_day)
                .unwrap_or(0);
            app.schedule_day = days[(idx + days.len() - 1) % days.len()].to_string();
            app.schedule_selected = 0;
            app.schedule_loading = true;
            let _ = terminal.draw(|f| ui::render(f, app));
            fetch_schedule_day(app).await;
        }
        KeyCode::Right | KeyCode::Char('l') => {
            let idx = days
                .iter()
                .position(|d| *d == app.schedule_day)
                .unwrap_or(0);
            app.schedule_day = days[(idx + 1) % days.len()].to_string();
            app.schedule_selected = 0;
            app.schedule_loading = true;
            let _ = terminal.draw(|f| ui::render(f, app));
            fetch_schedule_day(app).await;
        }
        KeyCode::Enter => {
            if let Some(anime) = app.schedule_anime.get(app.schedule_selected).cloned() {
                enter_anime_from_jikan(app, &anime, api, terminal).await;
            }
        }
        _ => {}
    }
}

async fn fetch_schedule_day(app: &mut App) {
    let jikan = app.jikan.clone();
    match jikan.get_schedule(&app.schedule_day, 1).await {
        Ok(page) => {
            app.schedule_anime = page.data;
            app.schedule_selected = 0;
        }
        Err(e) => app.toast(format!("schedule fetch failed: {}", e), true),
    }
    app.schedule_loading = false;
}

async fn enter_anime_from_jikan(
    app: &mut App,
    jikan_anime: &crate::domain::jikan::JikanAnime,
    api: &mut ApiClient,
    terminal: &mut AppTerminal,
) {
    let title = jikan_anime.display_title().to_string();
    let ep_count = jikan_anime.episodes.unwrap_or(0);

    app.search_loading = true;
    let _ = terminal.draw(|f| ui::render(f, app));

    match api.search_anime(&title).await {
        Ok(results) => {
            if let Some(anime) = results
                .iter()
                .find(|r| {
                    r.title.to_lowercase() == title.to_lowercase()
                        || (r.episode_count == ep_count && ep_count > 0)
                })
                .cloned()
                .or_else(|| results.into_iter().next())
            {
                app.selected_anime = Some(anime.clone());
                app.episodes_loading = true;
                app.jikan_anime = Some(jikan_anime.clone());
                app.jikan_loading = false;
                app.synopsis_scroll = 0;
                app.navigate(Screen::AnimeDetail);
                let _ = terminal.draw(|f| ui::render(f, app));
                match api.episodes_list(&anime.id).await {
                    Ok(eps) => {
                        app.episodes = eps;
                        app.episode_selected = 0;
                    }
                    Err(e) => app.toast(e, true),
                }
                app.episodes_loading = false;
            } else {
                app.toast("couldn't find that show on AllAnime", true);
            }
        }
        Err(e) => app.toast(e, true),
    }
    app.search_loading = false;
}
