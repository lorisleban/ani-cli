use std::fmt::Write as _;
use std::io;

use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};

use crate::api::Mode;
use crate::app::AppOptions;
use crate::db::Database;
use crate::player::PlayerType;
use crate::update::{self, UpgradeOutcome};

#[derive(Debug, Parser)]
#[command(
    name = "ani-cli",
    version,
    about = "A TUI anime streaming client",
    long_about = "A TUI anime streaming client for searching shows, tracking history, and launching playback in an external player."
)]
pub struct Cli {
    #[arg(long, global = true, value_enum)]
    player: Option<PlayerArg>,
    #[arg(long, global = true, value_enum)]
    mode: Option<ModeArg>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Inspect local setup and runtime prerequisites
    Doctor,
    /// Inspect application paths
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Check GitHub Releases for a newer version
    CheckUpdate,
    /// Upgrade from the latest GitHub release when this install is unmanaged
    Upgrade,
    /// Generate shell completions
    Completion {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// Print a resolved config/data path
    Path {
        #[arg(value_enum, default_value_t = ConfigPathArg::HistoryDb)]
        kind: ConfigPathArg,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ConfigPathArg {
    DataDir,
    HistoryDb,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum PlayerArg {
    Mpv,
    Iina,
    Vlc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ModeArg {
    Sub,
    Dub,
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Doctor) => {
            println!("{}", doctor_report());
            Ok(())
        }
        Some(Commands::Config { command }) => {
            match command {
                ConfigCommand::Path { kind } => println!("{}", config_path(kind)?),
            }
            Ok(())
        }
        Some(Commands::CheckUpdate) => {
            println!("{}", check_update_report().await?);
            Ok(())
        }
        Some(Commands::Upgrade) => {
            println!("{}", run_upgrade().await?);
            Ok(())
        }
        Some(Commands::Completion { shell }) => {
            let mut command = <Cli as clap::CommandFactory>::command();
            let name = command.get_name().to_string();
            generate(shell, &mut command, name, &mut io::stdout());
            Ok(())
        }
        None => {
            crate::runtime::run(AppOptions {
                player_type: cli.player.map(Into::into),
                mode: cli.mode.map(Into::into),
            })
            .await
        }
    }
}

fn doctor_report() -> String {
    let detected = PlayerType::detect();
    let history_db = Database::preferred_db_path_for_cli();
    let data_dir = Database::data_dir_for_cli();

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

    writeln!(&mut report, "data dir: {}", data_dir.display()).expect("write to string");
    writeln!(&mut report, "history db: {}", history_db.display()).expect("write to string");
    writeln!(
        &mut report,
        "api debug env: {}",
        std::env::var("ANI_CLI_DEBUG_API").unwrap_or_else(|_| "unset".to_string())
    )
    .expect("write to string");

    report
}

fn config_path(kind: ConfigPathArg) -> Result<String, String> {
    match kind {
        ConfigPathArg::DataDir => Ok(Database::data_dir_for_cli().display().to_string()),
        ConfigPathArg::HistoryDb => Ok(Database::preferred_db_path_for_cli().display().to_string()),
    }
}

async fn check_update_report() -> Result<String, String> {
    let summary = update::check_for_update().await?;
    if summary.update_available {
        Ok(format!(
            "update available: {} -> {} ({})",
            summary.current_version, summary.latest_version, summary.release_url
        ))
    } else {
        Ok(format!(
            "up to date: {} ({})",
            summary.current_version, summary.release_url
        ))
    }
}

async fn run_upgrade() -> Result<String, String> {
    match update::upgrade().await? {
        UpgradeOutcome::UpToDate { version } => Ok(format!("already on latest version: {version}")),
        UpgradeOutcome::Updated { version } => Ok(format!("updated to version {version}")),
        UpgradeOutcome::UsePackageManager { message } => Ok(message),
    }
}

impl From<PlayerArg> for PlayerType {
    fn from(value: PlayerArg) -> Self {
        match value {
            PlayerArg::Mpv => PlayerType::Mpv,
            PlayerArg::Iina => PlayerType::Iina,
            PlayerArg::Vlc => PlayerType::Vlc,
        }
    }
}

impl From<ModeArg> for Mode {
    fn from(value: ModeArg) -> Self {
        match value {
            ModeArg::Sub => Mode::Sub,
            ModeArg::Dub => Mode::Dub,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{doctor_report, Cli};
    use clap::{error::ErrorKind, Parser};

    #[test]
    fn parses_doctor_subcommand() {
        let cli = Cli::try_parse_from(["ani-cli", "doctor"]).expect("parse doctor");
        assert!(format!("{cli:?}").contains("Doctor"));
    }

    #[test]
    fn parses_global_mode_and_player_flags() {
        let cli = Cli::try_parse_from(["ani-cli", "--mode", "dub", "--player", "vlc"])
            .expect("parse overrides");
        let rendered = format!("{cli:?}");
        assert!(rendered.contains("Dub"));
        assert!(rendered.contains("Vlc"));
    }

    #[test]
    fn parses_completion_subcommand() {
        let cli = Cli::try_parse_from(["ani-cli", "completion", "powershell"]).expect("completion");
        assert!(format!("{cli:?}").contains("Completion"));
    }

    #[test]
    fn built_in_help_subcommand_is_available() {
        let err = Cli::try_parse_from(["ani-cli", "help"]).expect_err("help exits");
        assert_eq!(err.kind(), ErrorKind::DisplayHelp);
    }

    #[test]
    fn doctor_report_mentions_key_fields() {
        let report = doctor_report();
        assert!(report.contains("ani-cli doctor"));
        assert!(report.contains("version:"));
        assert!(report.contains("history db:"));
        assert!(report.contains("data dir:"));
    }
}
