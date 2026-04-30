use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::app::App;

pub struct Theme {
    pub bg: Color,
    pub surface: Color,
    pub text: Color,
    pub text_dim: Color,
    pub text_subtle: Color,
    pub border: Color,
    pub border_focus: Color,
    pub gold: Color,   // primary accent — lantern
    pub moon: Color,   // secondary accent — moonlight
    pub sage: Color,   // success / playing
    pub coral: Color,  // danger / errors
}

impl Theme {
    pub fn lantern() -> Self {
        Self {
            bg: Color::Rgb(26, 27, 38),
            surface: Color::Rgb(36, 40, 59),
            text: Color::Rgb(192, 202, 245),
            text_dim: Color::Rgb(120, 130, 170),
            text_subtle: Color::Rgb(65, 72, 104),
            border: Color::Rgb(41, 46, 66),
            border_focus: Color::Rgb(122, 162, 247),
            gold: Color::Rgb(224, 175, 104),
            moon: Color::Rgb(122, 162, 247),
            sage: Color::Rgb(158, 206, 106),
            coral: Color::Rgb(247, 118, 142),
        }
    }
}

pub const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
pub const TOAST_REVEAL: &[&str] = &["▁", "▃", "▆", "█"];

// Glyphs — kept tiny, deliberately
pub const SEL_BAR: &str = "▌";
pub const DOT: &str = "·";
pub const RING: &str = "○";
pub const RING_FILLED: &str = "●";
pub const CHECK: &str = "✓";
pub const ARROW: &str = "→";
pub const SPARKLE: &str = "✦";
pub const HEART: &str = "♡";
pub const FLOWER: &str = "✿";

// Style helpers ---------------------------------------------------------

pub fn fg(c: Color) -> Style {
    Style::default().fg(c)
}
pub fn bold(c: Color) -> Style {
    Style::default().fg(c).add_modifier(Modifier::BOLD)
}
pub fn dim(c: Color) -> Style {
    Style::default().fg(c).add_modifier(Modifier::DIM)
}
pub fn italic(c: Color) -> Style {
    Style::default().fg(c).add_modifier(Modifier::ITALIC)
}

// Truncate / pad on display width (cheap char count — fine for our text)
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else if max <= 1 {
        "…".to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}

pub fn pad_right(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        truncate(s, width)
    } else {
        format!("{}{}", s, " ".repeat(width - len))
    }
}

// A horizontal block-progress bar, 8 sub-cells per char for smoothness.
pub fn progress_bar(ratio: f64, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let r = ratio.clamp(0.0, 1.0);
    let total_eighths = (r * (width as f64) * 8.0).round() as usize;
    let full = total_eighths / 8;
    let partial = total_eighths % 8;
    let blocks = ['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];
    let mut out = String::with_capacity(width * 3);
    for _ in 0..full {
        out.push('█');
    }
    if full < width && partial > 0 {
        out.push(blocks[partial - 1]);
    }
    let drawn = full + if partial > 0 && full < width { 1 } else { 0 };
    for _ in drawn..width {
        out.push('░');
    }
    out
}

// A "pill" — short label with surface bg, used for mode/quality
pub fn pill<'a>(label: &'a str, fg: Color, bg: Color) -> Span<'a> {
    Span::styled(
        format!(" {} ", label),
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    )
}

// One key hint as a span sequence: [ k ] action
pub fn keyhint<'a>(key: &'a str, action: &'a str, t: &Theme) -> Vec<Span<'a>> {
    vec![
        Span::styled(format!(" {} ", key), Style::default().fg(t.bg).bg(t.gold).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" {}   ", action), fg(t.text_dim)),
    ]
}

// Spinner frame for current tick
pub fn spinner_frame(tick: usize) -> &'static str {
    SPINNER[tick % SPINNER.len()]
}

// "Breathing" mode pill — alternates BOLD/DIM by tick to feel alive
pub fn mode_pill<'a>(mode: &'a str, tick: usize, t: &Theme) -> Span<'a> {
    let m = if (tick / 5) % 2 == 0 {
        Modifier::BOLD
    } else {
        Modifier::DIM
    };
    Span::styled(
        format!(" {} ", mode),
        Style::default().fg(t.bg).bg(t.gold).add_modifier(m),
    )
}

// Toast formatting — uses TOAST_REVEAL frames during first ~120ms of life
pub fn render_toasts<'a>(app: &'a App, width: u16) -> Vec<Line<'a>> {
    let t = &app.theme;
    let mut lines = Vec::new();
    for toast in app.toasts.iter().rev().take(3) {
        let age = toast.born.elapsed().as_millis();
        let frame_idx = ((age / 30) as usize).min(TOAST_REVEAL.len() - 1);
        let bar_char = TOAST_REVEAL[frame_idx];
        let (bar_color, label_color) = if toast.is_error {
            (t.coral, t.coral)
        } else {
            (t.moon, t.text)
        };
        let body = truncate(&toast.message, (width as usize).saturating_sub(6));
        let pad = (width as usize)
            .saturating_sub(body.chars().count() + 4);
        lines.push(Line::from(vec![
            Span::raw(" ".repeat(pad)),
            Span::styled(bar_char.to_string(), fg(bar_color)),
            Span::raw(" "),
            Span::styled(body, fg(label_color)),
            Span::raw(" "),
        ]));
    }
    lines
}

pub fn now_clock() -> String {
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let h = ((secs / 3600) % 24) as u8;
    let m = ((secs / 60) % 60) as u8;
    format!("{:02}:{:02}", h, m)
}
