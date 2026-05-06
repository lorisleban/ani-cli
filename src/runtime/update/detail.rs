use super::super::msg::Cmd;
use crate::app::App;
use crate::db::WatchEntry;
use crossterm::event::{KeyCode, KeyEvent};

pub fn update(app: &mut App, key: KeyEvent, cols: usize) -> Vec<Cmd> {
    match key.code {
        KeyCode::Esc => {
            app.go_back();
            vec![]
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.show_recommendations {
                if app.recommendations_selected > 0 {
                    app.recommendations_selected -= 1;
                }
            } else if app.episode_selected >= cols {
                app.episode_selected -= cols;
            }
            vec![]
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.show_recommendations {
                if app.recommendations_selected + 1 < app.recommendations.len() {
                    app.recommendations_selected += 1;
                }
            } else if app.episode_selected + cols < app.episodes.len() {
                app.episode_selected += cols;
            }
            vec![]
        }
        KeyCode::Left | KeyCode::Char('h')
            if !app.show_recommendations && app.episode_selected > 0 =>
        {
            app.episode_selected -= 1;
            vec![]
        }
        KeyCode::Right | KeyCode::Char('l')
            if !app.show_recommendations && app.episode_selected + 1 < app.episodes.len() =>
        {
            app.episode_selected += 1;
            vec![]
        }
        KeyCode::Char('K') => {
            app.synopsis_scroll = app.synopsis_scroll.saturating_sub(1);
            vec![]
        }
        KeyCode::Char('J') => {
            app.synopsis_scroll = app.synopsis_scroll.saturating_add(1);
            vec![]
        }
        KeyCode::Char('d') => {
            app.toggle_mode();
            if let Some(anime) = app.selected_anime.clone() {
                app.episodes_loading = true;
                vec![Cmd::FetchAnimeDetail {
                    provider_id: Some(anime.id.clone()),
                    title: anime.title.clone(),
                    session_id: app.current_session_id,
                }]
            } else {
                vec![]
            }
        }
        KeyCode::Char('r') => {
            app.show_recommendations = !app.show_recommendations;
            if app.show_recommendations && app.recommendations.is_empty() {
                if let Some(ref jikan) = app.jikan_anime {
                    app.recommendations_loading = true;
                    vec![Cmd::FetchRecommendations(
                        jikan.mal_id,
                        app.current_session_id,
                    )]
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        }
        KeyCode::Enter | KeyCode::Char('p') => {
            if app.show_recommendations {
                if let Some(rec) = app
                    .recommendations
                    .get(app.recommendations_selected)
                    .cloned()
                {
                    app.loading = true;
                    app.current_session_id += 1;
                    let sid = app.current_session_id;
                    vec![Cmd::FetchAnimeDetail {
                        provider_id: None,
                        title: rec.entry.title.clone().unwrap_or_default(),
                        session_id: sid,
                    }]
                } else {
                    vec![]
                }
            } else if let Some(anime) = app.selected_anime.as_ref() {
                if let Some(ep) = app.episodes.get(app.episode_selected).cloned() {
                    vec![Cmd::LaunchPlayer(WatchEntry {
                        id: 0,
                        anime_id: anime.id.clone(),
                        title: anime.title.clone(),
                        episode: ep,
                        total_episodes: Some(anime.episode_count),
                        watched_at: chrono::Utc::now().to_rfc3339(),
                    })]
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        }
        _ => vec![],
    }
}
