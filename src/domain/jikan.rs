use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JikanPaginated<T> {
    pub data: Vec<T>,
    pub pagination: JikanPagination,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JikanPagination {
    pub last_visible_page: u32,
    pub has_next_page: bool,
    pub current_page: u32,
    pub items: JikanPaginationItems,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JikanPaginationItems {
    pub count: u32,
    pub total: u32,
    pub per_page: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JikanAnime {
    pub mal_id: u32,
    pub url: String,
    pub images: JikanImages,
    pub title: String,
    #[serde(default)]
    pub title_english: Option<String>,
    #[serde(default)]
    pub title_japanese: Option<String>,
    #[serde(default)]
    pub title_synonyms: Vec<String>,
    #[serde(default, rename = "type")]
    pub anime_type: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub episodes: Option<u32>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub airing: bool,
    #[serde(default)]
    pub aired: JikanAired,
    #[serde(default)]
    pub duration: Option<String>,
    #[serde(default)]
    pub rating: Option<String>,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default)]
    pub scored_by: Option<u32>,
    #[serde(default)]
    pub rank: Option<u32>,
    #[serde(default)]
    pub popularity: Option<u32>,
    #[serde(default)]
    pub members: Option<u32>,
    #[serde(default)]
    pub favorites: Option<u32>,
    #[serde(default)]
    pub synopsis: Option<String>,
    #[serde(default)]
    pub season: Option<String>,
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub broadcast: JikanBroadcast,
    #[serde(default)]
    pub producers: Vec<JikanMalItem>,
    #[serde(default)]
    pub licensors: Vec<JikanMalItem>,
    #[serde(default)]
    pub studios: Vec<JikanMalItem>,
    #[serde(default)]
    pub genres: Vec<JikanMalItem>,
    #[serde(default)]
    pub explicit_genres: Vec<JikanMalItem>,
    #[serde(default)]
    pub themes: Vec<JikanMalItem>,
    #[serde(default)]
    pub demographics: Vec<JikanMalItem>,
    #[serde(default)]
    pub trailer: Option<JikanTrailer>,
    #[serde(default)]
    pub streaming: Vec<JikanStreaming>,
    #[serde(default)]
    pub relations: Vec<JikanRelation>,
    #[serde(default)]
    pub theme: Option<JikanTheme>,
    #[serde(default)]
    pub background: Option<String>,
    #[serde(default)]
    pub approved: bool,
}

impl JikanAnime {
    pub fn display_title(&self) -> &str {
        self.title_english
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.title)
    }

    pub fn studio_names(&self) -> Vec<&str> {
        self.studios.iter().map(|s| s.name.as_str()).collect()
    }

    pub fn genre_names(&self) -> Vec<&str> {
        self.genres.iter().map(|g| g.name.as_str()).collect()
    }

    pub fn theme_names(&self) -> Vec<&str> {
        self.themes.iter().map(|t| t.name.as_str()).collect()
    }

    pub fn demographic_names(&self) -> Vec<&str> {
        self.demographics.iter().map(|d| d.name.as_str()).collect()
    }

    pub fn streaming_platforms(&self) -> Vec<&str> {
        self.streaming.iter().map(|s| s.name.as_str()).collect()
    }

    pub fn is_currently_airing(&self) -> bool {
        self.airing
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JikanImages {
    #[serde(default)]
    pub jpg: JikanImageUrls,
    #[serde(default)]
    pub webp: JikanImageUrls,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JikanImageUrls {
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub small_image_url: Option<String>,
    #[serde(default)]
    pub large_image_url: Option<String>,
}

impl JikanImages {
    pub fn best_url(&self) -> Option<&str> {
        self.webp
            .large_image_url
            .as_deref()
            .or(self.jpg.large_image_url.as_deref())
            .or(self.webp.image_url.as_deref())
            .or(self.jpg.image_url.as_deref())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JikanAired {
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub string: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JikanBroadcast {
    #[serde(default)]
    pub day: Option<String>,
    #[serde(default)]
    pub time: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub string: Option<String>,
}

impl fmt::Display for JikanBroadcast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.string {
            Some(s) if !s.is_empty() => write!(f, "{}", s),
            _ => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JikanMalItem {
    #[serde(default)]
    pub mal_id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JikanTrailer {
    #[serde(default)]
    pub youtube_id: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub embed_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JikanStreaming {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JikanRelation {
    #[serde(default)]
    pub relation: String,
    #[serde(default)]
    pub entry: Vec<JikanRelationEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JikanRelationEntry {
    pub mal_id: u32,
    #[serde(default, rename = "type")]
    pub entry_type: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JikanTheme {
    #[serde(default)]
    pub openings: Vec<String>,
    #[serde(default)]
    pub endings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JikanSeasonInfo {
    pub year: i32,
    pub seasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JikanGenre {
    pub mal_id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JikanRecommendation {
    #[serde(default)]
    pub entry: JikanRecommendationEntry,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub votes: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JikanRecommendationEntry {
    #[serde(default)]
    pub mal_id: u32,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub images: Option<JikanImages>,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JikanCharacter {
    #[serde(default)]
    pub character: JikanMalItem,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub voice_actors: Vec<JikanVoiceActor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JikanVoiceActor {
    #[serde(default)]
    pub person: JikanMalItem,
    #[serde(default)]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MalIdMapping {
    pub allanime_id: String,
    pub mal_id: u32,
    pub confidence: f64,
    pub confirmed: bool,
}
