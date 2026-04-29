mod event_loop;
mod terminal;

use crate::api::ApiClient;
use crate::app::App;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = terminal::TerminalSession::enter()?;
    let mut app = App::new();
    let api = ApiClient::new(app.mode);
    let result = event_loop::run_app(terminal.terminal_mut(), &mut app, api).await;

    if let Err(err) = result {
        eprintln!("error: {}", err);
    }

    Ok(())
}
