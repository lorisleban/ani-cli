use super::super::msg::Cmd;
use crate::app::{App, Screen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn update(app: &mut App, key: KeyEvent) -> Vec<Cmd> {
    match key.code {
        KeyCode::Esc => {
            app.go_back();
            vec![]
        }
        KeyCode::Up if app.search_selected > 0 => {
            app.search_selected -= 1;
            vec![]
        }
        KeyCode::Down if app.search_selected + 1 < app.search_results.len() => {
            app.search_selected += 1;
            vec![]
        }
        KeyCode::Backspace => {
            app.search_input.pop();
            // Trigger search immediately or via debounce logic
            // In the reactor, we can just return a Cmd if we want,
            // but usually we want to debounce.
            // For now, let's just trigger it if length >= 2
            if app.search_input.len() >= 2 {
                app.current_session_id += 1;
                vec![Cmd::SearchAnime(
                    app.search_input.clone(),
                    app.current_session_id,
                )]
            } else {
                app.search_results.clear();
                vec![]
            }
        }
        KeyCode::Enter => {
            if let Some(result) = app.search_results.get(app.search_selected).cloned() {
                app.selected_anime = Some(result.clone());
                app.episodes.clear();
                app.recommendations.clear();
                app.episodes_loading = true;
                app.jikan_loading = true;
                app.current_session_id += 1;
                let sid = app.current_session_id;
                app.navigate(Screen::AnimeDetail);
                vec![Cmd::FetchAnimeDetail {
                    provider_id: Some(result.id),
                    title: result.title,
                    session_id: sid,
                }]
            } else {
                vec![]
            }
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.search_input.push(c);
            if app.search_input.len() >= 2 {
                app.current_session_id += 1;
                vec![Cmd::SearchAnime(
                    app.search_input.clone(),
                    app.current_session_id,
                )]
            } else {
                vec![]
            }
        }
        _ => vec![],
    }
}
