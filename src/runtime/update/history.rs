use super::super::msg::Cmd;
use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent};

pub fn update(app: &mut App, key: KeyEvent) -> Vec<Cmd> {
    match key.code {
        KeyCode::Esc => {
            app.go_back();
            vec![]
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.history_selected > 0 {
                app.history_selected -= 1;
            }
            vec![]
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.history_selected + 1 < app.history.len() {
                app.history_selected += 1;
            }
            vec![]
        }
        KeyCode::Enter | KeyCode::Char('p') => {
            if let Some(entry) = app.history.get(app.history_selected).cloned() {
                // Set selected_anime for context
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
                vec![]
            }
        }
        KeyCode::Char('D') => {
            // Delete from history
            if let Some(entry) = app.history.get(app.history_selected) {
                let _ = app.db.delete_entry(entry.id);
                app.refresh_history();
            }
            vec![]
        }
        _ => vec![],
    }
}
