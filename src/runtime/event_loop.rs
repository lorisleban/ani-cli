use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::terminal::AppTerminal;
use crate::api::ApiClient;
use crate::app::{App, Screen};
use crate::update::{open_release_notes, perform_update, UpdateOutcome};
use chrono::{DateTime, Utc};
use crate::ui;

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
                    reset_search_state(app);
                    app.navigate(Screen::Search);
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
    Some(tokio::spawn(async { crate::update::check_for_update().await }))
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
                app.selected_anime = Some(anime);
                app.episodes_loading = true;
                app.navigate(Screen::AnimeDetail);
                let _ = terminal.draw(|f| ui::render(f, app));

                if let Ok(episodes) = api.episodes_list(&entry.anime_id).await {
                    app.episodes = episodes;
                    let current_ep: f64 = entry.episode.parse().unwrap_or(0.0);
                    app.episode_selected = app
                        .episodes
                        .iter()
                        .position(|ep| ep.parse::<f64>().unwrap_or(0.0) > current_ep)
                        .unwrap_or(0);
                }
                app.episodes_loading = false;
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
                app.navigate(Screen::AnimeDetail);
                let _ = terminal.draw(|f| ui::render(f, app));
                match api.episodes_list(&result.id).await {
                    Ok(eps) => {
                        app.episodes = eps;
                        app.episode_selected = 0;
                    }
                    Err(e) => app.toast(e, true),
                }
                app.episodes_loading = false;
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

    let metadata_client = api.clone();
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
