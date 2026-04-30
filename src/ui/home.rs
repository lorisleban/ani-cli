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
        ("⏎", "resume"),
        ("jk", "move"),
        ("s", "search"),
        ("w", "history"),
        ("d", "sub/dub"),
    ]
    .iter()
    .flat_map(|(k, a)| theme::keyhint(k, a, t))
    .collect();
    let stage = chrome::shell(f, app, "home", hints);

    match (app.continue_watching.is_empty(), app.history.is_empty()) {
        (true, true) => render_first_run(f, stage, app),
        (true, false) => render_empty_queue(f, stage, app),
        _ => render_dashboard(f, stage, app),
    }
}

// ── Dashboard ─────────────────────────────────────────────────────────────

fn render_dashboard(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let sel = app.home_selected.min(app.continue_watching.len() - 1);

    // Left 36 cols = queue list | 1 col divider | rest = detail
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(36),
            Constraint::Length(1),
            Constraint::Min(20),
        ])
        .split(area);

    // Vertical divider
    let divider: Vec<Line> = (0..cols[1].height)
        .map(|_| Line::from(Span::styled("│", theme::dim(t.border))))
        .collect();
    f.render_widget(Paragraph::new(divider), cols[1]);

    render_queue_pane(f, cols[0], app, sel);
    render_detail_pane(f, cols[2], app, sel);
}

// ── Queue pane (left) ─────────────────────────────────────────────────────

fn render_queue_pane(f: &mut Frame, area: Rect, app: &App, sel: usize) {
    let t = &app.theme;
    let inner = inset(area, 2, 1);

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                "QUEUE",
                Style::default().fg(t.gold).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("   {} shows", app.continue_watching.len()),
                theme::dim(t.text_subtle),
            ),
        ]),
        Line::raw(""),
    ];

    // Each entry: 3 rows (title / progress+ep / gap)
    let row_h = 3usize;
    let header_h = 2usize;
    let body_h = (inner.height as usize).saturating_sub(header_h);
    let visible = (body_h / row_h).max(1);
    let offset = if sel >= visible { sel - visible + 1 } else { 0 };

    for (i, entry) in app
        .continue_watching
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible)
    {
        let is_sel = i == sel;
        let total = entry.total_episodes.unwrap_or(0);
        let cur: f64 = entry.episode.parse().unwrap_or(0.0);
        let ratio = if total > 0 { cur / total as f64 } else { 0.0 };

        let sel_bar = if is_sel {
            Span::styled(SEL_BAR, theme::fg(t.gold))
        } else {
            Span::raw(" ")
        };
        let title_style = if is_sel {
            Style::default().fg(t.text).add_modifier(Modifier::BOLD)
        } else {
            theme::fg(t.text_dim)
        };
        let title_w = (inner.width as usize).saturating_sub(4);

        // Title row
        lines.push(Line::from(vec![
            sel_bar,
            Span::raw(" "),
            Span::styled(theme::truncate(&entry.title, title_w), title_style),
        ]));

        // Progress + ep row
        let bar_color = if is_sel { t.gold } else { t.text_subtle };
        let ep_color = if is_sel { t.text_dim } else { t.text_subtle };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(theme::progress_bar(ratio, 12), theme::fg(bar_color)),
            Span::raw("  "),
            Span::styled(
                if total > 0 {
                    format!("ep {}/{}", entry.episode, total)
                } else {
                    format!("ep {}", entry.episode)
                },
                theme::fg(ep_color),
            ),
        ]));

        lines.push(Line::raw(""));
    }

    // Scroll position
    if app.continue_watching.len() > visible {
        lines.push(Line::from(Span::styled(
            format!("  {}/{}", sel + 1, app.continue_watching.len()),
            theme::dim(t.text_subtle),
        )));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

// ── Detail pane (right) ───────────────────────────────────────────────────

fn render_detail_pane(f: &mut Frame, area: Rect, app: &App, sel: usize) {
    let t = &app.theme;
    let entry = &app.continue_watching[sel];
    let inner = inset(area, 3, 1);

    let total = entry.total_episodes.unwrap_or(0);
    let cur: f64 = entry.episode.parse().unwrap_or(0.0);
    let next_ep = (cur + 1.0) as u32;
    let ratio = if total > 0 { cur / total as f64 } else { 0.0 };
    let last_at = app
        .history
        .iter()
        .find(|h| h.anime_id == entry.anime_id)
        .map(|h| {
            if h.watched_at.len() >= 10 {
                h.watched_at[..10].to_string()
            } else {
                h.watched_at.clone()
            }
        })
        .unwrap_or_default();

    // Split detail pane into top (content) and bottom (Mochi + recent)
    let pane = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),   // main info
            Constraint::Length(1), // dotted divider
            Constraint::Length(5), // Mochi + recent strip
        ])
        .split(inner);

    render_detail_main(f, pane[0], app, entry, next_ep, ratio, total, &last_at, t);
    f.render_widget(
        Paragraph::new(chrome::dotted(inner.width as usize, t)),
        pane[1],
    );
    render_mochi_recent(f, pane[2], app, sel, t);
}

