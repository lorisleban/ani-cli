use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::Utc;
use discord_rich_presence::activity::{Activity, Assets, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use serde_json::{json, Value};

use crate::domain::anime::AnimePresenceMetadata;
use crate::domain::playback::PlayerType;

const DISCORD_UPDATE_DEBOUNCE_MS: u64 = 900;

#[derive(Debug, Clone)]
pub struct PresencePlayback {
    pub token: u64,
    pub anime_title: String,
    pub episode: String,
    pub total_episodes: Option<u32>,
    pub player: PlayerType,
    pub mode: String,
    pub quality: String,
    pub started_at_unix: i64,
    pub metadata: Option<AnimePresenceMetadata>,
}

#[derive(Debug, Clone)]
pub enum PlayerActivityMonitor {
    Mpv { endpoint: String },
}

#[derive(Debug, Clone, Default)]
struct TimelineState {
    position_seconds: Option<f64>,
    duration_seconds: Option<f64>,
    paused: bool,
}

enum PresenceCommand {
    Start(PresencePlayback, Option<PlayerActivityMonitor>),
    Stop(u64),
    Timeline {
        token: u64,
        timeline: TimelineState,
    },
    Ended(u64),
    Shutdown,
}

pub struct DiscordPresence {
    sender: Sender<PresenceCommand>,
    next_token: Arc<AtomicU64>,
}

impl DiscordPresence {
    pub fn new(client_id: String) -> Self {
        let (sender, receiver) = mpsc::channel();
        let next_token = Arc::new(AtomicU64::new(1));
        spawn_presence_worker(client_id, receiver, sender.clone());

        Self { sender, next_token }
    }

    pub fn next_token(&self) -> u64 {
        self.next_token.fetch_add(1, Ordering::Relaxed)
    }

    pub fn start_playback(
        &self,
        playback: PresencePlayback,
        monitor: Option<PlayerActivityMonitor>,
    ) {
        let _ = self.sender.send(PresenceCommand::Start(playback, monitor));
    }

    pub fn stop(&self, token: u64) {
        let _ = self.sender.send(PresenceCommand::Stop(token));
    }
}

impl Drop for DiscordPresence {
    fn drop(&mut self) {
        let _ = self.sender.send(PresenceCommand::Shutdown);
    }
}

fn spawn_presence_worker(
    client_id: String,
    receiver: Receiver<PresenceCommand>,
    sender: Sender<PresenceCommand>,
) {
    thread::spawn(move || {
        let mut discord = DiscordClient::new(client_id);
        let mut active_token = None;
        let mut active_playback = None;

        while let Ok(command) = receiver.recv() {
            match command {
                PresenceCommand::Start(playback, monitor) => {
                    active_token = Some(playback.token);
                    active_playback = Some(playback.clone());
                    discord.set_activity(&playback, &TimelineState::default());

                    if let Some(PlayerActivityMonitor::Mpv { endpoint }) = monitor {
                        spawn_mpv_monitor(playback.token, endpoint, sender.clone());
                    }
                }
                PresenceCommand::Timeline { token, timeline } => {
                    if active_token != Some(token) {
                        continue;
                    }
                    if let Some(playback) = active_playback.as_ref() {
                        discord.set_activity(playback, &timeline);
                    }
                }
                PresenceCommand::Ended(token) | PresenceCommand::Stop(token) => {
                    if active_token == Some(token) {
                        active_token = None;
                        active_playback = None;
                        discord.clear_activity();
                    }
                }
                PresenceCommand::Shutdown => {
                    discord.clear_activity();
                    break;
                }
            }
        }

        discord.close();
    });
}

fn spawn_mpv_monitor(token: u64, endpoint: String, sender: Sender<PresenceCommand>) {
    thread::spawn(move || {
        let mut stream = match connect_mpv_ipc(&endpoint) {
            Some(stream) => stream,
            None => return,
        };

        let observe_commands = [
            json!({ "command": ["observe_property", 1, "time-pos"] }),
            json!({ "command": ["observe_property", 2, "duration"] }),
            json!({ "command": ["observe_property", 3, "pause"] }),
            json!({ "command": ["get_property", "time-pos"] }),
            json!({ "command": ["get_property", "duration"] }),
            json!({ "command": ["get_property", "pause"] }),
        ];

        for command in observe_commands {
            if writeln!(stream, "{command}").is_err() {
                return;
            }
        }
        let _ = stream.flush();

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        let mut timeline = TimelineState::default();
        let mut last_emit = SystemTime::UNIX_EPOCH;

        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    let _ = sender.send(PresenceCommand::Ended(token));
                    break;
                }
                Ok(_) => {
                    if let Ok(value) = serde_json::from_str::<Value>(line.trim()) {
                        if let Some(event) = value.get("event").and_then(Value::as_str) {
                            if event == "end-file" {
                                let _ = sender.send(PresenceCommand::Ended(token));
                                break;
                            }

                            if event == "property-change" {
                                if apply_property_change(&mut timeline, &value)
                                    && should_emit_timeline(&mut last_emit)
                                {
                                    let _ = sender.send(PresenceCommand::Timeline {
                                        token,
                                        timeline: timeline.clone(),
                                    });
                                }
                            }
                        } else if value.get("error").and_then(Value::as_str) == Some("success")
                            && apply_property_reply(&mut timeline, &value)
                            && should_emit_timeline(&mut last_emit)
                        {
                            let _ = sender.send(PresenceCommand::Timeline {
                                token,
                                timeline: timeline.clone(),
                            });
                        }
                    }
                }
                Err(_) => {
                    let _ = sender.send(PresenceCommand::Ended(token));
                    break;
                }
            }
        }
    });
}

