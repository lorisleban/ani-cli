pub mod chrome;
pub mod cover_image;
pub mod detail;
pub mod genre;
pub mod help;
pub mod history;
pub mod home;
pub mod playing;
pub mod recommendations;
pub mod schedule;
pub mod search;
pub mod season;
pub mod top;

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
        Screen::SeasonBrowse => season::render(f, app),
        Screen::Schedule => schedule::render(f, app),
        Screen::TopAnime => top::render(f, app),
        Screen::GenreBrowse => genre::render(f, app),
    }
}
