use std::fmt::Write as _;

use clap::{Parser, Subcommand};

use crate::db::Database;
use crate::player::PlayerType;

#[derive(Debug, Parser)]
#[command(
    name = "ani-cli",
    version,
    about = "A TUI anime streaming client",
    long_about = "A TUI anime streaming client for searching shows, tracking history, and launching playback in an external player."
)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Inspect local setup and runtime prerequisites
    Doctor,
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Doctor) => {
            println!("{}", doctor_report());
            Ok(())
        }
        None => crate::runtime::run().await,
    }
}

fn doctor_report() -> String {
    let detected = PlayerType::detect();
    let db_path = Database::resolve_db_path_for_cli()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|err| format!("unavailable ({err})"));

    let mut report = String::new();
    writeln!(&mut report, "ani-cli doctor").expect("write to string");
    writeln!(&mut report, "version: {}", env!("CARGO_PKG_VERSION")).expect("write to string");
    writeln!(&mut report, "detected player: {}", detected.name()).expect("write to string");

    for player in PlayerType::all() {
        let status = match crate::player::detected_player_path(player) {
            Some(path) => format!("ok ({})", path.display()),
            None => "missing".to_string(),
        };
        writeln!(&mut report, "player {:>4}: {}", player.name(), status).expect("write to string");
    }

    writeln!(&mut report, "history db: {}", db_path).expect("write to string");
    writeln!(
        &mut report,
        "api debug env: {}",
        std::env::var("ANI_CLI_DEBUG_API").unwrap_or_else(|_| "unset".to_string())
    )
    .expect("write to string");

    report
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::{error::ErrorKind, Parser};

    #[test]
    fn parses_doctor_subcommand() {
        let cli = Cli::try_parse_from(["ani-cli", "doctor"]).expect("parse doctor");
        assert!(format!("{cli:?}").contains("Doctor"));
    }

    #[test]
    fn built_in_help_subcommand_is_available() {
        let err = Cli::try_parse_from(["ani-cli", "help"]).expect_err("help exits");
        assert_eq!(err.kind(), ErrorKind::DisplayHelp);
    }

    #[test]
    fn doctor_report_mentions_key_fields() {
        let report = super::doctor_report();
        assert!(report.contains("ani-cli doctor"));
        assert!(report.contains("version:"));
        assert!(report.contains("history db:"));
    }
}
