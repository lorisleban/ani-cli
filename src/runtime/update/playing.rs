use super::super::msg::Cmd;
use crate::app::{App, Screen};
use crate::db::WatchEntry;
use crossterm::event::{KeyCode, KeyEvent};

pub fn update(app: &mut App, key: KeyEvent) -> Vec<Cmd> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.stop_active_watch_session();
            app.go_back();
            vec![]
        }
        KeyCode::Char('n') | KeyCode::Char('l') => {
            if app.next_episode() {
                if let Some(anime) = app.selected_anime.clone() {
                    let ep = app.episodes[app.episode_selected].clone();
                    vec![Cmd::LaunchPlayer(WatchEntry {
                        id: 0,
                        anime_id: anime.id,
                        title: anime.title,
                        episode: ep,
                        total_episodes: Some(anime.episode_count),
                        watched_at: chrono::Utc::now().to_rfc3339(),
                    })]
                } else {
                    vec![]
                }
            } else {
                app.toast("no more episodes", false);
                vec![]
            }
        }
        KeyCode::Char('p') | KeyCode::Char('h') => {
            if app.previous_episode() {
                if let Some(anime) = app.selected_anime.clone() {
                    let ep = app.episodes[app.episode_selected].clone();
                    vec![Cmd::LaunchPlayer(WatchEntry {
                        id: 0,
                        anime_id: anime.id,
                        title: anime.title,
                        episode: ep,
                        total_episodes: Some(anime.episode_count),
                        watched_at: chrono::Utc::now().to_rfc3339(),
                    })]
                } else {
                    vec![]
                }
            } else {
                app.toast("first episode", false);
                vec![]
            }
        }
        KeyCode::Char('r') => {
            if let Some(anime) = app.selected_anime.clone() {
                let ep = app.episodes[app.episode_selected].clone();
                vec![Cmd::LaunchPlayer(WatchEntry {
                    id: 0,
                    anime_id: anime.id,
                    title: anime.title,
                    episode: ep,
                    total_episodes: Some(anime.episode_count),
                    watched_at: chrono::Utc::now().to_rfc3339(),
                })]
            } else {
                vec![]
            }
        }
        KeyCode::Char('s') => {
            app.navigate(Screen::AnimeDetail);
            vec![]
        }
        _ => vec![],
    }
}