fn apply_property_change(timeline: &mut TimelineState, value: &Value) -> bool {
    match value.get("name").and_then(Value::as_str) {
        Some("time-pos") => {
            timeline.position_seconds = value.get("data").and_then(Value::as_f64);
            true
        }
        Some("duration") => {
            timeline.duration_seconds = value.get("data").and_then(Value::as_f64);
            true
        }
        Some("pause") => {
            timeline.paused = value.get("data").and_then(Value::as_bool).unwrap_or(false);
            true
        }
        _ => false,
    }
}

fn apply_property_reply(timeline: &mut TimelineState, value: &Value) -> bool {
    let data = value.get("data");
    match data {
        Some(data) if data.is_boolean() => {
            timeline.paused = data.as_bool().unwrap_or(false);
            true
        }
        Some(data) if data.is_number() => {
            let number = data.as_f64();
            if timeline.position_seconds.is_none() {
                timeline.position_seconds = number;
            } else if timeline.duration_seconds.is_none() {
                timeline.duration_seconds = number;
            }
            true
        }
        _ => false,
    }
}

fn should_emit_timeline(last_emit: &mut SystemTime) -> bool {
    let now = SystemTime::now();
    let elapsed = now
        .duration_since(*last_emit)
        .unwrap_or_else(|_| Duration::from_secs(0));
    if elapsed >= Duration::from_millis(DISCORD_UPDATE_DEBOUNCE_MS) {
        *last_emit = now;
        true
    } else {
        false
    }
}

#[cfg(unix)]
fn connect_mpv_ipc(endpoint: &str) -> Option<std::os::unix::net::UnixStream> {
    for _ in 0..50 {
        match std::os::unix::net::UnixStream::connect(endpoint) {
            Ok(stream) => return Some(stream),
            Err(err) if err.kind() == ErrorKind::NotFound => thread::sleep(Duration::from_millis(100)),
            Err(_) => return None,
        }
    }
    None
}

