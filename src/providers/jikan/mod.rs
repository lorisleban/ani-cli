use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde::de::DeserializeOwned;
use tokio::sync::Mutex;
use urlencoding::encode;

use crate::domain::anime::AnimePresenceMetadata;
use crate::domain::jikan::*;
use crate::persistence::sqlite_history::Database;

const PRIMARY_API: &str = "https://api.jikan.moe/v4";
const MIRROR_API: &str = "https://jikan.lorisleban.uk/v4";

const RATE_LIMIT_REFILL_MS: u64 = 350;
const RATE_LIMIT_BURST: u64 = 5;

const CACHE_TTL_ANIME_HOURS: u32 = 168;
const CACHE_TTL_SEASON_HOURS: u32 = 24;
const CACHE_TTL_SCHEDULE_HOURS: u32 = 12;
const CACHE_TTL_SEARCH_HOURS: u32 = 48;
const CACHE_TTL_GENRE_HOURS: u32 = 720;
const CACHE_TTL_RECOMMENDATIONS_HOURS: u32 = 168;
const CACHE_TTL_TOP_HOURS: u32 = 24;

#[derive(Clone)]
pub struct JikanClient {
    client: Client,
    db: Arc<Mutex<Database>>,
    tokens: Arc<AtomicU64>,
    last_refill: Arc<Mutex<tokio::time::Instant>>,
}

impl JikanClient {
    pub fn new(db: Database) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create Jikan HTTP client");

