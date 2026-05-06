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
    let t = &app.theme;
    let hints = [
        ("j/k", "move"),
        ("⏎", "play"),
        ("n", "next page"),
        ("t", "filter type"),
        ("r", "rating"),
        ("f", "sfw"),
        ("Esc", "back"),
    ]
    .iter()
    .flat_map(|(k, a)| theme::keyhint(k, a, t))
    .collect();

    let filter_label = app.top_filter_type.as_deref().unwrap_or("all");
    let rating_label = app.top_filter_rating.as_deref().unwrap_or("any");
    let sfw_label = if app.top_filter_sfw { "sfw" } else { "all" };
    let title = format!(
        "top · {} · {} · {}",
        filter_label.to_lowercase(),
        rating_label.to_lowercase(),
        sfw_label
    );
    let stage = chrome::shell(f, app, &title, hints);

    if app.top_loading {
        let line = Line::from(vec![
            Span::raw("  "),
            Span::styled(
                theme::spinner_frame(app.spinner_tick).to_string(),
                theme::fg(t.gold),
            ),
            Span::raw("  "),
            Span::styled("loading top anime…".to_string(), theme::italic(t.text_dim)),
        ]);
        f.render_widget(Paragraph::new(line), stage);
        return;
    }

    if app.top_anime.is_empty() {
        let line = Line::from(Span::styled(
            "  no results found".to_string(),
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
    render_list(f, inner, app);
}

fn render_list(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let visible = area.height as usize;
    let sel = app.top_selected;
    let offset = if sel >= visible { sel - visible + 1 } else { 0 };

    let lines: Vec<Line<'static>> = app
        .top_anime
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

            let rank = format!("{:>3}", i + 1);
            let rank_style = if is_sel {
                Style::default().fg(t.gold).add_modifier(Modifier::BOLD)
            } else {
                theme::fg(t.text_subtle)
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

            let type_str = anime.anime_type.as_deref().unwrap_or(" ").to_string();

            let studio = anime
                .studio_names()
                .first()
                .map(|s| s.to_string())
                .unwrap_or_default();

            Line::from(vec![
                Span::raw("  "),
                bar,
                Span::raw(" "),
                Span::styled(rank, rank_style),
                Span::raw(" "),
                Span::styled(theme::truncate(anime.display_title(), 32), title_style),
                Span::styled("  ".to_string(), theme::fg(t.text_subtle)),
                Span::styled(score_str, theme::fg(t.gold)),
                Span::raw(" "),
                Span::styled(ep_str, theme::fg(t.text_dim)),
                Span::raw(" "),
                Span::styled(type_str, theme::fg(t.moon)),
                Span::raw(" "),
                Span::styled(theme::truncate(&studio, 14), theme::fg(t.text_subtle)),
            ])
        })
        .collect();

    f.render_widget(Paragraph::new(lines), area);
}