#[cfg(windows)]
fn connect_mpv_ipc(endpoint: &str) -> Option<std::fs::File> {
    for _ in 0..50 {
        match OpenOptions::new().read(true).write(true).open(endpoint) {
            Ok(file) => return Some(file),
            Err(err)
                if matches!(
                    err.kind(),
                    ErrorKind::NotFound | ErrorKind::PermissionDenied | ErrorKind::WouldBlock
                ) =>
            {
                thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return None,
        }
    }
    None
}

struct DiscordClient {
    client_id: String,
    ipc: Option<DiscordIpcClient>,
}

impl DiscordClient {
    fn new(client_id: String) -> Self {
        Self {
            client_id,
            ipc: None,
        }
    }

    fn set_activity(&mut self, playback: &PresencePlayback, timeline: &TimelineState) {
        let activity = build_activity(playback, timeline);
        if self.ensure_connected().is_err() {
            return;
        }

        if let Some(ipc) = self.ipc.as_mut() {
            if ipc.set_activity(activity).is_err() {
                self.ipc = None;
            }
        }
    }

    fn clear_activity(&mut self) {
        if let Some(ipc) = self.ipc.as_mut() {
            if ipc.clear_activity().is_err() {
                self.ipc = None;
            }
        }
    }

    fn close(&mut self) {
        if let Some(ipc) = self.ipc.as_mut() {
            let _ = ipc.close();
        }
        self.ipc = None;
    }

    fn ensure_connected(&mut self) -> Result<(), ()> {
        if self.ipc.is_some() {
            return Ok(());
        }

        let mut client = DiscordIpcClient::new(&self.client_id);
        client.connect().map_err(|_| ())?;
        self.ipc = Some(client);
        Ok(())
    }
}

fn build_activity<'a>(playback: &'a PresencePlayback, timeline: &TimelineState) -> Activity<'a> {
    let display_title = playback
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.canonical_title.as_deref())
        .unwrap_or(&playback.anime_title);
    let mut activity = Activity::new()
        .details(display_title)
        .state(build_state(playback))
        .timestamps(build_timestamps(playback, timeline));

    if let Some(metadata) = playback.metadata.as_ref() {
        let mut assets = Assets::new();
        if let Some(image_url) = metadata.image_url.as_deref() {
            assets = assets.large_image(image_url);
        }
        let hover = build_large_text(metadata);
        if !hover.is_empty() {
            assets = assets.large_text(hover);
        }
        if let Some(url) = metadata.external_url.as_deref() {
            assets = assets.large_url(url);
            activity = activity.details_url(url);
        }
        activity = activity.assets(assets);
    }

    activity
}

fn build_state(playback: &PresencePlayback) -> String {
    let mut parts = Vec::new();
    if let Some(total) = playback.total_episodes {
        parts.push(format!("Episode {}/{}", playback.episode, total));
    } else {
        parts.push(format!("Episode {}", playback.episode));
    }
    parts.push(playback.mode.to_uppercase());
    parts.push(playback.quality.clone());
    parts.push(playback.player.name().to_string());
    parts.join(" • ")
}

fn build_timestamps(playback: &PresencePlayback, timeline: &TimelineState) -> Timestamps {
    if let (Some(position), Some(duration)) = (timeline.position_seconds, timeline.duration_seconds) {
        let now = Utc::now().timestamp();
        let start = now.saturating_sub(position.floor() as i64);
        let end = start.saturating_add(duration.ceil() as i64);
        if timeline.paused {
            return Timestamps::new().start(start);
        }
        return Timestamps::new().start(start).end(end);
    }

    Timestamps::new().start(playback.started_at_unix)
}

fn build_large_text(metadata: &AnimePresenceMetadata) -> String {
    let mut parts = Vec::new();
    if let Some(media_type) = metadata.media_type.as_deref() {
        parts.push(media_type.to_string());
    }
    if let Some(episodes) = metadata.episode_count {
        parts.push(format!("{episodes} eps"));
    }
    if let Some(score) = metadata.score {
        parts.push(format!("Score {:.1}", score));
    }

    let season_label = match (metadata.season.as_deref(), metadata.year) {
        (Some(season), Some(year)) => Some(format!("{season} {year}")),
        (Some(season), None) => Some(season.to_string()),
        (None, Some(year)) => Some(year.to_string()),
        (None, None) => None,
    };
    if let Some(season_label) = season_label {
        parts.push(season_label);
    }

    parts.join(" • ")
}

pub fn session_started_at_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_else(|_| Utc::now().timestamp())
}
