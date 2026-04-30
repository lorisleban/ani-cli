use aes::cipher::{KeyIvInit, StreamCipher};
use aes::Aes256;
use base64::prelude::*;
use ctr::Ctr128BE;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE};
use reqwest::Client;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;
use urlencoding::encode;

use crate::domain::anime::{AnimePresenceMetadata, AnimeResult, EpisodeUrl, Mode};

mod config;
mod transport;

use config::{
    ALLANIME_API, ALLANIME_BASE, ALLANIME_EP_QUERY_HASH, ALLANIME_ORIGIN, ALLANIME_REFR, USER_AGENT,
};
use transport::{curl_get, curl_post_api, is_captcha_response};

/// Returns the SHA256 hex digest of the key string used for decryption.
fn get_allanime_key() -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(b"Xot36i3lK3:v1");
    hasher.finalize().to_vec()
}

fn aes_256_ctr_decrypt(key: &[u8], ctr: &[u8; 16], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    type Aes256Ctr = Ctr128BE<Aes256>;

    let mut cipher =
        Aes256Ctr::new_from_slices(key, ctr).map_err(|e| format!("AES setup failed: {e}"))?;
    let mut plain = ciphertext.to_vec();
    cipher.apply_keystream(&mut plain);
    Ok(plain)
}

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    pub mode: Mode,
}

