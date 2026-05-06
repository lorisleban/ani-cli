use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::msg::{Cmd, Msg};
use super::terminal::AppTerminal;
use super::update;
use crate::api::ApiClient;
use crate::app::{App, Screen};
use crate::providers::jikan::JikanClient;
use crate::ui;
use crate::update::perform_update;
use tokio::sync::mpsc;

pub async fn run_app(
    terminal: &mut AppTerminal,
    app: &mut App,
    api: ApiClient,
) -> Result<(), Box<dyn std::error::Error>> {
    let tick_rate = Duration::from_millis(80);

    // Reactor Infrastructure
    let (tx, mut rx) = mpsc::channel::<Msg>(100);

    let mut interval = tokio::time::interval(tick_rate);
    let jikan_client = app.jikan.clone();

    // Initial fetch
    let _ = tx.send(Msg::Navigate(Screen::Home)).await;

    loop {
        let size = terminal.size().unwrap_or(ratatui::layout::Size {
            width: 120,
            height: 40,
        });
        let cols = grid_cols(size);
        let quality = app.quality.clone();

        terminal.draw(|f| ui::render(f, app))?;

        tokio::select! {
            _ = interval.tick() => {
                let _ = tx.send(Msg::Tick).await;
            }
            Some(msg) = rx.recv() => {
                let cmds = update::update(app, msg, cols);
                for cmd in cmds {
                    handle_command(cmd, tx.clone(), &api, &jikan_client, &quality);
                }
            }
            event_res = async {
                if event::poll(Duration::from_millis(10))? {
                    Ok::<_, Box<dyn std::error::Error>>(Some(event::read()?))
                } else {
                    Ok(None)
                }
            } => {
                if let Ok(Some(Event::Key(key))) = event_res {
                    if key.kind == KeyEventKind::Press {
                        // Check global keys first
                        if let Some(msg) = handle_global(app, key) {
                            let _ = tx.send(msg).await;
                        } else {
                            let _ = tx.send(Msg::Key(key)).await;
                        }
                    }
                }
            }
        }

        if !app.running {
            break;
        }
    }

    Ok(())
}

/// Handle keys that work everywhere. Returns true if consumed.
fn handle_global(app: &App, key: KeyEvent) -> Option<Msg> {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Msg::Quit);
    }

    if let Some((first, _)) = app.key_seq {
        if first == 'g' {
            match key.code {
                KeyCode::Char('h') => return Some(Msg::Navigate(Screen::Home)),
                KeyCode::Char('w') => {
                    return Some(Msg::Batch(vec![
                        Msg::RefreshHistory,
                        Msg::Navigate(Screen::WatchHistory),
                    ]))
                }
                KeyCode::Char('g') => return Some(Msg::Key(key)), // Let update handle resetting offset
                _ => {}
            }
        }
    }

    match key.code {
        KeyCode::Char('U') => {
            if app.update_in_progress {
                Some(Msg::Toast("update already running".to_string(), false))
            } else if app.update_available.is_some() {
                Some(Msg::Batch(vec![
                    Msg::Toast("updating...".to_string(), false),
                    Msg::TriggerUpdate,
                ]))
            } else {
                Some(Msg::Toast("checking for updates...".to_string(), false))
            }
        }
        KeyCode::Esc if app.update_popup_visible => Some(Msg::ToggleUpdatePopup(false)),
        KeyCode::Char('?') => Some(Msg::Navigate(Screen::Help)),
        KeyCode::Char('Q') => Some(Msg::Quit),
        KeyCode::Char('/') => Some(Msg::Navigate(Screen::Search)),
        KeyCode::Char('g') => Some(Msg::Key(key)), // Let update handle prefix
        _ => None,
    }
}

