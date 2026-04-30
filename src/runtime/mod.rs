mod event_loop;
mod terminal;

use crate::api::ApiClient;
use crate::app::{App, AppOptions};

pub async fn run(options: AppOptions) -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = terminal::TerminalSession::enter()?;
    let mut app = App::with_options(options);
    let api = ApiClient::new(app.mode);
    let result = event_loop::run_app(terminal.terminal_mut(), &mut app, api).await;
    app.stop_active_watch_session();

    if let Err(err) = result {
        eprintln!("error: {}", err);
    }

    Ok(())
}
