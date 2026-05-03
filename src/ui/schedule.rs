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

const DAYS: &[&str] = &[
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];

pub fn render(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let hints = [
        ("j/k", "move"),
        ("⏎", "play"),
        ("h/l", "prev/next day"),
        ("Esc", "back"),
    ]
    .iter()
    .flat_map(|(k, a)| theme::keyhint(k, a, t))
    .collect();

    let day_label = capitalize_first(&app.schedule_day);
    let title = format!("schedule · {}", day_label.to_lowercase());
    let stage = chrome::shell(f, app, &title, hints);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(3)])
        .split(stage);

    render_day_tabs(f, chunks[0], app);
    render_list(f, chunks[1], app);
}

fn render_day_tabs(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let mut spans: Vec<Span<'static>> = vec![Span::raw(" ")];

    for (i, day) in DAYS.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        let is_active = *day == app.schedule_day;
        let label = match *day {
            "monday" => "Mon",
            "tuesday" => "Tue",
            "wednesday" => "Wed",
            "thursday" => "Thu",
            "friday" => "Fri",
            "saturday" => "Sat",
            "sunday" => "Sun",
            _ => day,
        };
        if is_active {
            spans.push(Span::styled(
                format!(" {} ", label),
                Style::default()
                    .fg(ratatui::style::Color::Black)
                    .bg(t.gold)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(format!(" {} ", label), theme::fg(t.text_dim)));
        }
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_list(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;

    if app.schedule_loading {
        let line = Line::from(vec![
            Span::raw(" "),
            Span::styled(
                theme::spinner_frame(app.spinner_tick).to_string(),
                theme::fg(t.gold),
            ),
            Span::raw(" "),
            Span::styled("loading schedule…".to_string(), theme::italic(t.text_dim)),
        ]);
        f.render_widget(Paragraph::new(line), area);
        return;
    }

    if app.schedule_anime.is_empty() {
        let line = Line::from(Span::styled(
            " nothing airing on this day".to_string(),
            theme::italic(t.text_dim),
        ));
        f.render_widget(Paragraph::new(line), area);
        return;
    }

    let visible = area.height as usize;
    let sel = app.schedule_selected;
    let offset = if sel >= visible { sel - visible + 1 } else { 0 };

    let lines: Vec<Line<'static>> = app
        .schedule_anime
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

            let broadcast_str = anime.broadcast.string.as_deref().unwrap_or("");

            Line::from(vec![
                Span::raw(" "),
                bar,
                Span::raw(" "),
                Span::styled(theme::truncate(anime.display_title(), 36), title_style),
                Span::styled("  ".to_string(), theme::fg(t.text_subtle)),
                Span::styled(score_str, theme::fg(t.gold)),
                Span::raw(" "),
                Span::styled(ep_str, theme::fg(t.text_dim)),
                Span::raw(" "),
                Span::styled(theme::truncate(broadcast_str, 18), theme::fg(t.text_subtle)),
            ])
        })
        .collect();

    f.render_widget(Paragraph::new(lines), area);
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => s.to_string(),
    }
}