        Self {
            client,
            db: Arc::new(Mutex::new(db)),
            tokens: Arc::new(AtomicU64::new(RATE_LIMIT_BURST)),
            last_refill: Arc::new(Mutex::new(tokio::time::Instant::now())),
        }
    }

    async fn acquire_rate_limit(&self) {
        loop {
            let current = self.tokens.load(Ordering::Relaxed);
            if current > 0 {
                if self
                    .tokens
                    .compare_exchange(current, current - 1, Ordering::SeqCst, Ordering::Relaxed)
                    .is_ok()
                {
                    return;
                }
                continue;
            }

            let mut last = self.last_refill.lock().await;
            let elapsed = last.elapsed();
            if elapsed >= Duration::from_millis(RATE_LIMIT_REFILL_MS) {
                let refill = (elapsed.as_millis() / RATE_LIMIT_REFILL_MS as u128) as u64;
                let new_tokens = (current + refill).min(RATE_LIMIT_BURST);
                self.tokens.store(new_tokens, Ordering::SeqCst);
                *last = tokio::time::Instant::now();
                if new_tokens > 0 {
                    self.tokens.fetch_sub(1, Ordering::SeqCst);
                    return;
                }
            }
            drop(last);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    fn endpoint_url(base: &str, endpoint: &str) -> String {
        format!(
            "{}/{}",
            base.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        )
    }

    fn env_override() -> Option<String> {
        std::env::var("ANI_CLI_METADATA_API_BASE").ok()
    }

    fn api_bases() -> Vec<&'static str> {
        if let Some(base) = Self::env_override() {
            vec![Box::leak(base.into_boxed_str())]
        } else {
            vec![PRIMARY_API, MIRROR_API]
        }
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        cache_ttl: Option<u32>,
    ) -> Result<T, String> {
        if cache_ttl.is_some() {
            let db = self.db.lock().await;
            if let Some(cached) = db.jikan_get_cached(endpoint) {
                if let Ok(parsed) = serde_json::from_str::<T>(&cached) {
                    return Ok(parsed);
                }
            }
        }

        self.acquire_rate_limit().await;

        let bases = Self::api_bases();
        let mut last_error = String::new();

        for base in &bases {
            let url = Self::endpoint_url(base, endpoint);
            match self.client.get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        self.tokens.store(0, Ordering::SeqCst);
                        continue;
                    }
                    if !status.is_success() {
                        last_error = format!("HTTP {} from {}", status, base);
                        continue;
                    }
                    let text = match response.text().await {
                        Ok(t) => t,
                        Err(e) => {
                            last_error = format!("Read error from {}: {}", base, e);
                            continue;
                        }
                    };
                    let parsed: T = match serde_json::from_str(&text) {
                        Ok(v) => v,
                        Err(e) => {
                            last_error = format!("Parse error from {}: {}", base, e);
                            continue;
                        }
                    };
                    if let Some(ttl) = cache_ttl {
                        let db = self.db.lock().await;
                        db.jikan_set_cached(endpoint, &text, ttl);
                    }
                    return Ok(parsed);
                }
                Err(e) => {
                    last_error = format!("Request to {} failed: {}", base, e);
                }
            }
        }

        Err(if last_error.is_empty() {
            "All Jikan API bases failed".to_string()
        } else {
            last_error
        })
    }

    pub async fn get_anime_full(&self, mal_id: u32) -> Result<JikanAnime, String> {
        let endpoint = format!("anime/{}/full", mal_id);
        let result: JikanAnime = self
            .get_json(&endpoint, Some(CACHE_TTL_ANIME_HOURS))
            .await?;
        Ok(result)
    }

    pub async fn search_anime(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<JikanPaginated<JikanAnime>, String> {
        let encoded = encode(query);
        let endpoint = format!("anime?q={}&limit={}", encoded, limit);
        self.get_json(&endpoint, Some(CACHE_TTL_SEARCH_HOURS)).await
    }

    pub async fn get_current_season(
        &self,
        page: u32,
    ) -> Result<JikanPaginated<JikanAnime>, String> {
        let endpoint = format!("seasons/now?page={}", page);
        self.get_json(&endpoint, Some(CACHE_TTL_SEASON_HOURS)).await
    }

    pub async fn get_season(
        &self,
        year: i32,
        season: &str,
        page: u32,
    ) -> Result<JikanPaginated<JikanAnime>, String> {
        let endpoint = format!("seasons/{}/{}?page={}", year, season, page);
        self.get_json(&endpoint, Some(CACHE_TTL_SEASON_HOURS)).await
    }

    pub async fn get_upcoming(&self, page: u32) -> Result<JikanPaginated<JikanAnime>, String> {
        let endpoint = format!("seasons/upcoming?page={}", page);
        self.get_json(&endpoint, Some(CACHE_TTL_SEASON_HOURS)).await
    }

    pub async fn get_schedule(
        &self,
        day: &str,
        page: u32,
    ) -> Result<JikanPaginated<JikanAnime>, String> {
        let endpoint = format!("schedules?filter={}&page={}", day, page);
        self.get_json(&endpoint, Some(CACHE_TTL_SCHEDULE_HOURS))
            .await
    }

    pub async fn get_top_anime(&self, page: u32) -> Result<JikanPaginated<JikanAnime>, String> {
        let endpoint = format!("top/anime?page={}", page);
        self.get_json(&endpoint, Some(CACHE_TTL_TOP_HOURS)).await
    }

    pub async fn get_genres(&self) -> Result<JikanPaginated<JikanGenre>, String> {
        let endpoint = "genres/anime".to_string();
        self.get_json(&endpoint, Some(CACHE_TTL_GENRE_HOURS)).await
    }

    pub async fn get_recommendations(
        &self,
        mal_id: u32,
    ) -> Result<JikanPaginated<JikanRecommendation>, String> {
        let endpoint = format!("anime/{}/recommendations", mal_id);
        self.get_json(&endpoint, Some(CACHE_TTL_RECOMMENDATIONS_HOURS))
            .await
    }

    pub async fn get_characters(
        &self,
        mal_id: u32,
    ) -> Result<JikanPaginated<JikanCharacter>, String> {
        let endpoint = format!("anime/{}/characters", mal_id);
        self.get_json(&endpoint, Some(CACHE_TTL_ANIME_HOURS)).await
    }

    pub async fn get_season_list(&self) -> Result<Vec<JikanSeasonInfo>, String> {
        let endpoint = "seasons".to_string();
        let result: JikanPaginated<JikanSeasonInfo> = self
            .get_json(&endpoint, Some(CACHE_TTL_GENRE_HOURS))
            .await?;
        Ok(result.data)
    }

    pub async fn fetch_presence_metadata(
        &self,
        title: &str,
        episode_count_hint: Option<u32>,
    ) -> Result<Option<AnimePresenceMetadata>, String> {
        let title = title.trim();
        if title.is_empty() {
            return Ok(None);
        }

        let results = self.search_anime(title, 5).await?;
        let best = results.data.into_iter().max_by(|a, b| {
            let score_a = match_confidence(title, a, episode_count_hint);
            let score_b = match_confidence(title, b, episode_count_hint);
            score_a
                .partial_cmp(&score_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let anime = match best {
            Some(a) => a,
            None => return Ok(None),
        };

        let confidence = match_confidence(title, &anime, episode_count_hint);
        if confidence < 0.3 {
            return Ok(None);
        }

        Ok(Some(AnimePresenceMetadata {
            canonical_title: Some(anime.display_title().to_string()),
            image_url: anime.images.best_url().map(str::to_string),
            external_url: Some(anime.url),
            media_type: anime.anime_type,
            episode_count: anime.episodes.or(episode_count_hint),
            score: anime.score,
            season: anime.season.as_deref().map(capitalize_first),
            year: anime.year,
        }))
    }

    pub async fn resolve_mal_id(
        &self,
        allanime_id: &str,
        title: &str,
        episode_count_hint: Option<u32>,
    ) -> Result<Option<MalIdMapping>, String> {
        {
            let db = self.db.lock().await;
            if let Some(mapping) = db.jikan_get_mal_mapping(allanime_id) {
                return Ok(Some(mapping));
            }
        }

        let results = self.search_anime(title, 5).await?;
        let mut scored: Vec<(f64, &JikanAnime)> = results
            .data
            .iter()
            .map(|a| (match_confidence(title, a, episode_count_hint), a))
            .filter(|(score, _)| *score >= 0.3)
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let (confidence, best) = match scored.first() {
            Some(pair) => *pair,
            None => return Ok(None),
        };

        let confirmed = confidence >= 0.8;
        let mapping = MalIdMapping {
            allanime_id: allanime_id.to_string(),
            mal_id: best.mal_id,
            confidence,
            confirmed,
        };

        {
            let db = self.db.lock().await;
            db.jikan_set_mal_id(allanime_id, best.mal_id, confidence, confirmed);
        }

        Ok(Some(mapping))
    }
}

fn match_confidence(title: &str, anime: &JikanAnime, episode_count_hint: Option<u32>) -> f64 {
    let query = title.to_lowercase();
    let canonical = anime.display_title().to_lowercase();
    let original = anime.title.to_lowercase();

    let name_score = if query == canonical || query == original {
        1.0
    } else if canonical.contains(&query) || query.contains(&canonical) {
        0.85
    } else if original.contains(&query) || query.contains(&original) {
        0.8
    } else {
        let dist = levenshtein(&query, &canonical).min(levenshtein(&query, &original));
        let max_len = query.len().max(canonical.len().max(original.len())) as f64;
        if max_len == 0.0 {
            0.0
        } else {
            (1.0 - (dist as f64 / max_len)).max(0.0) * 0.7
        }
    };

    let mut bonus = 0.0;
    if let (Some(hint), Some(episodes)) = (episode_count_hint, anime.episodes) {
        if hint == episodes {
            bonus += 0.1;
        } else if hint > 0 && episodes > 0 {
            let ratio = (hint as f64 / episodes as f64).min(episodes as f64 / hint as f64);
            bonus += ratio * 0.05;
        }
    }

    if anime.approved {
        bonus += 0.05;
    }

    (name_score + bonus).min(1.0)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a_len = a.chars().count();
    let b_len = b.chars().count();
    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr = vec![0; b_len + 1];

    for (i, ca) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            curr[j + 1] = if ca == cb {
                prev[j]
            } else {
                prev[j].min(curr[j]).min(prev[j + 1]) + 1
            };
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b_len]
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{capitalize_first, levenshtein, match_confidence};
    use crate::domain::jikan::*;

    fn test_anime(
        mal_id: u32,
        title: &str,
        title_english: Option<&str>,
        episodes: Option<u32>,
    ) -> JikanAnime {
        JikanAnime {
            mal_id,
            url: String::new(),
            images: JikanImages::default(),
            title: title.to_string(),
            title_english: title_english.map(str::to_string),
            title_japanese: None,
            title_synonyms: vec![],
            anime_type: Some("TV".to_string()),
            source: None,
            episodes,
            status: None,
            airing: false,
            aired: JikanAired::default(),
            duration: None,
            rating: None,
            score: None,
            scored_by: None,
            rank: None,
            popularity: None,
            members: None,
            favorites: None,
            synopsis: None,
            season: None,
            year: None,
            broadcast: JikanBroadcast::default(),
            producers: vec![],
            licensors: vec![],
            studios: vec![],
            genres: vec![],
            explicit_genres: vec![],
            themes: vec![],
            demographics: vec![],
            trailer: None,
            streaming: vec![],
            relations: vec![],
            theme: None,
            background: None,
            approved: true,
        }
    }

    #[test]
    fn exact_match_scores_highest() {
        let anime = test_anime(1, "One Piece", Some("One Piece"), Some(1100));
        let score = match_confidence("One Piece", &anime, Some(1100));
        assert!(score >= 0.99, "expected >= 0.99, got {}", score);
    }

    #[test]
    fn substring_match_scores_well() {
        let anime = test_anime(1, "One Piece", Some("One Piece"), None);
        let score = match_confidence("one piece", &anime, None);
        assert!(score >= 0.85, "expected >= 0.85, got {}", score);
    }

    #[test]
    fn mismatched_episode_count_reduces_bonus() {
        let anime = test_anime(1, "Attack on Titan", Some("Attack on Titan"), Some(25));
        let matching = match_confidence("attack titan", &anime, Some(25));
        let mismatching = match_confidence("attack titan", &anime, Some(13));
        assert!(
            matching > mismatching,
            "matching {} should > mismatching {}",
            matching,
            mismatching
        );
    }

    #[test]
    fn levenshtein_basic() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", "abc"), 0);
    }

    #[test]
    fn capitalize_first_works() {
        assert_eq!(capitalize_first("spring"), "Spring");
        assert_eq!(capitalize_first(""), "");
    }
}
