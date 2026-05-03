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
        ("[/]", "prev/next season"),
        ("Esc", "back"),
    ]
    .iter()
    .flat_map(|(k, a)| theme::keyhint(k, a, t))
    .collect();

    let season_label = match (&app.season_year, &app.season_name) {
        (Some(y), Some(s)) => format!("{} {}", s, y),
        _ => "Current Season".to_string(),
    };
    let title = format!("season · {}", season_label.to_lowercase());
    let stage = chrome::shell(f, app, &title, hints);

    if app.season_loading {
        let line = Line::from(vec![
            Span::raw(" "),
            Span::styled(
                theme::spinner_frame(app.spinner_tick).to_string(),
                theme::fg(t.gold),
            ),
            Span::raw(" "),
            Span::styled("loading season…".to_string(), theme::italic(t.text_dim)),
        ]);
        f.render_widget(Paragraph::new(line), stage);
        return;
    }

    if app.season_anime.is_empty() {
        let line = Line::from(Span::styled(
            " no anime found for this season".to_string(),
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
    let sel = app.season_selected;
    let offset = if sel >= visible { sel - visible + 1 } else { 0 };

    let lines: Vec<Line<'static>> = app
        .season_anime
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible)
        .map(|(i, anime)| {
            let is_sel = i == sel;
            let bar = if is_sel {
                Span::styled(SEL_BAR.to_string(), theme::fg(t.gold))
            } else {
                Span::raw(" ")
            };

            let status_icon = if anime.is_currently_airing() {
                Span::styled(" ●".to_string(), theme::fg(t.sage))
            } else {
                Span::raw("  ".to_string())
            };

            let title_style = if is_sel {
                Style::default().fg(t.text).add_modifier(Modifier::BOLD)
            } else {
                theme::fg(t.text)
            };

            let score_str = anime
                .score
                .map(|s| format!("{:.1}", s))
                .unwrap_or_else(|| "  - ".to_string());

            let ep_str = anime
                .episodes
                .map(|e| format!("{:>2}ep", e))
                .unwrap_or_else(|| "   -".to_string());

            let type_str = anime.anime_type.as_deref().unwrap_or("    ").to_string();

            let studio = anime
                .studio_names()
                .first()
                .map(|s| s.to_string())
                .unwrap_or_default();

            Line::from(vec![
                Span::raw(" "),
                bar,
                Span::raw(" "),
                status_icon,
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
