use super::super::msg::Cmd;
use crate::app::{App, HomeFocus, Screen};
use crossterm::event::{KeyCode, KeyEvent};

pub fn update(app: &mut App, key: KeyEvent) -> Vec<Cmd> {
    match key.code {
        KeyCode::Char('q') => {
            app.running = false;
            vec![]
        }
        KeyCode::Char('s') => {
            app.current_session_id += 1;
            app.navigate(Screen::Search);
            vec![]
        }
        KeyCode::Char('w') => {
            app.current_session_id += 1;
            app.refresh_history();
            app.history_selected = 0;
            app.navigate(Screen::WatchHistory);
            vec![]
        }
        KeyCode::Char('d') => {
            app.toggle_mode();
            vec![]
        }
        KeyCode::Tab => {
            app.home_focus = match app.home_focus {
                HomeFocus::Queue => HomeFocus::Airing,
                HomeFocus::Airing => HomeFocus::Trending,
                HomeFocus::Trending => HomeFocus::Queue,
            };
            vec![]
        }
        KeyCode::BackTab => {
            app.home_focus = match app.home_focus {
                HomeFocus::Queue => HomeFocus::Trending,
                HomeFocus::Trending => HomeFocus::Airing,
                HomeFocus::Airing => HomeFocus::Queue,
            };
            vec![]
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.home_focus == HomeFocus::Queue {
                if app.home_selected > 0 {
                    app.home_selected -= 1;
                } else {
                    app.home_focus = HomeFocus::Airing;
                }
            } else if app.home_focus == HomeFocus::Trending {
                if app.top_selected > 0 {
                    app.top_selected -= 1;
                } else {
                    app.home_focus = HomeFocus::Airing;
                }
            }
            vec![]
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.home_focus == HomeFocus::Queue {
                if app.home_selected + 1 < app.continue_watching.len() {
                    app.home_selected += 1;
                }
            } else if app.home_focus == HomeFocus::Trending {
                if app.top_selected + 1 < app.top_anime.len().min(5) {
                    app.top_selected += 1;
                }
            } else if app.home_focus == HomeFocus::Airing {
                app.home_focus = HomeFocus::Queue;
            }
            vec![]
        }
        KeyCode::Left | KeyCode::Char('h') => {
            if app.home_focus == HomeFocus::Airing {
                if app.home_airing_selected > 0 {
                    app.home_airing_selected -= 1;
                    if app.home_airing_selected < app.home_airing_offset {
                        app.home_airing_offset = app.home_airing_selected;
                    }
                }
            } else {
                app.home_focus = HomeFocus::Queue;
            }
            vec![]
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if app.home_focus == HomeFocus::Airing {
                if app.home_airing_selected + 1 < app.airing_today.len() {
                    app.home_airing_selected += 1;
                    // Note: terminal size check should be handled carefully
                    // For now we assume a reasonable default or pass it in
                    if app.home_airing_selected >= app.home_airing_offset + 5 {
                        app.home_airing_offset += 1;
                    }
                }
            } else if app.home_focus == HomeFocus::Queue {
                app.home_focus = HomeFocus::Trending;
            } else {
                app.home_focus = HomeFocus::Airing;
            }
            vec![]
        }
        KeyCode::Enter | KeyCode::Char('p') => {
            match app.home_focus {
                HomeFocus::Airing => {
                    if let Some(anime) = app.airing_today.get(app.home_airing_selected).cloned() {
                        app.selected_anime = Some(crate::api::AnimeResult {
                            id: anime.mal_id.to_string(),
                            title: anime.display_title().to_string(),
                            episode_count: anime.episodes.unwrap_or(0),
                        });
                        app.current_session_id += 1;
                        let sid = app.current_session_id;
                        app.navigate(Screen::AnimeDetail);
                        app.episodes.clear();
                        app.recommendations.clear();
                        app.episodes_loading = true;
                        app.jikan_loading = true;
                        app.cover_art = None;
                        app.cover_art_loading = true;
                        vec![Cmd::FetchAnimeDetail {
                            provider_id: None,
                            title: anime.display_title().to_string(),
                            session_id: sid,
                        }]
                    } else {
                        vec![]
                    }
                }
                HomeFocus::Trending => {
                    if let Some(anime) = app.top_anime.get(app.top_selected).cloned() {
                        app.selected_anime = Some(crate::api::AnimeResult {
                            id: anime.mal_id.to_string(),
                            title: anime.display_title().to_string(),
                            episode_count: anime.episodes.unwrap_or(0),
                        });
                        app.current_session_id += 1;
                        let sid = app.current_session_id;
                        app.navigate(Screen::AnimeDetail);
                        app.episodes.clear();
                        app.recommendations.clear();
                        app.episodes_loading = true;
                        app.jikan_loading = true;
                        app.cover_art = None;
                        app.cover_art_loading = true;
                        vec![Cmd::FetchAnimeDetail {
                            provider_id: None,
                            title: anime.display_title().to_string(),
                            session_id: sid,
                        }]
                    } else {
                        vec![]
                    }
                }
                HomeFocus::Queue => {
                    if let Some(entry) = app.continue_watching.get(app.home_selected).cloned() {
                        // Set selected_anime so play_episode has context
                        app.selected_anime = Some(crate::api::AnimeResult {
                            id: entry.anime_id.clone(),
                            title: entry.title.clone(),
                            episode_count: entry.total_episodes.unwrap_or(0),
                        });
                        app.target_episode = Some(entry.episode.clone());
                        app.toast(
                            format!("resuming {} ep {}...", entry.title, entry.episode),
                            false,
                        );
                        vec![Cmd::LaunchPlayer(entry)]
                    } else {
                        app.toast("nothing to resume", false);
                        vec![]
                    }
                }
            }
        }
        _ => vec![],
    }
}
