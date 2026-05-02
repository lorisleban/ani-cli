use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
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
    let hints = [("Esc", "close")]
        .iter()
        .flat_map(|(k, a)| theme::keyhint(k, a, t))
        .collect();
    let stage = chrome::shell(f, app, "help", hints);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4), // mochi
            Constraint::Length(1),
            Constraint::Min(3), // columns
        ])
        .split(stage);

    let mochi = ascii::render_mochi(Mood::Happy, app.spinner_tick, t);
    f.render_widget(
        Paragraph::new(mochi).alignment(Alignment::Center),
        chunks[0],
    );

    let tagline = Line::from(Span::styled(
        "everything Mochi knows",
        theme::italic(t.text_dim),
    ));
    f.render_widget(
        Paragraph::new(tagline).alignment(Alignment::Center),
        chunks[1],
    );

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    let left = column(
        "global",
        &[
            ("/", "search from anywhere"),
            ("g h", "go home"),
            ("g s", "go search"),
            ("g w", "go history"),
            ("g p", "go now-playing"),
            ("U", "self-update"),
            ("R", "release notes"),
            ("?", "this card"),
            ("Q", "quit"),
            ("ctrl-c", "quit (hard)"),
        ],
        t,
    );
    let right = column(
        "in lists & grids",
        &[
            ("j / ↓", "down"),
            ("k / ↑", "up"),
            ("h / l", "left / right (grid)"),
            ("g g", "top"),
            ("G", "bottom"),
            ("⏎", "open / play"),
            ("d", "toggle sub/dub"),
            ("x  /  X", "delete / clear (history)"),
        ],
        t,
    );

    f.render_widget(Paragraph::new(left), cols[0]);
    f.render_widget(Paragraph::new(right), cols[1]);
}

fn column<'a>(title: &'a str, rows: &'a [(&'a str, &'a str)], t: &Theme) -> Vec<Line<'a>> {
    let mut out = vec![
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                title.to_uppercase(),
                Style::default().fg(t.gold).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
    ];
    for (k, desc) in rows {
        out.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format!("{:<10}", k),
                Style::default().fg(t.text).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(*desc, theme::fg(t.text_dim)),
        ]));
    }
    out
}
