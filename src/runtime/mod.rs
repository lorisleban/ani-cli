mod event_loop;
mod terminal;

use crate::api::ApiClient;
use crate::app::{App, AppOptions};
use chrono::{DateTime, Utc};

pub async fn run(options: AppOptions) -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = terminal::TerminalSession::enter()?;
    let mut app = App::with_options(options);
    kick_update_check(&mut app);
    let api = ApiClient::new(app.mode);
    let result = event_loop::run_app(terminal.terminal_mut(), &mut app, api).await;
    app.stop_active_watch_session();

    if let Err(err) = result {
        eprintln!("error: {}", err);
    }

    Ok(())
}

fn kick_update_check(app: &mut App) {
    let last_checked = app
        .db
        .get_state("update_last_checked")
        .ok()
        .flatten()
        .and_then(|v| DateTime::parse_from_rfc3339(&v).ok())
        .map(|dt| dt.with_timezone(&Utc));
    if crate::update::should_check(last_checked) {
        app.update_check_in_progress = true;
    }
}
