#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerType {
    Mpv,
    Iina,
    Vlc,
}

impl PlayerType {
    pub const fn all() -> [PlayerType; 3] {
        [PlayerType::Mpv, PlayerType::Iina, PlayerType::Vlc]
    }

    pub fn name(&self) -> &str {
        match self {
            PlayerType::Mpv => "mpv",
            PlayerType::Iina => "iina",
            PlayerType::Vlc => "vlc",
        }
    }
}