fn render_detail_main<'a>(
    f: &mut Frame,
    area: Rect,
    app: &App,
    entry: &crate::db::WatchEntry,
    next_ep: u32,
    ratio: f64,
    total: u32,
    last_at: &str,
    t: &crate::theme::Theme,
) {
    let bar_w = (area.width as usize).saturating_sub(2).min(56);

    let lines = vec![
        Line::from(vec![
            Span::styled(SPARKLE, theme::fg(t.gold)),
            Span::raw("  "),
            Span::styled("up next", theme::italic(t.text_dim)),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            theme::truncate(&entry.title, area.width as usize),
            Style::default().fg(t.text).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("episode ", theme::fg(t.text_dim)),
            Span::styled(
                format!("{}", next_ep),
                Style::default().fg(t.gold).add_modifier(Modifier::BOLD),
            ),
            if total > 0 {
                Span::styled(format!("  of {}", total), theme::fg(t.text_dim))
            } else {
                Span::raw("")
            },
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            theme::progress_bar(ratio, bar_w),
            theme::fg(t.gold),
        )),
        Line::from(vec![
            Span::styled(format!("{:.0}%", ratio * 100.0), theme::fg(t.text_dim)),
            Span::styled("   ·   ", theme::dim(t.text_subtle)),
            Span::styled(format!("{} watched", entry.episode), theme::fg(t.sage)),
            if total > 0 {
                Span::styled(
                    format!(
                        "   ·   {} to go",
                        total.saturating_sub(entry.episode.parse::<u32>().unwrap_or(0))
                    ),
                    theme::fg(t.text_dim),
                )
            } else {
                Span::raw("")
            },
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled(
                " ⏎ ",
                Style::default()
                    .fg(t.bg)
                    .bg(t.gold)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  resume episode ", theme::fg(t.text)),
            Span::styled(
                format!("{}", next_ep),
                Style::default().fg(t.gold).add_modifier(Modifier::BOLD),
            ),
        ]),
        if !last_at.is_empty() {
            Line::from(vec![Span::raw("")])
        } else {
            Line::raw("")
        },
        if !last_at.is_empty() {
            Line::from(vec![
                Span::styled("last watched  ", theme::dim(t.text_subtle)),
                Span::styled(last_at.to_string(), theme::fg(t.text_dim)),
            ])
        } else {
            Line::raw("")
        },
    ];

    f.render_widget(Paragraph::new(lines), area);
}

/// Bottom strip: Mochi on the left, recent entries on the right.
fn render_mochi_recent(f: &mut Frame, area: Rect, app: &App, sel: usize, t: &crate::theme::Theme) {
    // Mochi is 10 chars wide; the rest goes to the recent list.
    let mochi_w = 12u16;
    if area.width < mochi_w + 8 {
        // Not enough room — just Mochi
        let mochi = ascii::render_mochi(Mood::Idle, app.spinner_tick, t);
        f.render_widget(Paragraph::new(mochi), area);
        return;
    }

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(mochi_w), Constraint::Min(8)])
        .split(area);

    // Pick mood based on queue selection and time-of-day vibes
    let mood = mochi_mood(sel, app.spinner_tick);
    let mochi = ascii::render_mochi(mood, app.spinner_tick, t);
    f.render_widget(Paragraph::new(mochi), cols[0]);

    // Recent strip — header + up to (area.height - 1) entries
    render_recent_strip(f, cols[1], app, t);
}

fn mochi_mood(sel: usize, tick: usize) -> Mood {
    // Slightly happy when on first item, idle otherwise.
    // Sprinkle in a "thinking" frame very occasionally.
    if (tick / 100) % 8 == 0 {
        Mood::Thinking
    } else if sel == 0 {
        Mood::Happy
    } else {
        Mood::Idle
    }
}

