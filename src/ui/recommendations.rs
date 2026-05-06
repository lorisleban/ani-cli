use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::theme::{self, *};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;

    if app.recommendations_loading {
        let line = Line::from(vec![
            Span::raw("  "),
            Span::styled(
                theme::spinner_frame(app.spinner_tick).to_string(),
                theme::fg(t.gold),
            ),
            Span::raw("  "),
            Span::styled(
                "loading recommendations…".to_string(),
                theme::italic(t.text_dim),
            ),
        ]);
        f.render_widget(Paragraph::new(line), area);
        return;
    }

    if app.recommendations.is_empty() {
        let line = Line::from(Span::styled(
            "  no recommendations yet".to_string(),
            theme::italic(t.text_dim),
        ));
        f.render_widget(Paragraph::new(line), area);
        return;
    }

    let visible = area.height as usize;
    let sel = app.recommendations_selected;
    let offset = if sel >= visible { sel - visible + 1 } else { 0 };

    let mut lines: Vec<Line<'static>> = vec![Line::from(vec![
        Span::styled(
            "RECOMMENDATIONS",
            Style::default().fg(t.moon).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {} entries", app.recommendations.len()),
            theme::dim(t.text_subtle),
        ),
    ])];

    for (i, rec) in app
        .recommendations
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible.saturating_sub(1))
    {
        let is_sel = i == sel;
        let bar = if is_sel {
            Span::styled(SEL_BAR.to_string(), theme::fg(t.gold))
        } else {
            Span::raw("  ")
        };
        let title_style = if is_sel {
            Style::default().fg(t.text).add_modifier(Modifier::BOLD)
        } else {
            theme::fg(t.text_dim)
        };
        let title = rec.entry.title.as_deref().unwrap_or("unknown");
        let votes_str = rec.votes.map(|v| format!("{} recs", v)).unwrap_or_default();

        lines.push(Line::from(vec![
            Span::raw("  "),
            bar,
            Span::raw(" "),
            Span::styled(theme::truncate(title, 36), title_style),
            if !votes_str.is_empty() {
                Span::styled(format!("  {}", votes_str), theme::dim(t.text_subtle))
            } else {
                Span::raw("")
            },
        ]));
    }

    f.render_widget(Paragraph::new(lines), area);
}
