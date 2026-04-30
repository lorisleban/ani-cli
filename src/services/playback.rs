use crate::domain::playback::PlayerType;
use crate::player;
use crate::player::LaunchResult;

pub trait PlayerLauncher {
    fn launch(
        &self,
        player: PlayerType,
        url: &str,
        title: &str,
        referer: Option<&str>,
        subtitle: Option<&str>,
        enable_activity_monitor: bool,
    ) -> Result<LaunchResult, String>;
}

pub struct ExternalPlayerLauncher;

impl PlayerLauncher for ExternalPlayerLauncher {
    fn launch(
        &self,
        player: PlayerType,
        url: &str,
        title: &str,
        referer: Option<&str>,
        subtitle: Option<&str>,
        enable_activity_monitor: bool,
    ) -> Result<LaunchResult, String> {
        player::launch_player(
            player,
            url,
            title,
            referer,
            subtitle,
            enable_activity_monitor,
        )
    }
}
