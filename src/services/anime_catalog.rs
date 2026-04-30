use crate::domain::anime::{AnimeResult, EpisodeUrl};
use crate::providers::allanime::ApiClient;

#[allow(async_fn_in_trait)]
pub trait AnimeCatalog {
    async fn search_anime(&self, query: &str) -> Result<Vec<AnimeResult>, String>;
    async fn episodes_list(&self, show_id: &str) -> Result<Vec<String>, String>;
    async fn get_episode_url(
        &self,
        show_id: &str,
        episode: &str,
        quality: &str,
    ) -> Result<EpisodeUrl, String>;
}

impl AnimeCatalog for ApiClient {
    async fn search_anime(&self, query: &str) -> Result<Vec<AnimeResult>, String> {
        ApiClient::search_anime(self, query).await
    }

    async fn episodes_list(&self, show_id: &str) -> Result<Vec<String>, String> {
        ApiClient::episodes_list(self, show_id).await
    }

    async fn get_episode_url(
        &self,
        show_id: &str,
        episode: &str,
        quality: &str,
    ) -> Result<EpisodeUrl, String> {
        ApiClient::get_episode_url(self, show_id, episode, quality).await
    }
}
