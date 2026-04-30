use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

const ALLANIME_API: &str = "https://api.allanime.day";
const ALLANIME_BASE: &str = "allanime.day";
const ALLANIME_REFR: &str = "https://allmanga.to";
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/121.0";

#[derive(Debug, Clone)]
pub struct AnimeResult {
    pub id: String,
    pub title: String,
    pub episode_count: u32,
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

pub struct ApiClient {
    client: Client,
    pub mode: Mode,
}

impl ApiClient {
    pub fn new(mode: Mode) -> Self {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");
        Self { client, mode }
    }

    pub async fn search_anime(&self, query: &str) -> Result<Vec<AnimeResult>, String> {
        let search_gql = r#"query( $search: SearchInput $limit: Int $page: Int $translationType: VaildTranslationTypeEnumType $countryOrigin: VaildCountryOriginEnumType ) { shows( search: $search limit: $limit page: $page translationType: $translationType countryOrigin: $countryOrigin ) { edges { _id name availableEpisodes __typename } }}"#;

        let variables = serde_json::json!({
            "search": {
                "allowAdult": false,
                "allowUnknown": false,
                "query": query
            },
            "limit": 40,
            "page": 1,
            "translationType": self.mode.as_str(),
            "countryOrigin": "ALL"
        });

        let payload = serde_json::json!({
            "variables": variables,
            "query": search_gql
        });

        let resp = self
            .client
            .post(format!("{}/api", ALLANIME_API))
            .header("Referer", ALLANIME_REFR)
            .header("Content-Type", "application/json")
            .body(payload.to_string())
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let json: Value =
            serde_json::from_str(&resp).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let mut results = Vec::new();
        if let Some(edges) = json
            .pointer("/data/shows/edges")
            .and_then(|v| v.as_array())
        {
            for edge in edges {
                let id = edge
                    .get("_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let title = edge
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let episode_count = edge
                    .pointer(&format!("/availableEpisodes/{}", self.mode.as_str()))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;

                if !id.is_empty() && !title.is_empty() && episode_count > 0 {
                    results.push(AnimeResult {
                        id,
                        title,
                        episode_count,
                    });
                }
            }
        }

        Ok(results)
    }

    pub async fn episodes_list(&self, show_id: &str) -> Result<Vec<String>, String> {
        let episodes_gql = r#"query ($showId: String!) { show( _id: $showId ) { _id availableEpisodesDetail }}"#;

        let variables = serde_json::json!({ "showId": show_id });

        let payload = serde_json::json!({
            "variables": variables,
            "query": episodes_gql
        });

        let resp = self
            .client
            .post(format!("{}/api", ALLANIME_API))
            .header("Referer", ALLANIME_REFR)
            .header("Content-Type", "application/json")
            .body(payload.to_string())
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let json: Value =
            serde_json::from_str(&resp).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let mut episodes = Vec::new();
        if let Some(ep_detail) = json.pointer("/data/show/availableEpisodesDetail") {
            if let Some(eps) = ep_detail
                .get(self.mode.as_str())
                .and_then(|v| v.as_array())
            {
                for ep in eps {
                    if let Some(ep_str) = ep.as_str() {
                        episodes.push(ep_str.to_string());
                    }
                }
            }
        }

        // Sort numerically
        episodes.sort_by(|a, b| {
            let a_num: f64 = a.parse().unwrap_or(0.0);
            let b_num: f64 = b.parse().unwrap_or(0.0);
            a_num.partial_cmp(&b_num).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(episodes)
    }

    /// Faithfully replicates the shell script's get_episode_url logic:
    /// 1. Fetch sourceUrls from GraphQL
    /// 2. Parse each sourceUrl, match provider names (Default, Luf-Mp4, S-mp4, Yt-mp4)
    /// 3. Decode the hex-encoded provider path
    /// 4. Fetch https://allanime.day/<decoded_path> to get actual streaming links
    /// 5. Parse links from JSON response (link + resolutionStr fields)
    /// 6. Select best quality
    pub async fn get_episode_url(
        &self,
        show_id: &str,
        episode: &str,
        quality: &str,
    ) -> Result<EpisodeUrl, String> {
        let episode_gql = r#"query ($showId: String!, $translationType: VaildTranslationTypeEnumType!, $episodeString: String!) { episode( showId: $showId translationType: $translationType episodeString: $episodeString ) { episodeString sourceUrls }}"#;

        let variables = serde_json::json!({
            "showId": show_id,
            "translationType": self.mode.as_str(),
            "episodeString": episode
        });

        let payload = serde_json::json!({
            "variables": variables,
            "query": episode_gql
        });

        let resp = self
            .client
            .post(format!("{}/api", ALLANIME_API))
            .header("Referer", ALLANIME_REFR)
            .header("Content-Type", "application/json")
            .body(payload.to_string())
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        // The shell script does: tr '{}' '\n' | sed ... to extract sourceName:encodedUrl pairs
        // We parse the JSON properly instead
        let json: Value =
            serde_json::from_str(&resp).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let source_urls = json
            .pointer("/data/episode/sourceUrls")
            .and_then(|v| v.as_array())
            .ok_or("No source URLs found")?;

        // Extract provider entries: (sourceName, decoded_provider_path)
        let mut providers: Vec<(String, String)> = Vec::new();

        for source in source_urls {
            let source_url = source
                .get("sourceUrl")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let source_name = source
                .get("sourceName")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if source_url.starts_with("--") {
                let decoded = decode_provider_url(&source_url[2..]);
                if !decoded.is_empty() {
                    providers.push((source_name.to_string(), decoded));
                }
            }
        }

        // Shell script checks providers in order: Default, Yt-mp4, S-mp4, Luf-Mp4
        // Try all providers concurrently with timeout, collect all links
        let mut all_links: Vec<(String, String, Option<String>)> = Vec::new(); // (quality, url, referer)

        for (_provider_name, provider_path) in &providers {
            // The shell uses: curl -e "$allanime_refr" -s "https://${allanime_base}$provider_id"
            let provider_url = format!("https://{}{}", ALLANIME_BASE, provider_path);

            match self.fetch_provider_links(&provider_url).await {
                Ok(links) => {
                    for (qual, url, refr) in links {
                        all_links.push((qual, url, refr));
                    }
                }
                Err(_) => {
                    // Provider failed, try next
                    continue;
                }
            }
        }

        if all_links.is_empty() {
            return Err("No valid streaming links found".to_string());
        }

        // Sort by quality (highest first)
        all_links.sort_by(|a, b| {
            let a_num: u32 = a.0.trim_end_matches('p').parse().unwrap_or(0);
            let b_num: u32 = b.0.trim_end_matches('p').parse().unwrap_or(0);
            b_num.cmp(&a_num)
        });

        // Select quality
        let selected = match quality {
            "best" => all_links.first(),
            "worst" => all_links.last(),
            q => all_links
                .iter()
                .find(|(qual, _, _)| qual.contains(q))
                .or(all_links.first()),
        };

        match selected {
            Some((qual, url, refr)) => Ok(EpisodeUrl {
                url: url.clone(),
                quality: qual.clone(),
                referer: refr.clone().or_else(|| Some(ALLANIME_REFR.to_string())),
                subtitle: None,
            }),
            None => Err("No matching quality found".to_string()),
        }
    }

    /// Fetches a provider endpoint and extracts streaming links.
    /// Replicates the shell script's get_links() function.
    async fn fetch_provider_links(
        &self,
        url: &str,
    ) -> Result<Vec<(String, String, Option<String>)>, String> {
        let resp = self
            .client
            .get(url)
            .header("Referer", ALLANIME_REFR)
            .send()
            .await
            .map_err(|e| format!("Provider request failed: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read provider response: {}", e))?;

        let mut links: Vec<(String, String, Option<String>)> = Vec::new();

        // Try to parse as JSON
        if let Ok(json) = serde_json::from_str::<Value>(&resp) {
            // Pattern 1: JSON with "links" array containing {link, resolutionStr}
            // This is the most common pattern (used by Default/wixmp, Luf-Mp4/hianime)
            if let Some(links_arr) = json.get("links").and_then(|v| v.as_array()) {
                for link_obj in links_arr {
                    let link = link_obj
                        .get("link")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let resolution = link_obj
                        .get("resolutionStr")
                        .and_then(|v| v.as_str())
                        .unwrap_or("auto");
                    let _hls_info = link_obj.get("hls");

                    if !link.is_empty() {
                        // Check if this is a master m3u8 link — extract the Referer from response
                        let refr = if link.contains("master.m3u8") || link.contains(".m3u8") {
                            // Check for Referer in the response
                            json.get("links")
                                .and_then(|v| v.as_array())
                                .and_then(|arr| arr.iter().find_map(|obj| {
                                    obj.get("Referer").and_then(|v| v.as_str()).map(|s| s.to_string())
                                }))
                                .or_else(|| {
                                    // Try rawUrls or headers for the referer
                                    json.pointer("/headers/Referer")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string())
                                })
                        } else {
                            None
                        };

                        links.push((resolution.to_string(), link.to_string(), refr));
                    }
                }
            }
        }

        // If the response isn't JSON or has no links array, check if it's a redirect/direct URL
        if links.is_empty() {
            // Check if the URL itself is a direct video link
            if url.contains(".mp4") || url.contains(".m3u8") {
                links.push(("auto".to_string(), url.to_string(), None));
            }
        }

        Ok(links)
    }
}

/// Decode the AllAnime provider URL cipher (port of shell script's hex decode)
fn decode_provider_url(encoded: &str) -> String {
    let hex_map: HashMap<&str, &str> = [
        ("79", "A"), ("7a", "B"), ("7b", "C"), ("7c", "D"), ("7d", "E"),
        ("7e", "F"), ("7f", "G"), ("70", "H"), ("71", "I"), ("72", "J"),
        ("73", "K"), ("74", "L"), ("75", "M"), ("76", "N"), ("77", "O"),
        ("68", "P"), ("69", "Q"), ("6a", "R"), ("6b", "S"), ("6c", "T"),
        ("6d", "U"), ("6e", "V"), ("6f", "W"), ("60", "X"), ("61", "Y"),
        ("62", "Z"), ("59", "a"), ("5a", "b"), ("5b", "c"), ("5c", "d"),
        ("5d", "e"), ("5e", "f"), ("5f", "g"), ("50", "h"), ("51", "i"),
        ("52", "j"), ("53", "k"), ("54", "l"), ("55", "m"), ("56", "n"),
        ("57", "o"), ("48", "p"), ("49", "q"), ("4a", "r"), ("4b", "s"),
        ("4c", "t"), ("4d", "u"), ("4e", "v"), ("4f", "w"), ("40", "x"),
        ("41", "y"), ("42", "z"), ("08", "0"), ("09", "1"), ("0a", "2"),
        ("0b", "3"), ("0c", "4"), ("0d", "5"), ("0e", "6"), ("0f", "7"),
        ("00", "8"), ("01", "9"), ("15", "-"), ("16", "."), ("67", "_"),
        ("46", "~"), ("02", ":"), ("17", "/"), ("07", "?"), ("1b", "#"),
        ("63", "["), ("65", "]"), ("78", "@"), ("19", "!"), ("1c", "$"),
        ("1e", "&"), ("10", "("), ("11", ")"), ("12", "*"), ("13", "+"),
        ("14", ","), ("03", ";"), ("05", "="), ("1d", "%"),
    ]
    .iter()
    .cloned()
    .collect();

    let mut decoded = String::new();
    let chars: Vec<char> = encoded.chars().collect();
    let mut i = 0;

    while i + 1 < chars.len() {
        let hex = format!("{}{}", chars[i], chars[i + 1]);
        if let Some(ch) = hex_map.get(hex.as_str()) {
            decoded.push_str(ch);
        }
        i += 2;
    }

    // Replace /clock with /clock.json (matches shell script behavior)
    decoded = decoded.replace("/clock", "/clock.json");

    decoded
}
