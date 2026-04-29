#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerType {
    Mpv,
    Iina,
    Vlc,
}

impl PlayerType {
    pub fn name(&self) -> &str {
        match self {
            PlayerType::Mpv => "mpv",
            PlayerType::Iina => "iina",
            PlayerType::Vlc => "vlc",
        }
    }
}
