pub const DEFAULT_DISCORD_CLIENT_ID: &str = "1499484972929646702";

pub fn default_discord_client_id() -> Option<String> {
    let client_id = DEFAULT_DISCORD_CLIENT_ID.trim();
    if client_id.is_empty() {
        None
    } else {
        Some(client_id.to_string())
    }
}
