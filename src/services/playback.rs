use crate::domain::playback::PlayerType;
use crate::player;

pub trait PlayerLauncher {
    fn launch(
        &self,
        player: PlayerType,
        url: &str,
        title: &str,
        referer: Option<&str>,
        subtitle: Option<&str>,
    ) -> Result<(), String>;
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
    ) -> Result<(), String> {
        player::launch_player(player, url, title, referer, subtitle)
    }
}
