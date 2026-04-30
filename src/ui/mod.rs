pub mod chrome;
pub mod detail;
pub mod help;
pub mod history;
pub mod home;
pub mod playing;
pub mod search;

use ratatui::Frame;

use crate::app::{App, Screen};

pub fn render(f: &mut Frame, app: &App) {
    match app.screen {
        Screen::Home => home::render(f, app),
        Screen::Search => search::render(f, app),
        Screen::AnimeDetail => detail::render(f, app),
        Screen::WatchHistory => history::render(f, app),
        Screen::NowPlaying => playing::render(f, app),
        Screen::Help => help::render(f, app),
    }
}
