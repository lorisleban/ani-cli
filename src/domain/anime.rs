#[derive(Debug, Clone)]
pub struct AnimeResult {
    pub id: String,
    pub title: String,
    pub episode_count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct AnimePresenceMetadata {
    pub canonical_title: Option<String>,
    pub image_url: Option<String>,
    pub external_url: Option<String>,
    pub media_type: Option<String>,
    pub episode_count: Option<u32>,
    pub score: Option<f64>,
    pub season: Option<String>,
    pub year: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct EpisodeUrl {
    pub url: String,
    pub quality: String,
    pub referer: Option<String>,
    pub subtitle: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Sub,
    Dub,
}

impl Mode {
    pub fn as_str(&self) -> &str {
        match self {
            Mode::Sub => "sub",
            Mode::Dub => "dub",
        }
    }
}
