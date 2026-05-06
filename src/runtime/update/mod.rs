use super::msg::{Cmd, Msg};
use crate::app::{App, Screen};

pub mod detail;
pub mod history;
pub mod home;
pub mod playing;
pub mod search;

pub fn update(app: &mut App, msg: Msg, cols: usize) -> Vec<Cmd> {
    match msg {
        Msg::Tick => {
            app.spinner_tick = app.spinner_tick.wrapping_add(1);
            vec![]
        }
        Msg::Navigate(screen) => {
            app.current_session_id += 1;
            let sid = app.current_session_id;
            app.key_seq = None;
            app.navigate(screen.clone());
            match screen {
                Screen::Home => {
                    app.airing_today_loading = true;
                    app.top_loading = true;
                    vec![Cmd::FetchAiringToday(sid), Cmd::FetchTopAiring(sid)]
                }
                _ => vec![],
            }
        }
        Msg::Back => {
            app.go_back();
            vec![]
        }
        Msg::Toast(m, is_err) => {
            app.toast(m, is_err);
            vec![]
        }

        // Delegate to screen-specific updates
        Msg::Key(key) => {
            // Handle 'g' prefix
            if key.code == crossterm::event::KeyCode::Char('g') && app.key_seq.is_none() {
                app.key_seq = Some(('g', std::time::Instant::now()));
                return vec![];
            }

            // Handle 'gg' sequence (scroll to top)
            if let Some((first, _)) = app.key_seq.take() {
                if first == 'g' && key.code == crossterm::event::KeyCode::Char('g') {
                    match app.screen {
                        Screen::Home => app.home_selected = 0,
                        Screen::AnimeDetail => app.episode_selected = 0,
                        Screen::WatchHistory => app.history_selected = 0,
                        _ => {}
                    }
                    return vec![];
                }
            }

            if key.code == crossterm::event::KeyCode::Char('U') {
                if !app.update_in_progress && app.update_available.is_none() {
                    app.update_check_in_progress = true;
                    app.update_check_manual = true;
                    app.update_requested = false;
                }
                return vec![];
            }

            match app.screen {
                Screen::Home => home::update(app, key),
                Screen::AnimeDetail => detail::update(app, key, cols),
                Screen::Search => search::update(app, key),
                Screen::NowPlaying => playing::update(app, key),
                Screen::WatchHistory => history::update(app, key),
                _ => vec![],
            }
        }


        Msg::PerformSearch(query) => {
            if query.len() >= 2 {
                app.current_session_id += 1;
                let sid = app.current_session_id;
                app.search_loading = true;
                vec![Cmd::SearchAnime(query, sid)]
            } else {
                app.search_results.clear();
                vec![]
            }
        }

        Msg::SearchLoaded(res, sid) => {
            if sid != app.current_session_id {
                return vec![];
            }
            app.search_loading = false;
            match res {
                Ok(results) => {
                    app.search_results = results;
                    app.search_selected = 0;
                }
                Err(e) => app.toast(e, true),
            }
            vec![]
        }

        // Data Loading results (centralized for now)
        Msg::HomeAiringLoaded(res, sid) => {
            if sid != app.current_session_id {
                return vec![];
            }
            app.airing_today_loading = false;
            match res {
                Ok(data) => app.airing_today = data,
                Err(e) => app.toast(e, true),
            }
            vec![]
        }
        Msg::HomeTopLoaded(res, sid) => {
            if sid != app.current_session_id {
                return vec![];
            }
            app.top_loading = false;
            match res {
                Ok(data) => app.top_anime = data,
                Err(e) => app.toast(e, true),
            }
            vec![]
        }
        Msg::EpisodesLoaded(res, sid) => {
            if sid != app.current_session_id {
                return vec![];
            }
            app.episodes_loading = false;
            match res {
                Ok((eps, resolved_id)) => {
                    app.episodes = eps;
                    if let Some(ref mut anime) = app.selected_anime {
                        anime.id = resolved_id;
                    }
                    if let Some(target) = app.target_episode.take() {
                        app.episode_selected =
                            app.episodes.iter().position(|e| e == &target).unwrap_or(0);
                    }
                }
                Err(e) => app.toast(e, true),
            }
            vec![]
        }

        Msg::MetadataLoaded(data, sid) => {
            if sid != app.current_session_id {
                return vec![];
            }
            app.jikan_anime = *data;
            app.jikan_loading = false;
            if let Some(ref jikan) = app.jikan_anime {
                if let Some(url) = jikan.images.best_url() {
                    app.cover_art_loading = true;
                    return vec![Cmd::FetchImage(url.to_string(), sid)];
                }
            }
            vec![]
        }

        Msg::ImageLoaded(img, sid) => {
            if sid != app.current_session_id {
                return vec![];
            }
            app.cover_art_loading = false;
            if let (Some(ref picker), Some(img)) = (&app.image_picker, img) {
                let protocol = picker.new_resize_protocol(img);
                app.cover_art = Some(crate::ui::cover_image::CoverArt {
                    protocol: std::cell::RefCell::new(protocol),
                });
            }
            vec![]
        }

        Msg::StreamReady(data) => {
            if let Some(eps) = data.episodes.clone() {
                app.episodes = eps;
                app.episode_selected = app
                    .episodes
                    .iter()
                    .position(|e| e == &data.entry.episode)
                    .unwrap_or(0);
            }
            app.episode_url = Some(data.url.clone());
            app.active_presence_metadata = data.metadata.clone();

            vec![Cmd::ExecutePlayer(data)]
        }
        Msg::RecommendationsLoaded(res, sid) => {
            if sid != app.current_session_id {
                return vec![];
            }
            app.recommendations_loading = false;
            match res {
                Ok(data) => {
                    app.recommendations = data;
                    app.recommendations_selected = 0;
                }
                Err(e) => app.toast(e, true),
            }
            vec![]
        }
        Msg::RefreshHistory => {
            app.refresh_history();
            vec![]
        }
        Msg::Quit => {
            app.running = false;
            vec![]
        }
        Msg::ToggleUpdatePopup(v) => {
            app.update_popup_visible = v;
            vec![]
        }
        Msg::TriggerUpdate => {
            app.update_in_progress = true;
            vec![Cmd::PerformUpdate]
        }
        Msg::UpdateStatus(res) => {
            app.update_in_progress = false;
            match res {
                Ok(outcome) => {
                    app.toast(outcome.message, false);
                    if outcome.restart_required {
                        app.toast("restart required to finish update", false);
                    }
                    app.update_available = None;
                    app.update_popup_visible = false;
                }
                Err(e) => app.toast(format!("update failed: {}", e), true),
            }
            vec![]
        }
        Msg::UpdateCheckResult(res) => {
            app.update_check_in_progress = false;
            app.update_requested = false;
            match res {
                Ok(Some(info)) => {
                    app.update_available = Some(info);
                    app.update_popup_visible = true;
                }
                Ok(None) => {
                    if app.update_check_manual {
                        app.toast("app is up to date", false);
                    }
                }
                Err(e) => app.toast(format!("update check failed: {}", e), true),
            }
            app.update_check_manual = false;

            // Save last checked time
            let _ = app.db.set_state(
                "update_last_checked",
                &chrono::Utc::now().to_rfc3339(),
            );
            vec![]
        }
        Msg::Batch(msgs) => {
            let mut all_cmds = vec![];
            for m in msgs {
                all_cmds.extend(update(app, m, cols));
            }
            all_cmds
        }
        _ => vec![],
    }
}
