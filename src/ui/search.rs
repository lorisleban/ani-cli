use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
    Frame,
};

use super::chrome;
use crate::app::App;
use crate::theme::{self, *};

pub fn render(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let hints = [
        ("⏎", "open"),
        ("↑↓", "move"),
        ("⌫", "edit"),
        ("Esc", "back"),
    ]
    .iter()
    .flat_map(|(k, a)| theme::keyhint(k, a, t))
    .collect();
    let stage = chrome::shell(f, app, "search", hints);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2), // input
            Constraint::Length(1), // counter
            Constraint::Min(3),    // results
        ])
        .split(stage);

    render_input(f, chunks[0], app);
    render_counter(f, chunks[1], app);
    render_results(f, chunks[2], app);
}

fn render_input(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let cursor = if app.spinner_tick % 10 < 5 {
        "▌"
    } else {
        " "
    };
    let prompt = Line::from(vec![
        Span::styled("  /  ", theme::fg(t.gold)),
        Span::styled(
            &app.search_input,
            Style::default().fg(t.text).add_modifier(Modifier::BOLD),
        ),
        Span::styled(cursor, theme::fg(t.gold)),
    ]);
    let underline = Line::from(Span::styled(
        "─".repeat(area.width as usize),
        theme::fg(t.border),
    ));
    f.render_widget(Paragraph::new(vec![prompt, underline]), area);
}

fn render_counter(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let line = if app.search_loading {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(theme::spinner_frame(app.spinner_tick), theme::fg(t.gold)),
            Span::raw(" "),
            Span::styled("listening…", theme::italic(t.text_dim)),
        ])
    } else if app.search_input.len() < 2 {
        Line::from(Span::styled(
            "  type at least 2 letters",
            theme::dim(t.text_subtle),
        ))
    } else {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{}", app.search_results.len()),
                Style::default().fg(t.gold).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" results", theme::fg(t.text_dim)),
        ])
    };
    f.render_widget(Paragraph::new(line), area);
}

fn render_results(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    if app.search_results.is_empty() {
        return;
    }
    let visible = area.height as usize;
    let sel = app.search_selected;
    let offset = if sel >= visible { sel - visible + 1 } else { 0 };

    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible)
        .map(|(i, r)| {
            let is_sel = i == sel;
            let title_w = (area.width as usize).saturating_sub(16);
            let bar_span = if is_sel {
                Span::styled(SEL_BAR, theme::fg(t.gold))
            } else {
                Span::raw(" ")
            };
            let title_style = if is_sel {
                Style::default().fg(t.text).add_modifier(Modifier::BOLD)
            } else {
                theme::fg(t.text_dim)
            };
            ListItem::new(Line::from(vec![
                Span::raw(" "),
                bar_span,
                Span::raw(" "),
                Span::styled(theme::pad_right(&r.title, title_w), title_style),
                Span::styled(format!(" {} ep ", r.episode_count), theme::fg(t.moon)),
            ]))
        })
        .collect();
    f.render_widget(List::new(items), area);
}
