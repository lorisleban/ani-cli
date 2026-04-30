use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
    Frame,
};

use super::chrome;
use crate::app::App;
use crate::ascii::{self, Mood};
use crate::theme::{self, *};

pub fn render(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let hints = [
        ("⏎", "open"),
        ("jk", "move"),
        ("x", "delete"),
        ("X", "clear all"),
        ("Esc", "back"),
    ]
    .iter()
    .flat_map(|(k, a)| theme::keyhint(k, a, t))
    .collect();
    let stage = chrome::shell(f, app, "history", hints);

    if app.history.is_empty() {
        render_empty(f, stage, app);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(1), Constraint::Min(3)])
        .split(stage);

    let header = Line::from(vec![
        Span::styled(
            format!("{} entries", app.history.len()),
            Style::default().fg(t.gold).add_modifier(Modifier::BOLD),
        ),
        Span::styled("    ────", theme::fg(t.border)),
    ]);
    f.render_widget(Paragraph::new(header), chunks[0]);

    render_list(f, chunks[1], app);
}

fn render_empty(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(2),
            Constraint::Length(4),
            Constraint::Length(2),
            Constraint::Min(2),
        ])
        .split(area);

    let mochi = ascii::render_mochi(Mood::Idle, app.spinner_tick, t);
    f.render_widget(
        Paragraph::new(mochi).alignment(ratatui::layout::Alignment::Center),
        chunks[1],
    );
    let msg = Line::from(Span::styled(
        "no history yet — press s to find something",
        theme::italic(t.text_dim),
    ));
    f.render_widget(
        Paragraph::new(msg).alignment(ratatui::layout::Alignment::Center),
        chunks[2],
    );
}

fn render_list(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let visible = area.height as usize;
    let sel = app.history_selected;
    let offset = if sel >= visible { sel - visible + 1 } else { 0 };

    let items: Vec<ListItem> = app
        .history
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible)
        .map(|(i, e)| {
            let is_sel = i == sel;
            let bar = if is_sel {
                Span::styled(SEL_BAR, theme::fg(t.gold))
            } else {
                Span::raw(" ")
            };
            let title_style = if is_sel {
                Style::default().fg(t.text).add_modifier(Modifier::BOLD)
            } else {
                theme::fg(t.text_dim)
            };
            let date = if e.watched_at.len() >= 10 {
                &e.watched_at[..10]
            } else {
                &e.watched_at
            };
            let title_w = (area.width as usize).saturating_sub(28);
            ListItem::new(Line::from(vec![
                Span::raw(" "),
                bar,
                Span::raw(" "),
                Span::styled(CHECK, theme::fg(t.sage)),
                Span::raw("  "),
                Span::styled(theme::pad_right(&e.title, title_w), title_style),
                Span::styled(format!(" ep {} ", e.episode), theme::fg(t.moon)),
                Span::styled(DOT, theme::dim(t.text_subtle)),
                Span::raw(" "),
                Span::styled(date.to_string(), theme::dim(t.text_subtle)),
            ]))
        })
        .collect();

    f.render_widget(List::new(items), area);
}