fn handle_command(
    cmd: Cmd,
    tx: mpsc::Sender<Msg>,
    api: &ApiClient,
    jikan: &JikanClient,
    quality: &str,
) {
    match cmd {
        Cmd::None => {}
        Cmd::Batch(cmds) => {
            for c in cmds {
                handle_command(c, tx.clone(), api, jikan, quality);
            }
        }
        // ... (other commands)
        Cmd::ExecutePlayer(data) => {
            let tx = tx.clone();
            let quality = quality.to_string();
            // We'll need a way to launch the player without direct &mut App access.
            // For now, we'll spawn a task that handles the DB and process.
            tokio::spawn(async move {
                // In a real implementation, we'd need to recreate the DB connection or pass a handle
                let db = crate::persistence::sqlite_history::Database::new().ok();
                if let Some(db) = db {
                    let _ = db.start_watch_session(
                        crate::persistence::sqlite_history::NewWatchSession {
                            anime_id: &data.entry.anime_id,
                            title: &data.entry.title,
                            episode: &data.entry.episode,
                            total_episodes: data.entry.total_episodes,
                            player: "mpv", // Placeholder: should come from AppOptions or similar
                            mode: "sub",   // Placeholder
                            quality: &quality,
                        },
                    );
                }

                // Launch player process (simplified for now as we don't have all App fields)
                // In Phase 4 we will fully decouple this.
                let _ = crate::player::launch_player(
                    crate::player::PlayerType::detect(),
                    &data.url.url,
                    &format!("{} - Episode {}", data.entry.title, data.entry.episode),
                    data.url.referer.as_deref(),
                    data.url.subtitle.as_deref(),
                    false, // No discord for now in this worker
                );

                let _ = tx.send(Msg::RefreshHistory).await;
            });
        }
        Cmd::FetchAiringToday(sid) => {
            let jikan = jikan.clone();
            let tx = tx.clone();
            let day = today_day_name().to_string();
            tokio::spawn(async move {
                match jikan.get_schedule(&day, 1).await {
                    Ok(resp) => {
                        let _ = tx.send(Msg::HomeAiringLoaded(Ok(resp.data), sid)).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Msg::HomeAiringLoaded(Err(e), sid)).await;
                    }
                }
            });
        }
        Cmd::FetchTopAiring(sid) => {
            let jikan = jikan.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                match jikan
                    .get_top_anime(1, None, Some("airing"), None, true)
                    .await
                {
                    Ok(resp) => {
                        let _ = tx.send(Msg::HomeTopLoaded(Ok(resp.data), sid)).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Msg::HomeTopLoaded(Err(e), sid)).await;
                    }
                }
            });
        }
        Cmd::SearchAnime(query, sid) => {
            let api = api.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                match api.search_anime(&query).await {
                    Ok(results) => {
                        let _ = tx.send(Msg::SearchLoaded(Ok(results), sid)).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Msg::SearchLoaded(Err(e), sid)).await;
                    }
                }
            });
        }
        Cmd::FetchAnimeDetail {
            provider_id,
            title,
            session_id,
        } => {
            let api = api.clone();
            let jikan = jikan.clone();
            let tx = tx.clone();

            // Spawn episode list fetch (with potential ID lookup)
            let api_c = api.clone();
            let tx_c = tx.clone();
            let title_c = title.clone();
            tokio::spawn(async move {
                let resolved_id = if let Some(id) = provider_id {
                    Some(id)
                } else {
                    // Fuzzy match: search by title and take first result
                    match api_c.search_anime(&title_c).await {
                        Ok(results) => results.first().map(|r| r.id.clone()),
                        Err(_) => None,
                    }
                };

                if let Some(id) = resolved_id {
                    let id_for_msg = id.clone();
                    match api_c.episodes_list(&id).await {
                        Ok(eps) => {
                            let _ = tx_c
                                .send(Msg::EpisodesLoaded(Ok((eps, id_for_msg)), session_id))
                                .await;
                        }
                        Err(e) => {
                            let _ = tx_c.send(Msg::EpisodesLoaded(Err(e), session_id)).await;
                        }
                    }
                } else {
                    let _ = tx_c
                        .send(Msg::EpisodesLoaded(
                            Err("could not find show on provider".to_string()),
                            session_id,
                        ))
                        .await;
                }
            });

            // Spawn metadata fetch
            tokio::spawn(async move {
                let jikan_data = jikan.fetch_jikan_anime(&title, None).await.ok().flatten();
                let _ = tx
                    .send(Msg::MetadataLoaded(Box::new(jikan_data), session_id))
                    .await;
            });
        }
        Cmd::FetchImage(url, sid) => {
            let tx = tx.clone();
            tokio::spawn(async move {
                let img = crate::ui::cover_image::fetch_image_data(&url).await;
                let _ = tx.send(Msg::ImageLoaded(img, sid)).await;
            });
        }
        Cmd::FetchRecommendations(mal_id, sid) => {
            let jikan = jikan.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                match jikan.get_recommendations(mal_id).await {
                    Ok(resp) => {
                        let _ = tx
                            .send(Msg::RecommendationsLoaded(Ok(resp.data), sid))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx.send(Msg::RecommendationsLoaded(Err(e), sid)).await;
                    }
                }
            });
        }
        Cmd::LaunchPlayer(entry) => {
            let api = api.clone();
            let jikan = jikan.clone();
            let tx = tx.clone();
            let quality = quality.to_string();
            tokio::spawn(async move {
                let _ = tx
                    .send(Msg::Toast(
                        format!("preparing ep {}...", entry.episode),
                        false,
                    ))
                    .await;

                let (stream_res, meta_res, eps_res) = tokio::join!(
                    api.get_episode_url(&entry.anime_id, &entry.episode, &quality),
                    jikan.fetch_presence_metadata(&entry.title, entry.total_episodes),
                    api.episodes_list(&entry.anime_id)
                );

                match stream_res {
                    Ok(url) => {
                        let meta = meta_res.ok().flatten();
                        let eps = eps_res.ok();
                        let _ = tx
                            .send(Msg::StreamReady(Box::new(
                                crate::runtime::msg::PlaybackData {
                                    url,
                                    entry,
                                    metadata: meta,
                                    episodes: eps,
                                },
                            )))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Msg::Toast(format!("stream error: {}", e), true))
                            .await;
                    }
                }
            });
        }
        Cmd::PerformUpdate => {
            let tx = tx.clone();
            tokio::spawn(async move {
                let res = perform_update().await;
                let _ = tx.send(Msg::UpdateStatus(res)).await;
            });
        }
    }
}

fn grid_cols(size: ratatui::layout::Size) -> usize {
    crate::ui::layout::calculate_grid_cols(size.width)
}

fn today_day_name() -> &'static str {
    use chrono::Datelike;
    let now = chrono::Local::now();
    match now.weekday() {
        chrono::Weekday::Mon => "monday",
        chrono::Weekday::Tue => "tuesday",
        chrono::Weekday::Wed => "wednesday",
        chrono::Weekday::Thu => "thursday",
        chrono::Weekday::Fri => "friday",
        chrono::Weekday::Sat => "saturday",
        chrono::Weekday::Sun => "sunday",
    }
}
