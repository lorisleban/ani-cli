use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::chrome;
use crate::app::App;
use crate::theme::{self, *};

pub fn render(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let hints = [
        ("⏎", "play"),
        ("hjkl", "move"),
        ("d", "sub/dub"),
        ("Esc", "back"),
    ]
    .iter()
    .flat_map(|(k, a)| theme::keyhint(k, a, t))
    .collect();

    let title = app
        .selected_anime
        .as_ref()
        .map(|a| theme::truncate(&a.title, 48))
        .unwrap_or_else(|| "detail".to_string());
    let stage = chrome::shell(f, app, &title, hints);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4), // header
            Constraint::Length(1), // dotted divider
            Constraint::Min(3),    // grid
        ])
        .split(stage);

    render_header(f, chunks[0], app);
    f.render_widget(
        Paragraph::new(chrome::dotted(chunks[1].width as usize, t)),
        chunks[1],
    );
    render_grid(f, chunks[2], app);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let anime = match app.selected_anime.as_ref() {
        Some(a) => a,
        None => return,
    };
    let watched = watched_eps_for(app);
    let watched_count = watched.len();
    let total = anime.episode_count as usize;
    let ratio = if total > 0 {
        watched_count as f64 / total as f64
    } else {
        0.0
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(SPARKLE, theme::fg(t.gold)),
            Span::raw("  "),
            Span::styled(
                anime.title.clone(),
                Style::default().fg(t.text).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format!("{} episodes", anime.episode_count),
                theme::fg(t.text_dim),
            ),
            Span::styled("  ·  ", theme::dim(t.text_subtle)),
            Span::styled(format!("{} watched", watched_count), theme::fg(t.sage)),
            Span::styled("  ·  ", theme::dim(t.text_subtle)),
            Span::styled(
                match app.mode {
                    crate::api::Mode::Sub => "sub",
                    crate::api::Mode::Dub => "dub",
                },
                theme::fg(t.gold),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("    "),
            Span::styled(theme::progress_bar(ratio, 36), theme::fg(t.gold)),
            Span::raw("  "),
            Span::styled(format!("{:.0}%", ratio * 100.0), theme::fg(t.text_dim)),
        ]),
    ];
    f.render_widget(Paragraph::new(lines), area);
}

fn watched_eps_for(app: &App) -> Vec<String> {
    let id = match app.selected_anime.as_ref() {
        Some(a) => a.id.clone(),
        None => return vec![],
    };
    app.history
        .iter()
        .filter(|h| h.anime_id == id)
        .map(|h| h.episode.clone())
        .collect()
}

fn render_grid(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;

    if app.episodes_loading {
        let line = Line::from(vec![
            Span::raw("    "),
            Span::styled(theme::spinner_frame(app.spinner_tick), theme::fg(t.gold)),
            Span::raw(" "),
            Span::styled("gathering episodes…", theme::italic(t.text_dim)),
        ]);
        f.render_widget(Paragraph::new(line), area);
        return;
    }
    if app.episodes.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "    nothing here yet",
                theme::italic(t.text_dim),
            ))),
            area,
        );
        return;
    }

    // For very long shows fall back to a list
    if app.episodes.len() > 200 {
        render_long_list(f, area, app);
        return;
    }

    let watched = watched_eps_for(app);
    let cell_w: usize = 6; // " 12 " plus gutter
    let inner_w = (area.width as usize).saturating_sub(4);
    let cols = (inner_w / cell_w).max(1);

    let mut lines: Vec<Line> = Vec::with_capacity(area.height as usize);
    let total = app.episodes.len();
    let rows = total.div_ceil(cols);
    let visible_rows = (area.height as usize).saturating_sub(1);
    let sel_row = app.episode_selected / cols;
    let start_row = sel_row.saturating_sub(visible_rows.saturating_sub(2));
    let end_row = (start_row + visible_rows).min(rows);

    for row in start_row..end_row {
        let mut cells: Vec<Span> = vec![Span::raw("  ")];
        let mut floor: Vec<Span> = vec![Span::raw("  ")];
        for col in 0..cols {
            let i = row * cols + col;
            if i >= total {
                break;
            }
            let ep = &app.episodes[i];
            let is_sel = i == app.episode_selected;
            let is_watched = watched.contains(ep);

            let label = format!(" {:>3} ", short_ep(ep));
            let style = match (is_sel, is_watched) {
                (true, _) => Style::default()
                    .fg(t.bg)
                    .bg(t.gold)
                    .add_modifier(Modifier::BOLD),
                (false, true) => theme::fg(t.sage),
                (false, false) => theme::fg(t.text_dim),
            };
            cells.push(Span::styled(label, style));
            cells.push(Span::raw(" "));

            // floor row: tiny mark under each cell
            let mark = if is_sel {
                "▔▔▔▔▔"
            } else if is_watched {
                "·····"
            } else {
                "     "
            };
            let mark_style = if is_sel {
                theme::fg(t.gold)
            } else {
                theme::dim(t.text_subtle)
            };
            floor.push(Span::styled(mark.to_string(), mark_style));
            floor.push(Span::raw(" "));
        }
        lines.push(Line::from(cells));
        lines.push(Line::from(floor));
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn short_ep(s: &str) -> String {
    // Prefer integer-looking ep names, e.g. "12.5" -> "12.5", "12" -> "12"
    s.to_string()
}

fn render_long_list(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let watched = watched_eps_for(app);
    let visible = area.height as usize;
    let sel = app.episode_selected;
    let offset = if sel >= visible { sel - visible + 1 } else { 0 };

    let lines: Vec<Line> = app
        .episodes
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible)
        .map(|(i, ep)| {
            let is_sel = i == sel;
            let is_watched = watched.contains(ep);
            let bar = if is_sel {
                Span::styled(SEL_BAR, theme::fg(t.gold))
            } else {
                Span::raw(" ")
            };
            let icon = if is_watched {
                Span::styled(CHECK, theme::fg(t.sage))
            } else {
                Span::styled(RING, theme::fg(t.text_subtle))
            };
            let title_style = if is_sel {
                Style::default().fg(t.text).add_modifier(Modifier::BOLD)
            } else {
                theme::fg(t.text_dim)
            };
            Line::from(vec![
                Span::raw(" "),
                bar,
                Span::raw(" "),
                icon,
                Span::raw("  "),
                Span::styled(format!("Episode {}", ep), title_style),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines), area);
}