fn render_recent_strip(f: &mut Frame, area: Rect, app: &App, t: &crate::theme::Theme) {
    let mut lines = vec![Line::from(vec![
        Span::styled(
            "RECENT",
            Style::default().fg(t.gold).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("   {} entries", app.history.len()),
            theme::dim(t.text_subtle),
        ),
        Span::styled("   ·   ", theme::dim(t.text_subtle)),
        Span::styled(
            " w ",
            Style::default()
                .fg(t.bg)
                .bg(t.moon)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  archive", theme::dim(t.text_subtle)),
    ])];

    let cap = (area.height as usize).saturating_sub(1);
    for h in app.history.iter().take(cap) {
        let date = if h.watched_at.len() >= 10 {
            &h.watched_at[..10]
        } else {
            &h.watched_at
        };
        let title_w = (area.width as usize).saturating_sub(24);
        lines.push(Line::from(vec![
            Span::styled(DOT, theme::dim(t.text_subtle)),
            Span::raw("  "),
            Span::styled(theme::truncate(&h.title, title_w), theme::fg(t.text_dim)),
            Span::raw("  "),
            Span::styled(format!("ep {}", h.episode), theme::dim(t.text_subtle)),
            Span::raw("  "),
            Span::styled(date.to_string(), theme::dim(t.text_subtle)),
        ]));
    }

    f.render_widget(Paragraph::new(lines), area);
}

// ── Empty states ──────────────────────────────────────────────────────────

fn render_empty_queue(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let inner = inset(area, 4, 2);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(4), // Mochi
            Constraint::Length(1), // gap
            Constraint::Length(1), // heading
            Constraint::Length(1), // cta search
            Constraint::Length(1), // cta history
            Constraint::Min(1),
        ])
        .split(inner);

    let mochi = ascii::render_mochi(Mood::Idle, app.spinner_tick, t);
    f.render_widget(
        Paragraph::new(mochi).alignment(Alignment::Center),
        chunks[1],
    );

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "your queue is empty",
            Style::default().fg(t.text).add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center),
        chunks[3],
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("press ", theme::fg(t.text_dim)),
            Span::styled(
                " s ",
                Style::default()
                    .fg(t.bg)
                    .bg(t.gold)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  to find something new", theme::fg(t.text_dim)),
        ]))
        .alignment(Alignment::Center),
        chunks[4],
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("or ", theme::dim(t.text_subtle)),
            Span::styled(
                " w ",
                Style::default()
                    .fg(t.bg)
                    .bg(t.moon)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  revisit your {} past shows", app.history.len()),
                theme::dim(t.text_subtle),
            ),
        ]))
        .alignment(Alignment::Center),
        chunks[5],
    );
}

fn render_first_run(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let inner = inset(area, 2, 1);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(7), // wordmark
            Constraint::Length(1), // tagline
            Constraint::Length(1), // gap
            Constraint::Length(4), // Mochi
            Constraint::Length(1), // gap
            Constraint::Length(1), // cta
            Constraint::Min(1),
        ])
        .split(inner);

    let wm = ascii::render_wordmark(t, app.splash_tick);
    f.render_widget(Paragraph::new(wm).alignment(Alignment::Center), chunks[1]);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(SPARKLE, theme::fg(t.gold)),
            Span::raw("  "),
            Span::styled("a quiet place to watch", theme::italic(t.text_dim)),
            Span::raw("  "),
            Span::styled(SPARKLE, theme::fg(t.gold)),
        ]))
        .alignment(Alignment::Center),
        chunks[2],
    );

    let mochi = ascii::render_mochi(Mood::Happy, app.spinner_tick, t);
    f.render_widget(
        Paragraph::new(mochi).alignment(Alignment::Center),
        chunks[4],
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("press ", theme::fg(t.text_dim)),
            Span::styled(
                " s ",
                Style::default()
                    .fg(t.bg)
                    .bg(t.gold)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  to find your first show", theme::fg(t.text_dim)),
        ]))
        .alignment(Alignment::Center),
        chunks[6],
    );
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn inset(r: Rect, dx: u16, dy: u16) -> Rect {
    Rect {
        x: r.x + dx,
        y: r.y + dy,
        width: r.width.saturating_sub(dx * 2),
        height: r.height.saturating_sub(dy * 2),
    }
}
