use crate::domain::anime::AnimePresenceMetadata;
use crate::domain::jikan::*;
use crate::providers::jikan::JikanClient;

#[allow(async_fn_in_trait)]
pub trait JikanCatalog {
    async fn get_anime_full(&self, mal_id: u32) -> Result<JikanAnime, String>;
    async fn search_anime(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<JikanPaginated<JikanAnime>, String>;
    async fn get_current_season(&self, page: u32) -> Result<JikanPaginated<JikanAnime>, String>;
    async fn get_season(
        &self,
        year: i32,
        season: &str,
        page: u32,
    ) -> Result<JikanPaginated<JikanAnime>, String>;
    async fn get_upcoming(&self, page: u32) -> Result<JikanPaginated<JikanAnime>, String>;
    async fn get_schedule(
        &self,
        day: &str,
        page: u32,
    ) -> Result<JikanPaginated<JikanAnime>, String>;
    async fn get_top_anime(
        &self,
        page: u32,
        anime_type: Option<&str>,
        filter: Option<&str>,
        rating: Option<&str>,
        sfw: bool,
    ) -> Result<JikanPaginated<JikanAnime>, String>;
    async fn get_genres(&self) -> Result<JikanResponse<Vec<JikanGenre>>, String>;
    async fn get_recommendations(
        &self,
        mal_id: u32,
    ) -> Result<JikanResponse<Vec<JikanRecommendation>>, String>;
    async fn get_characters(
        &self,
        mal_id: u32,
    ) -> Result<JikanResponse<Vec<JikanCharacter>>, String>;
    async fn get_season_list(&self) -> Result<Vec<JikanSeasonInfo>, String>;
    async fn fetch_presence_metadata(
        &self,
        title: &str,
        episode_count_hint: Option<u32>,
    ) -> Result<Option<AnimePresenceMetadata>, String>;
    async fn resolve_mal_id(
        &self,
        allanime_id: &str,
        title: &str,
        episode_count_hint: Option<u32>,
    ) -> Result<Option<MalIdMapping>, String>;
    async fn fetch_jikan_anime(
        &self,
        title: &str,
        episode_count_hint: Option<u32>,
    ) -> Result<Option<JikanAnime>, String>;
}

impl JikanCatalog for JikanClient {
    async fn get_anime_full(&self, mal_id: u32) -> Result<JikanAnime, String> {
        JikanClient::get_anime_full(self, mal_id).await
    }

    async fn search_anime(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<JikanPaginated<JikanAnime>, String> {
        JikanClient::search_anime(self, query, limit).await
    }

    async fn get_current_season(&self, page: u32) -> Result<JikanPaginated<JikanAnime>, String> {
        JikanClient::get_current_season(self, page).await
    }

    async fn get_season(
        &self,
        year: i32,
        season: &str,
        page: u32,
    ) -> Result<JikanPaginated<JikanAnime>, String> {
        JikanClient::get_season(self, year, season, page).await
    }

    async fn get_upcoming(&self, page: u32) -> Result<JikanPaginated<JikanAnime>, String> {
        JikanClient::get_upcoming(self, page).await
    }

    async fn get_schedule(
        &self,
        day: &str,
        page: u32,
    ) -> Result<JikanPaginated<JikanAnime>, String> {
        JikanClient::get_schedule(self, day, page).await
    }

    async fn get_top_anime(
        &self,
        page: u32,
        anime_type: Option<&str>,
        filter: Option<&str>,
        rating: Option<&str>,
        sfw: bool,
    ) -> Result<JikanPaginated<JikanAnime>, String> {
        JikanClient::get_top_anime(self, page, anime_type, filter, rating, sfw).await
    }

    async fn get_genres(&self) -> Result<JikanResponse<Vec<JikanGenre>>, String> {
        JikanClient::get_genres(self).await
    }
    async fn get_recommendations(
        &self,
        mal_id: u32,
    ) -> Result<JikanResponse<Vec<JikanRecommendation>>, String> {
        JikanClient::get_recommendations(self, mal_id).await
    }
    async fn get_characters(
        &self,
        mal_id: u32,
    ) -> Result<JikanResponse<Vec<JikanCharacter>>, String> {
        JikanClient::get_characters(self, mal_id).await
    }

    async fn get_season_list(&self) -> Result<Vec<JikanSeasonInfo>, String> {
        JikanClient::get_season_list(self).await
    }

    async fn fetch_presence_metadata(
        &self,
        title: &str,
        episode_count_hint: Option<u32>,
    ) -> Result<Option<AnimePresenceMetadata>, String> {
        JikanClient::fetch_presence_metadata(self, title, episode_count_hint).await
    }

    async fn resolve_mal_id(
        &self,
        allanime_id: &str,
        title: &str,
        episode_count_hint: Option<u32>,
    ) -> Result<Option<MalIdMapping>, String> {
        JikanClient::resolve_mal_id(self, allanime_id, title, episode_count_hint).await
    }

    async fn fetch_jikan_anime(
        &self,
        title: &str,
        episode_count_hint: Option<u32>,
    ) -> Result<Option<JikanAnime>, String> {
        JikanClient::fetch_jikan_anime(self, title, episode_count_hint).await
    }
}
