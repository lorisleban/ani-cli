use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::ascii::MARK_TINY;
use crate::theme::{self, Theme};

/// Three-band layout: mast (1) | stage (flex) | rail (1). Returns stage rect.
pub fn shell<'a>(f: &mut Frame, app: &App, context: &str, hints: Vec<Span<'a>>) -> Rect {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area);

    render_mast(f, chunks[0], app, context);
    render_rail(f, chunks[2], app, hints);
    render_toasts(f, chunks[1], app);

    chunks[1]
}

fn render_mast(f: &mut Frame, area: Rect, app: &App, context: &str) {
    let t = &app.theme;
    let clock = theme::now_clock();
    let mode_str = match app.mode {
        crate::api::Mode::Sub => "SUB",
        crate::api::Mode::Dub => "DUB",
    };

    let left: Vec<Span> = vec![
        Span::raw(" "),
        Span::styled(
            MARK_TINY,
            Style::default().fg(t.gold).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ╱  ", theme::dim(t.text_subtle)),
        Span::styled(context.to_string(), theme::fg(t.text)),
    ];

    let right: Vec<Span> = vec![
        theme::mode_pill(mode_str, app.spinner_tick, t),
        Span::raw(" "),
        Span::styled(format!("{} ", theme::SPARKLE), theme::fg(t.gold)),
        Span::styled(clock, theme::fg(t.text_dim)),
        Span::raw(" "),
    ];

    let left_w: usize = left.iter().map(|s| s.content.chars().count()).sum();
    let right_w: usize = right.iter().map(|s| s.content.chars().count()).sum();
    let fill = (area.width as usize)
        .saturating_sub(left_w)
        .saturating_sub(right_w);

    let mut spans = left;
    spans.push(Span::raw(" ".repeat(fill)));
    spans.extend(right);

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_rail<'a>(f: &mut Frame, area: Rect, app: &App, hints: Vec<Span<'a>>) {
    let t = &app.theme;
    let mut spans = vec![Span::raw(" ")];
    spans.extend(hints);

    // right side: vim-buffer indicator
    let right_text = if let Some((c, _)) = app.key_seq {
        format!("…{}_ ", c)
    } else {
        " ?  help    Q  quit ".to_string()
    };
    let right = Span::styled(right_text.clone(), theme::dim(t.text_subtle));

    let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let right_w = right_text.chars().count();
    let fill = (area.width as usize)
        .saturating_sub(used)
        .saturating_sub(right_w);
    spans.push(Span::raw(" ".repeat(fill)));
    spans.push(right);

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_toasts(f: &mut Frame, stage: Rect, app: &App) {
    if app.toasts.is_empty() {
        return;
    }
    let lines = theme::render_toasts(app, stage.width);
    let h = lines.len() as u16;
    if h == 0 {
        return;
    }
    let y = stage.y + stage.height.saturating_sub(h + 1);
    let toast_area = Rect::new(stage.x, y, stage.width, h);
    f.render_widget(Paragraph::new(lines), toast_area);
}

/// A divider line of dots, full width.
pub fn dotted<'a>(width: usize, t: &Theme) -> Line<'a> {
    Line::from(Span::styled("·".repeat(width), theme::dim(t.text_subtle)))
}
