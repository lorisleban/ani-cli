//! Wordmark + Mochi the cat. Two things, kept simple.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme::{self, Theme};

// ── Wordmark ─────────────────────────────────────────────────────────────

pub const WORDMARK: &[&str] = &[
    r" ________  ________   ___                 ________  ___       ___     ",
    r"|\   __  \|\   ___  \|\  \               |\   ____\|\  \     |\  \    ",
    r"\ \  \|\  \ \  \\ \  \ \  \  ____________\ \  \___|\ \  \    \ \  \   ",
    r" \ \   __  \ \  \\ \  \ \  \|\____________\ \  \    \ \  \    \ \  \  ",
    r"  \ \  \ \  \ \  \\ \  \ \  \|____________|\ \  \____\ \  \____\ \  \ ",
    r"   \ \__\ \__\ \__\\ \__\ \__\              \ \_______\ \_______\ \__\",
    r"    \|__|\|__|\|__| \|__|\|__|               \|_______|\|_______|\|__|",
];

pub const MARK_TINY: &str = "ani·cli";

// ── Mochi frames ─────────────────────────────────────────────────────────

/// Neutral / idle
pub const MOCHI_IDLE: &[&str] = &[r"  ╱|、   ", r" (˚ˎ 。7 ", r"  |、˜〵 ", r" じしˍ,)ノ"];

/// Blink (one frame)
pub const MOCHI_BLINK: &[&str] = &[r"  ╱|、   ", r" (- ˕ -7 ", r"  |、˜〵 ", r" じしˍ,)ノ"];

/// Happy / excited
pub const MOCHI_HAPPY: &[&str] = &[r"  ╱|、   ", r" (^ ω ^7 ", r"  |、˜〵 ", r" じしˍ,)ノ"];

/// Watching intently — head tilted
pub const MOCHI_WATCHING: &[&str] = &[r"  ╱|、   ", r" (◕ᴗ◕7  ", r"  |、˜〵 ", r" じしˍ,)ノ"];

/// Sleepy
pub const MOCHI_SLEEPY: &[&str] = &[r"  ╱|、   ", r" (─ ˕ ─7 ", r"  |、˜〵 ", r" じしˍ,)ノ"];

/// Thinking / searching
pub const MOCHI_THINK: &[&str] = &[r"  ╱|、   ", r" (・_・7  ", r"  |、˜〵 ", r" じしˍ,)ノ"];

// ── Mood enum ─────────────────────────────────────────────────────────────

#[derive(Copy, Clone, PartialEq)]
pub enum Mood {
    Idle,
    Happy,
    Watching,
    Sleepy,
    Thinking,
}

pub fn mochi_for_tick(mood: Mood, tick: usize) -> &'static [&'static str] {
    // blink roughly every 4 seconds (at 80ms tick that's ~50 ticks)
    let blink = (tick % 50) < 2;
    match (mood, blink) {
        (_, true) => MOCHI_BLINK,
        (Mood::Idle, _) => MOCHI_IDLE,
        (Mood::Happy, _) => MOCHI_HAPPY,
        (Mood::Watching, _) => MOCHI_WATCHING,
        (Mood::Sleepy, _) => MOCHI_SLEEPY,
        (Mood::Thinking, _) => MOCHI_THINK,
    }
}

// ── Render helpers ────────────────────────────────────────────────────────

/// Full 4-row Mochi, gold-tinted.
pub fn render_mochi<'a>(mood: Mood, tick: usize, t: &Theme) -> Vec<Line<'a>> {
    mochi_for_tick(mood, tick)
        .iter()
        .map(|row| Line::from(Span::styled(row.to_string(), theme::fg(t.gold))))
        .collect()
}

/// The wordmark, revealed two rows at a time on first render.
/// Row colours: first two gold, middle three moon, last two dim.
pub fn render_wordmark<'a>(t: &Theme, splash_tick: usize) -> Vec<Line<'a>> {
    let revealed = ((splash_tick / 2) + 1).min(WORDMARK.len());
    WORDMARK
        .iter()
        .take(revealed)
        .enumerate()
        .map(|(i, row)| {
            let style = if i < 2 {
                Style::default().fg(t.gold).add_modifier(Modifier::BOLD)
            } else if i < 5 {
                Style::default().fg(t.moon)
            } else {
                theme::dim(t.text_subtle)
            };
            Line::from(Span::styled(row.to_string(), style))
        })
        .collect()
}

/// A single-line compact form: the face row only, for tight spaces.
/// Looks like:  (◕ᴗ◕7  "message"
pub fn render_mochi_inline<'a>(mood: Mood, tick: usize, message: &'a str, t: &Theme) -> Line<'a> {
    // row index 1 is the "face" row
    let face = mochi_for_tick(mood, tick)
        .get(1)
        .copied()
        .unwrap_or("(˚ˎ 。7");
    Line::from(vec![
        Span::styled(face.to_string(), theme::fg(t.gold)),
        Span::raw("  "),
        Span::styled(message, theme::italic(t.text_dim)),
    ])
}