impl ApiClient {
    pub fn new(mode: Mode) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .default_headers(headers)
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(10))
            .http1_only()
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

        if std::env::var("ANI_CLI_DEBUG_API").ok().as_deref() == Some("1") {
            let _ = std::fs::write("/tmp/ani-cli-search-request.json", payload.to_string());
        }

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

        let resp = if is_captcha_response(&resp) {
            curl_post_api(payload.to_string(), Some("search"))?
        } else {
            resp
        };

        if std::env::var("ANI_CLI_DEBUG_API").ok().as_deref() == Some("1") {
            let _ = std::fs::write("/tmp/ani-cli-search.json", &resp);
            eprintln!("DEBUG search api response: {}", resp);
        }

        let json: Value =
            serde_json::from_str(&resp).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let mut results = Vec::new();
        if let Some(edges) = json.pointer("/data/shows/edges").and_then(|v| v.as_array()) {
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
        let episodes_gql =
            r#"query ($showId: String!) { show( _id: $showId ) { _id availableEpisodesDetail }}"#;

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

        let resp = if is_captcha_response(&resp) {
            curl_post_api(payload.to_string(), Some("episodes"))?
        } else {
            resp
        };

        let json: Value =
            serde_json::from_str(&resp).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let mut episodes = Vec::new();
        if let Some(ep_detail) = json.pointer("/data/show/availableEpisodesDetail") {
            if let Some(eps) = ep_detail.get(self.mode.as_str()).and_then(|v| v.as_array()) {
                for ep in eps {
                    if let Some(ep_str) = ep.as_str() {
                        episodes.push(ep_str.to_string());
                    }
                }
            }
        }

        sort_episode_numbers(&mut episodes);

        Ok(episodes)
    }

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

        let query_ext = serde_json::json!({
            "persistedQuery": {
                "version": 1,
                "sha256Hash": ALLANIME_EP_QUERY_HASH
            }
        });

        let payload = serde_json::json!({
            "variables": variables.clone(),
            "query": episode_gql
        });

        let api_url = format!(
            "{}/api?variables={}&extensions={}",
            ALLANIME_API,
            encode(&variables.to_string()),
            encode(&query_ext.to_string())
        );

        if std::env::var("ANI_CLI_DEBUG_API").ok().as_deref() == Some("1") {
            let _ = std::fs::write("/tmp/ani-cli-episode-request.json", payload.to_string());
            let _ = std::fs::write("/tmp/ani-cli-episode-persisted-url.txt", &api_url);
        }

        let mut resp = self
            .client
            .get(&api_url)
            .header("Referer", ALLANIME_REFR)
            .header("Origin", ALLANIME_ORIGIN)
            .send()
            .await
            .map_err(|e| format!("Persisted episode request failed: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read persisted episode response: {}", e))?;

        if is_captcha_response(&resp) {
            resp = curl_get(&api_url, Some("episode-persisted"), Some(ALLANIME_ORIGIN))?;
        }

        if resp.trim().is_empty() || !resp.contains("tobeparsed") {
            resp = self
                .client
                .post(format!("{}/api", ALLANIME_API))
                .header("Referer", ALLANIME_REFR)
                .header("Content-Type", "application/json")
                .body(payload.to_string())
                .send()
                .await
                .map_err(|e| format!("Fallback episode request failed: {}", e))?
                .text()
                .await
                .map_err(|e| format!("Failed to read fallback episode response: {}", e))?;

            if is_captcha_response(&resp) {
                resp = curl_post_api(payload.to_string(), Some("episode"))?;
            }
        }

        if std::env::var("ANI_CLI_DEBUG_API").ok().as_deref() == Some("1") {
            let _ = std::fs::write("/tmp/ani-cli-episode.json", &resp);
            eprintln!("DEBUG episode api response: {}", resp);
        }

        let mut provider_data = Vec::new();

        if let Some(tobeparsed) = extract_tobeparsed_from_raw(&resp) {
            provider_data = self.decode_tobeparsed(&tobeparsed)?;
        } else if let Ok(json) = serde_json::from_str::<Value>(&resp) {
            if let Some(tobeparsed) = json
                .pointer("/data/episode/tobeparsed")
                .and_then(|v| v.as_str())
            {
                provider_data = self.decode_tobeparsed(tobeparsed)?;
            } else if let Some(source_urls) = json
                .pointer("/data/episode/sourceUrls")
                .and_then(|v| v.as_array())
            {
                for source in source_urls {
                    let source_url = source
                        .get("sourceUrl")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let source_name = source
                        .get("sourceName")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if let Some(stripped) = source_url.strip_prefix("--") {
                        let decoded = decode_provider_url(stripped);
                        if !decoded.is_empty() {
                            provider_data.push((source_name.to_string(), decoded));
                        }
                    }
                }
            }
        }

        if provider_data.is_empty() {
            provider_data = extract_source_pairs_from_raw(&resp)
                .into_iter()
                .map(|(name, encoded)| (name, decode_provider_url(&encoded)))
                .filter(|(_, decoded)| !decoded.is_empty())
                .collect();
        }

        if provider_data.is_empty() {
            if std::env::var("ANI_CLI_DEBUG_API").ok().as_deref() == Some("1") {
                eprintln!("DEBUG no provider data. raw response len: {}", resp.len());
            }
            return Err("No source URLs found".to_string());
        }

        let mut all_links: Vec<(String, String, Option<String>)> = Vec::new();

        for (source_name, provider_path) in &provider_data {
            let provider_url = if provider_path.starts_with("https://") {
                provider_path.clone()
            } else {
                format!("https://{}{}", ALLANIME_BASE, provider_path)
            };
            match self.fetch_provider_links(source_name, &provider_url).await {
                Ok(links) => {
                    for (qual, url, refr) in links {
                        all_links.push((qual, url, refr));
                    }
                }
                Err(_) => continue,
            }
        }

        if all_links.is_empty() {
            return Err("No valid streaming links found".to_string());
        }

        all_links.sort_by(|a, b| {
            let a_num: u32 = a.0.trim_end_matches('p').parse().unwrap_or(0);
            let b_num: u32 = b.0.trim_end_matches('p').parse().unwrap_or(0);
            b_num.cmp(&a_num)
        });

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

    pub async fn fetch_presence_metadata(
        &self,
        title: &str,
        episode_count_hint: Option<u32>,
    ) -> Result<Option<AnimePresenceMetadata>, String> {
        let title = title.trim();
        if title.is_empty() {
            return Ok(None);
        }

        let encoded = encode(title);
        let mut candidates = Vec::new();
        if let Ok(base) = std::env::var("ANI_CLI_METADATA_API_BASE") {
            candidates.push(base);
        } else {
            candidates.push("https://jikan.lorisleban.uk/v4".to_string());
            candidates.push("https://api.jikan.moe/v4".to_string());
        }

        for base in candidates {
            let url = format!("{}/anime?q={encoded}&limit=1", base.trim_end_matches('/'));
            let response = match self.client.get(&url).send().await {
                Ok(response) => response,
                Err(_) => continue,
            };
            let text = match response.text().await {
                Ok(text) => text,
                Err(_) => continue,
            };
            let json: Value = match serde_json::from_str(&text) {
                Ok(json) => json,
                Err(_) => continue,
            };
            if let Some(metadata) = parse_presence_metadata(&json, episode_count_hint) {
                return Ok(Some(metadata));
            }
        }

        Ok(None)
    }

    fn decode_tobeparsed(&self, blob: &str) -> Result<Vec<(String, String)>, String> {
        let data = BASE64_STANDARD
            .decode(blob)
            .map_err(|e| format!("Base64 decode failed: {}", e))?;
        if data.len() < 13 + 16 {
            return Err("Payload too short".to_string());
        }

        let key = get_allanime_key();
        let iv = &data[1..13];
        let mut ctr = [0u8; 16];
        ctr[..12].copy_from_slice(iv);
        ctr[12] = 0;
        ctr[13] = 0;
        ctr[14] = 0;
        ctr[15] = 2; // iv + 00000002

        let ct_len = data.len() - 13 - 16;
        let ct = &data[13..13 + ct_len];

        let plain_bytes = aes_256_ctr_decrypt(&key, &ctr, ct)?;

        let plain = String::from_utf8_lossy(&plain_bytes);

        let mut results = Vec::new();
        // The plain text contains a series of JSON-like structures
        // Format: {"sourceUrl":"--<encoded>","sourceName":"<name>",...}
        // We can parse it roughly using splits since it's a stream of objects
        for part in plain.split('}') {
            if part.contains("\"sourceUrl\":\"--") {
                let url = part
                    .split("\"sourceUrl\":\"--")
                    .nth(1)
                    .and_then(|s| s.split('"').next())
                    .unwrap_or("");
                let name = part
                    .split("\"sourceName\":\"")
                    .nth(1)
                    .and_then(|s| s.split('"').next())
                    .unwrap_or("");
                if !url.is_empty() {
                    results.push((name.to_string(), decode_provider_url(url)));
                }
            }
        }

        if std::env::var("ANI_CLI_DEBUG_API").ok().as_deref() == Some("1") {
            eprintln!("DEBUG tobeparsed decoded entries: {}", results.len());
        }

        Ok(results)
    }

    async fn fetch_provider_links(
        &self,
        source_name: &str,
        url: &str,
    ) -> Result<Vec<(String, String, Option<String>)>, String> {
        // Some providers hand us a direct playable URL (for example tools.fast4speed.rsvp).
        // Do not fetch those as text first; just return them as stream candidates.
        if url.contains("tools.fast4speed.rsvp") {
            return Ok(vec![(
                "Yt".to_string(),
                url.to_string(),
                Some(ALLANIME_REFR.to_string()),
            )]);
        }

        if url.contains(".mp4") || url.contains(".m3u8") {
            return Ok(vec![("auto".to_string(), url.to_string(), None)]);
        }

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

        if source_name.contains("Fm-mp4") || source_name.to_lowercase().contains("filemoon") {
            links = parse_filemoon_links(&resp)?;
            return Ok(links);
        }

        if let Ok(json) = serde_json::from_str::<Value>(&resp) {
            if let Some(links_arr) = json.get("links").and_then(|v| v.as_array()) {
                for link_obj in links_arr {
                    let link = link_obj.get("link").and_then(|v| v.as_str()).unwrap_or("");
                    let resolution = link_obj
                        .get("resolutionStr")
                        .and_then(|v| v.as_str())
                        .unwrap_or("auto");
                    if !link.is_empty() {
                        let refr = if link.contains(".m3u8") {
                            link_obj
                                .get("Referer")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .or_else(|| {
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

        if links.is_empty() && (url.contains(".mp4") || url.contains(".m3u8")) {
            links.push(("auto".to_string(), url.to_string(), None));
        }

        Ok(links)
    }
}

fn decode_provider_url(encoded: &str) -> String {
    let hex_map: HashMap<&str, &str> = [
        ("79", "A"),
        ("7a", "B"),
        ("7b", "C"),
        ("7c", "D"),
        ("7d", "E"),
        ("7e", "F"),
        ("7f", "G"),
        ("70", "H"),
        ("71", "I"),
        ("72", "J"),
        ("73", "K"),
        ("74", "L"),
        ("75", "M"),
        ("76", "N"),
        ("77", "O"),
        ("68", "P"),
        ("69", "Q"),
        ("6a", "R"),
        ("6b", "S"),
        ("6c", "T"),
        ("6d", "U"),
        ("6e", "V"),
        ("6f", "W"),
        ("60", "X"),
        ("61", "Y"),
        ("62", "Z"),
        ("59", "a"),
        ("5a", "b"),
        ("5b", "c"),
        ("5c", "d"),
        ("5d", "e"),
        ("5e", "f"),
        ("5f", "g"),
        ("50", "h"),
        ("51", "i"),
        ("52", "j"),
        ("53", "k"),
        ("54", "l"),
        ("55", "m"),
        ("56", "n"),
        ("57", "o"),
        ("48", "p"),
        ("49", "q"),
        ("4a", "r"),
        ("4b", "s"),
        ("4c", "t"),
        ("4d", "u"),
        ("4e", "v"),
        ("4f", "w"),
        ("40", "x"),
        ("41", "y"),
        ("42", "z"),
        ("08", "0"),
        ("09", "1"),
        ("0a", "2"),
        ("0b", "3"),
        ("0c", "4"),
        ("0d", "5"),
        ("0e", "6"),
        ("0f", "7"),
        ("00", "8"),
        ("01", "9"),
        ("15", "-"),
        ("16", "."),
        ("67", "_"),
        ("46", "~"),
        ("02", ":"),
        ("17", "/"),
        ("07", "?"),
        ("1b", "#"),
        ("63", "["),
        ("65", "]"),
        ("78", "@"),
        ("19", "!"),
        ("1c", "$"),
        ("1e", "&"),
        ("10", "("),
        ("11", ")"),
        ("12", "*"),
        ("13", "+"),
        ("14", ","),
        ("03", ";"),
        ("05", "="),
        ("1d", "%"),
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
    decoded.replace("/clock", "/clock.json")
}

fn b64url_decode(input: &str) -> Result<Vec<u8>, String> {
    let mut normalized = input.replace('-', "+").replace('_', "/");
    while !normalized.len().is_multiple_of(4) {
        normalized.push('=');
    }
    BASE64_STANDARD
        .decode(normalized.as_bytes())
        .map_err(|e| format!("Base64url decode failed: {}", e))
}

fn parse_filemoon_links(resp: &str) -> Result<Vec<(String, String, Option<String>)>, String> {
    let json: Value =
        serde_json::from_str(resp).map_err(|e| format!("Invalid Filemoon response JSON: {}", e))?;

    let iv = json
        .get("iv")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing Filemoon iv".to_string())?;
    let payload = json
        .get("payload")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing Filemoon payload".to_string())?;
    let key_parts = json
        .get("key_parts")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Missing Filemoon key_parts".to_string())?;

    if key_parts.len() < 2 {
        return Err("Invalid Filemoon key_parts".to_string());
    }

    let kp1 = key_parts[0]
        .as_str()
        .ok_or_else(|| "Invalid Filemoon key_parts[0]".to_string())?;
    let kp2 = key_parts[1]
        .as_str()
        .ok_or_else(|| "Invalid Filemoon key_parts[1]".to_string())?;

    let key = [b64url_decode(kp1)?, b64url_decode(kp2)?].concat();
    if key.len() != 32 {
        return Err(format!("Invalid Filemoon key length: {}", key.len()));
    }

    let iv_bytes = b64url_decode(iv)?;
    if iv_bytes.len() != 12 {
        return Err(format!("Invalid Filemoon iv length: {}", iv_bytes.len()));
    }

    let mut ctr = [0u8; 16];
    ctr[..12].copy_from_slice(&iv_bytes);
    ctr[15] = 2;

    let payload_bytes = b64url_decode(payload)?;
    if payload_bytes.len() <= 16 {
        return Err("Filemoon payload too short".to_string());
    }

    let ct = &payload_bytes[..payload_bytes.len() - 16];
    let plain_bytes = aes_256_ctr_decrypt(&key, &ctr, ct)
        .map_err(|e| format!("Filemoon AES decryption failed: {}", e))?;
    let plain = String::from_utf8_lossy(&plain_bytes);

    let parsed: Value = serde_json::from_str(&plain)
        .map_err(|e| format!("Invalid Filemoon plaintext JSON: {}", e))?;
    let streams = parsed
        .as_array()
        .ok_or_else(|| "Filemoon plaintext is not an array".to_string())?;

    let mut links = Vec::new();
    for stream in streams {
        let url = stream
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let height = stream.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
        if !url.is_empty() {
            links.push((
                format!("{height}p"),
                url.replace("\\u0026", "&").replace("\\u003D", "="),
                None,
            ));
        }
    }

    links.sort_by(|a, b| {
        let a_num: u32 = a.0.trim_end_matches('p').parse().unwrap_or(0);
        let b_num: u32 = b.0.trim_end_matches('p').parse().unwrap_or(0);
        b_num.cmp(&a_num)
    });

    Ok(links)
}

fn extract_tobeparsed_from_raw(raw: &str) -> Option<String> {
    let needle = "\"tobeparsed\":\"";
    let start = raw.find(needle)? + needle.len();
    let rest = &raw[start..];
    let end = rest.find('"')?;
    let blob = &rest[..end];
    if blob.is_empty() {
        None
    } else {
        Some(blob.to_string())
    }
}

fn extract_source_pairs_from_raw(raw: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut cursor = raw;
    while let Some(url_pos) = cursor.find("\"sourceUrl\":\"--") {
        let after_url = &cursor[url_pos + "\"sourceUrl\":\"--".len()..];
        let url_end = match after_url.find('"') {
            Some(idx) => idx,
            None => break,
        };
        let encoded = &after_url[..url_end];
        let after_url = &after_url[url_end..];

        let name_pos = match after_url.find("\"sourceName\":\"") {
            Some(idx) => idx,
            None => {
                cursor = &after_url[1..];
                continue;
            }
        };
        let after_name = &after_url[name_pos + "\"sourceName\":\"".len()..];
        let name_end = match after_name.find('"') {
            Some(idx) => idx,
            None => break,
        };
        let name = &after_name[..name_end];

        if !encoded.is_empty() {
            out.push((name.to_string(), encoded.to_string()));
        }

        cursor = &after_name[name_end..];
    }
    out
}

fn sort_episode_numbers(episodes: &mut [String]) {
    episodes.sort_by(|a, b| {
        let a_num: f64 = a.parse().unwrap_or(0.0);
        let b_num: f64 = b.parse().unwrap_or(0.0);
        a_num
            .partial_cmp(&b_num)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

fn parse_presence_metadata(
    json: &Value,
    episode_count_hint: Option<u32>,
) -> Option<AnimePresenceMetadata> {
    let first = json.get("data")?.as_array()?.first()?;
    let canonical_title = first
        .get("title_english")
        .and_then(Value::as_str)
        .or_else(|| first.get("title").and_then(Value::as_str))
        .map(str::to_string);
    let image_url = first
        .pointer("/images/webp/large_image_url")
        .and_then(Value::as_str)
        .or_else(|| first.pointer("/images/jpg/large_image_url").and_then(Value::as_str))
        .or_else(|| first.pointer("/images/jpg/image_url").and_then(Value::as_str))
        .map(str::to_string);
    let external_url = first.get("url").and_then(Value::as_str).map(str::to_string);
    let media_type = first.get("type").and_then(Value::as_str).map(str::to_string);
    let episode_count = first
        .get("episodes")
        .and_then(Value::as_u64)
        .map(|value| value as u32)
        .or(episode_count_hint);
    let score = first.get("score").and_then(Value::as_f64);
    let season = first.get("season").and_then(Value::as_str).map(|season| {
        let mut chars = season.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => season.to_string(),
        }
    });
    let year = first.get("year").and_then(Value::as_i64).map(|value| value as i32);

    Some(AnimePresenceMetadata {
        canonical_title,
        image_url,
        external_url,
        media_type,
        episode_count,
        score,
        season,
        year,
    })
}

#[cfg(test)]
mod tests {
    use super::{decode_provider_url, extract_source_pairs_from_raw, sort_episode_numbers};

    #[test]
    fn decodes_allanime_provider_url_tokens() {
        assert_eq!(decode_provider_url("504c4c48021717"), "http://");
    }

    #[test]
    fn rewrites_clock_provider_endpoint() {
        assert_eq!(decode_provider_url("175b54575b53"), "/clock.json");
    }

    #[test]
    fn extracts_source_pairs_from_raw_response() {
        let raw = r#"{"sourceUrl":"--504c4c48","sourceName":"Default"}"#;

        assert_eq!(
            extract_source_pairs_from_raw(raw),
            vec![("Default".to_string(), "504c4c48".to_string())]
        );
    }

    #[test]
    fn sorts_episode_numbers_numerically() {
        let mut episodes = vec!["10".to_string(), "2".to_string(), "1.5".to_string()];

        sort_episode_numbers(&mut episodes);

        assert_eq!(episodes, vec!["1.5", "2", "10"]);
    }
}
