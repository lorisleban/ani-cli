use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::chrome;
use crate::app::App;
use crate::theme::{self, *};

pub fn render(f: &mut Frame, app: &App) {
    if app.genre_picked.is_some() {
        render_genre_browse(f, app);
    } else {
        render_genre_picker(f, app);
    }
}

fn render_genre_picker(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let hints = [("j/k", "move"), ("⏎", "select"), ("Esc", "back")]
        .iter()
        .flat_map(|(k, a)| theme::keyhint(k, a, t))
        .collect();

    let title = "genres";
    let stage = chrome::shell(f, app, title, hints);

    if app.genre_loading {
        let line = Line::from(vec![
            Span::raw("  "),
            Span::styled(
                theme::spinner_frame(app.spinner_tick).to_string(),
                theme::fg(t.gold),
            ),
            Span::raw("  "),
            Span::styled("loading genres…".to_string(), theme::italic(t.text_dim)),
        ]);
        f.render_widget(Paragraph::new(line), stage);
        return;
    }

    if app.genres.is_empty() {
        let line = Line::from(Span::styled(
            "  no genres found".to_string(),
            theme::italic(t.text_dim),
        ));
        f.render_widget(Paragraph::new(line), stage);
        return;
    }

    let inner = Rect::new(
        stage.x + 1,
        stage.y + 1,
        stage.width.saturating_sub(2),
        stage.height.saturating_sub(2),
    );
    render_genre_list(f, inner, app);
}

fn render_genre_list(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let visible = area.height as usize;
    let sel = app.genre_selected;

    let col_w: usize = 22;
    let cols = (area.width as usize / col_w).clamp(1, 4);
    let total = app.genres.len();
    let rows = total.div_ceil(cols);
    let visible_rows = visible;
    let start_row = if sel / cols >= visible_rows {
        sel / cols - visible_rows + 1
    } else {
        0
    };
    let end_row = (start_row + visible_rows).min(rows);

    let mut lines: Vec<Line<'static>> = Vec::new();
    for row in start_row..end_row {
        let mut spans: Vec<Span<'static>> = vec![Span::raw("  ")];
        for col in 0..cols {
            let i = row * cols + col;
            if i >= total {
                break;
            }
            let genre = &app.genres[i];
            let is_sel = i == sel;
            let label = format!(" {} ", genre.name);
            let count_str = genre.count.map(|c| format!("{}", c)).unwrap_or_default();
            let style = if is_sel {
                Style::default()
                    .fg(t.bg)
                    .bg(t.gold)
                    .add_modifier(Modifier::BOLD)
            } else {
                theme::fg(t.text_dim)
            };
            spans.push(Span::styled(label.clone(), style));
            if !count_str.is_empty() {
                let count_style = if is_sel {
                    Style::default()
                        .fg(t.bg)
                        .bg(t.moon)
                        .add_modifier(Modifier::BOLD)
                } else {
                    theme::dim(t.text_subtle)
                };
                spans.push(Span::styled(format!(" {} ", count_str), count_style));
            }
            let padding = col_w.saturating_sub(label.len() + count_str.len() + 2);
            spans.push(Span::raw(" ".repeat(padding)));
        }
        lines.push(Line::from(spans));
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn render_genre_browse(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let genre_name = app
        .genre_picked
        .as_ref()
        .map(|g| g.name.clone())
        .unwrap_or_default();

    let hints = [
        ("j/k", "move"),
        ("⏎", "play"),
        ("n", "next page"),
        ("Esc", "back to genres"),
    ]
    .iter()
    .flat_map(|(k, a)| theme::keyhint(k, a, t))
    .collect();

    let title = format!("genre · {}", genre_name.to_lowercase());
    let stage = chrome::shell(f, app, &title, hints);

    if app.genre_anime_loading {
        let line = Line::from(vec![
            Span::raw("  "),
            Span::styled(
                theme::spinner_frame(app.spinner_tick).to_string(),
                theme::fg(t.gold),
            ),
            Span::raw("  "),
            Span::styled(
                format!("loading {} anime…", genre_name),
                theme::italic(t.text_dim),
            ),
        ]);
        f.render_widget(Paragraph::new(line), stage);
        return;
    }

    if app.genre_anime.is_empty() {
        let line = Line::from(Span::styled(
            "  no anime found for this genre".to_string(),
            theme::italic(t.text_dim),
        ));
        f.render_widget(Paragraph::new(line), stage);
        return;
    }

    let inner = Rect::new(
        stage.x + 1,
        stage.y + 1,
        stage.width.saturating_sub(2),
        stage.height.saturating_sub(2),
    );
    render_genre_anime_list(f, inner, app);
}

fn render_genre_anime_list(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let visible = area.height as usize;
    let sel = app.genre_anime_selected;
    let offset = if sel >= visible { sel - visible + 1 } else { 0 };

    let lines: Vec<Line<'static>> = app
        .genre_anime
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible)
        .map(|(i, anime)| {
            let is_sel = i == sel;
            let bar = if is_sel {
                Span::styled(SEL_BAR.to_string(), theme::fg(t.gold))
            } else {
                Span::raw("  ")
            };

            let title_style = if is_sel {
                Style::default().fg(t.text).add_modifier(Modifier::BOLD)
            } else {
                theme::fg(t.text)
            };

            let score_str = anime
                .score
                .map(|s| format!("{:.1}", s))
                .unwrap_or_else(|| " - ".to_string());

            let ep_str = anime
                .episodes
                .map(|e| format!("{:>2}ep", e))
                .unwrap_or_else(|| " -".to_string());

            Line::from(vec![
                Span::raw("  "),
                bar,
                Span::raw(" "),
                Span::styled(theme::truncate(anime.display_title(), 36), title_style),
                Span::styled("  ".to_string(), theme::fg(t.text_subtle)),
                Span::styled(score_str, theme::fg(t.gold)),
                Span::raw(" "),
                Span::styled(ep_str, theme::fg(t.text_dim)),
            ])
        })
        .collect();

    f.render_widget(Paragraph::new(lines), area);
}
