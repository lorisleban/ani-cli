use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::chrome;
use crate::app::App;
use crate::ascii::{self, Mood};
use crate::theme::{self, *};

pub fn render(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let hints = [
        ("n / l", "next"),
        ("p / h", "prev"),
        ("r", "replay"),
        ("s", "pick ep"),
        ("Esc", "back"),
    ]
    .iter()
    .flat_map(|(k, a)| theme::keyhint(k, a, t))
    .collect();
    let stage = chrome::shell(f, app, "now playing", hints);

    let anime_title = app
        .selected_anime
        .as_ref()
        .map(|a| a.title.clone())
        .unwrap_or_else(|| "—".into());
    let ep = app.current_episode.as_deref().unwrap_or("—");
    let total = app
        .selected_anime
        .as_ref()
        .map(|a| a.episode_count)
        .unwrap_or(0);
    let cur: f64 = ep.parse().unwrap_or(0.0);
    let ratio = if total > 0 { cur / total as f64 } else { 0.0 };
    let quality = app
        .episode_url
        .as_ref()
        .map(|u| u.quality.clone())
        .unwrap_or_else(|| "—".into());

    // Three horizontal zones: [left spacer | center info | Mochi corner]
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(4),
            Constraint::Percentage(60),
            Constraint::Length(14),
        ])
        .split(stage);

    render_center(f, cols[1], app, &anime_title, ep, total, ratio, &quality);
    render_mochi_corner(f, cols[2], app);
}

fn render_center(
    f: &mut Frame,
    area: Rect,
    app: &App,
    anime_title: &str,
    ep: &str,
    total: u32,
    ratio: f64,
    quality: &str,
) {
    let t = &app.theme;

    // Vertically center the block
    let block_h = 10u16;
    let pad_top = area.height.saturating_sub(block_h) / 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(pad_top),
            Constraint::Length(1), // overline
            Constraint::Length(1), // gap
            Constraint::Length(2), // title (can wrap once)
            Constraint::Length(1), // episode
            Constraint::Length(1), // gap
            Constraint::Length(1), // progress bar
            Constraint::Length(1), // pct + stats
            Constraint::Length(1), // gap
            Constraint::Length(1), // resume pill
            Constraint::Length(1), // gap
            Constraint::Length(1), // meta
            Constraint::Min(0),
        ])
        .split(area);

    // overline
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(SPARKLE, theme::fg(t.gold)),
            Span::raw("  "),
            Span::styled("now playing", theme::italic(t.text_dim)),
            Span::raw("  "),
            Span::styled(SPARKLE, theme::fg(t.gold)),
        ]))
        .alignment(Alignment::Center),
        chunks[1],
    );

    // title
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            anime_title,
            Style::default().fg(t.text).add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center),
        chunks[3],
    );

    // episode
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("episode ", theme::fg(t.text_dim)),
            Span::styled(
                ep.to_string(),
                Style::default().fg(t.gold).add_modifier(Modifier::BOLD),
            ),
            if total > 0 {
                Span::styled(format!("  of  {}", total), theme::fg(t.text_dim))
            } else {
                Span::raw("")
            },
        ]))
        .alignment(Alignment::Center),
        chunks[4],
    );

    // progress bar
    let bar_w = (chunks[6].width as usize).saturating_sub(8).min(52);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(theme::progress_bar(ratio, bar_w), theme::fg(t.gold)),
            Span::raw("  "),
            Span::styled(format!("{:.0}%", ratio * 100.0), theme::fg(t.text_dim)),
        ]))
        .alignment(Alignment::Center),
        chunks[6],
    );

    // pct + episodes remaining
    if total > 0 {
        let remaining = total.saturating_sub(ep.parse::<u32>().unwrap_or(0));
        f.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                format!("{} episodes remaining", remaining),
                theme::dim(t.text_subtle),
            )]))
            .alignment(Alignment::Center),
            chunks[7],
        );
    }

    // next ep pill
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " n ",
                Style::default()
                    .fg(t.bg)
                    .bg(t.gold)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  next episode", theme::fg(t.text_dim)),
            Span::styled("     ", theme::fg(t.text_dim)),
            Span::styled(
                " p ",
                Style::default()
                    .fg(t.bg)
                    .bg(t.moon)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  previous", theme::fg(t.text_dim)),
        ]))
        .alignment(Alignment::Center),
        chunks[9],
    );

    // meta
    let mode_str = match app.mode {
        crate::api::Mode::Sub => "sub",
        crate::api::Mode::Dub => "dub",
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(quality, theme::fg(t.moon)),
            Span::styled("  ·  ", theme::dim(t.text_subtle)),
            Span::styled(app.player_type.name(), theme::fg(t.text_dim)),
            Span::styled("  ·  ", theme::dim(t.text_subtle)),
            Span::styled(mode_str, theme::fg(t.gold)),
        ]))
        .alignment(Alignment::Center),
        chunks[11],
    );
}

fn render_mochi_corner(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;

    // Mochi anchored to bottom of the column
    let mochi_h = 4u16;
    let gap_h = area.height.saturating_sub(mochi_h + 2);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(gap_h),
            Constraint::Length(mochi_h),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let mochi = ascii::render_mochi(Mood::Watching, app.spinner_tick, t);
    f.render_widget(Paragraph::new(mochi), chunks[1]);

    // A tiny label under Mochi
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "watching~",
            theme::italic(t.text_subtle),
        ))),
        chunks[2],
    );
}

fn inset(r: Rect, dx: u16, dy: u16) -> Rect {
    Rect {
        x: r.x + dx,
        y: r.y + dy,
        width: r.width.saturating_sub(dx * 2),
        height: r.height.saturating_sub(dy * 2),
    }
}
