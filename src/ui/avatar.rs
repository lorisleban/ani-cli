// src/ui/avatar.rs  ── complete replacement
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Expression {
    Idle, Happy, Excited, Sleepy,
    Searching, Watching, Thinking,
    Love, Surprised, Celebrate,
    Waving, Sad,
}

struct Face {
    left_eye:       &'static str,
    right_eye:      &'static str,
    mouth:          &'static str,
    eye_color:      fn(&Theme) -> ratatui::style::Color,
    mouth_color:    fn(&Theme) -> ratatui::style::Color,
    blush:          bool,
    acc_right:      &'static str,
    acc_left:       &'static str,
}

fn face_for(expr: Expression) -> Face {
    match expr {
        Expression::Idle      => Face { left_eye:"●", right_eye:"●", mouth:"ᴗ",
            eye_color:|t|t.text, mouth_color:|t|t.accent, blush:false, acc_right:"", acc_left:"" },
        Expression::Happy     => Face { left_eye:"●", right_eye:"●", mouth:"ω",
            eye_color:|t|t.text, mouth_color:|t|t.accent, blush:true, acc_right:" ♪", acc_left:"" },
        Expression::Excited   => Face { left_eye:"★", right_eye:"★", mouth:"▽",
            eye_color:|t|t.sparkle, mouth_color:|t|t.accent, blush:true, acc_right:" ✧", acc_left:"✧ " },
        Expression::Sleepy    => Face { left_eye:"─", right_eye:"─", mouth:"ᴗ",
            eye_color:|t|t.text_dim, mouth_color:|t|t.text_dim, blush:false, acc_right:" zZ", acc_left:"" },
        Expression::Searching => Face { left_eye:"◉", right_eye:"◉", mouth:"△",
            eye_color:|t|t.secondary, mouth_color:|t|t.text_secondary, blush:false, acc_right:" ?", acc_left:"" },
        Expression::Watching  => Face { left_eye:"◕", right_eye:"◕", mouth:"∀",
            eye_color:|t|t.primary, mouth_color:|t|t.accent, blush:true, acc_right:" ♫", acc_left:"" },
        Expression::Thinking  => Face { left_eye:"●", right_eye:"·", mouth:"~",
            eye_color:|t|t.text, mouth_color:|t|t.text_secondary, blush:false, acc_right:" …", acc_left:"" },
        Expression::Love      => Face { left_eye:"♥", right_eye:"♥", mouth:"ω",
            eye_color:|t|t.error, mouth_color:|t|t.accent, blush:true, acc_right:" ♥", acc_left:"♥ " },
        Expression::Surprised => Face { left_eye:"◎", right_eye:"◎", mouth:"○",
            eye_color:|t|t.warning, mouth_color:|t|t.warning, blush:false, acc_right:" !", acc_left:"" },
        Expression::Celebrate => Face { left_eye:"✦", right_eye:"✦", mouth:"▽",
            eye_color:|t|t.sparkle, mouth_color:|t|t.sparkle, blush:true, acc_right:" ✧˖°", acc_left:"°˖✧ " },
        Expression::Waving    => Face { left_eye:"●", right_eye:"●", mouth:"ᴗ",
            eye_color:|t|t.text, mouth_color:|t|t.accent, blush:true, acc_right:" ノ", acc_left:"" },
        Expression::Sad       => Face { left_eye:"●", right_eye:"●", mouth:"︵",
            eye_color:|t|t.text_secondary, mouth_color:|t|t.text_dim, blush:false, acc_right:"", acc_left:"" },
    }
}

pub const AVATAR_WIDTH:  u16 = 18;
pub const AVATAR_HEIGHT: u16 = 6;

pub fn render_full<'a>(expr: Expression, theme: &Theme, tick: usize) -> Vec<Line<'a>> {
    let face    = face_for(expr);
    let body    = Style::default().fg(theme.companion);
    let eye_s   = Style::default().fg((face.eye_color)(theme));
    let mouth_s = Style::default().fg((face.mouth_color)(theme));
    let acc_s   = Style::default().fg(theme.sparkle);
    let blush_s = Style::default().fg(theme.accent);

    // Animated ears
    let ear_tick = tick % 4;
    let (ear_l, ear_r) = match expr {
        Expression::Excited | Expression::Celebrate if ear_tick < 2 => ("╭┐", "┌╮"),
        _ => ("┌┐", "┌┐"),
    };

    let blush = if face.blush { "·" } else { " " };

    // Animated feet
    let feet_tick = tick % 6;
    let (foot_l, foot_r) = match expr {
        Expression::Watching | Expression::Excited | Expression::Celebrate if feet_tick < 3 =>
            ("└─╯", "╰─┘"),
        _ => ("└─┘", "└─┘"),
    };

    let al = face.acc_left;
    let al_w = al.chars().count();
    let pad = if al_w > 0 { " ".repeat(al_w) } else { String::new() };

    vec![
        Line::from(vec![
            Span::styled(al.to_string(), acc_s),
            Span::raw(" "),
            Span::styled(ear_l.to_string(), body),
            Span::raw("     "),
            Span::styled(ear_r.to_string(), body),
        ]),
        Line::from(vec![
            Span::raw(pad.clone()),
            Span::styled("┌┘", body),
            Span::styled("└─────┘", body),
            Span::styled("└┐", body),
        ]),
        Line::from(vec![
            Span::raw(pad.clone()),
            Span::styled("│", body),
            Span::raw(" "),
            Span::styled(blush.to_string(), blush_s),
            Span::styled(face.left_eye.to_string(), eye_s),
            Span::raw("   "),
            Span::styled(face.right_eye.to_string(), eye_s),
            Span::styled(blush.to_string(), blush_s),
            Span::raw(" "),
            Span::styled("│", body),
        ]),
        Line::from(vec![
            Span::raw(pad.clone()),
            Span::styled("│", body),
            Span::raw("    "),
            Span::styled(face.mouth.to_string(), mouth_s),
            Span::raw("    "),
            Span::styled("│", body),
            Span::styled(face.acc_right.to_string(), acc_s),
        ]),
        Line::from(vec![
            Span::raw(pad.clone()),
            Span::styled("└─┬┘", body),
            Span::raw("   "),
            Span::styled("└┬─┘", body),
        ]),
        Line::from(vec![
            Span::raw(pad),
            Span::raw("  "),
            Span::styled(foot_l.to_string(), body),
            Span::raw(" "),
            Span::styled(foot_r.to_string(), body),
        ]),
    ]
}

pub fn render_compact<'a>(expr: Expression, message: &str, theme: &Theme) -> Line<'a> {
    let face    = face_for(expr);
    let body    = Style::default().fg(theme.companion);
    let eye_s   = Style::default().fg((face.eye_color)(theme));
    let mouth_s = Style::default().fg((face.mouth_color)(theme));
    let blush_s = Style::default().fg(theme.accent);
    let blush   = if face.blush { "·" } else { " " };

    Line::from(vec![
        Span::raw("  "),
        Span::styled("┃", body),
        Span::styled(blush.to_string(), blush_s),
        Span::styled(face.left_eye.to_string(), eye_s),
        Span::raw(" "),
        Span::styled(face.mouth.to_string(), mouth_s),
        Span::raw(" "),
        Span::styled(face.right_eye.to_string(), eye_s),
        Span::styled(blush.to_string(), blush_s),
        Span::styled("┃", body),
        Span::raw("  "),
        Span::styled(
            message.to_string(),
            Style::default().fg(theme.text_dim).add_modifier(Modifier::ITALIC),
        ),
    ])
}

pub fn render_tiny<'a>(expr: Expression, theme: &Theme) -> Vec<Span<'a>> {
    let face    = face_for(expr);
    let eye_s   = Style::default().fg((face.eye_color)(theme));
    let mouth_s = Style::default().fg((face.mouth_color)(theme));
    vec![
        Span::styled(face.left_eye.to_string(), eye_s),
        Span::raw(" "),
        Span::styled(face.mouth.to_string(), mouth_s),
        Span::raw(" "),
        Span::styled(face.right_eye.to_string(), eye_s),
    ]
}

pub fn greeting_for_hour(hour: u32) -> (Expression, &'static str) {
    match hour {
        5..=8   => (Expression::Sleepy,  "Early bird~ Let's find something cozy…"),
        9..=11  => (Expression::Waving,  "Good morning! Ready for a screening?"),
        12..=14 => (Expression::Happy,   "Afternoon vibes~ What's on the queue?"),
        15..=17 => (Expression::Idle,    "Golden hour — perfect for a marathon~"),
        18..=20 => (Expression::Excited, "Prime time! The stage is set."),
        21..=23 => (Expression::Happy,   "Late night session? I'm right here~"),
        _       => (Expression::Sleepy,  "Up past midnight… one more episode?"),
    }
}

pub fn message_for_search_state(
    has_results: bool,
    is_loading: bool,
    input_len: usize,
) -> (Expression, &'static str) {
    if is_loading       { (Expression::Searching, "Sniffing out titles…") }
    else if has_results { (Expression::Excited,   "Found some good ones!") }
    else if input_len >= 2 { (Expression::Sad,    "Hmm, couldn't find that one…") }
    else                { (Expression::Happy,     "Type a title and I'll find it~") }
}

pub fn message_for_detail(loading: bool, has_eps: bool) -> (Expression, &'static str) {
    if loading      { (Expression::Searching, "Fetching episodes…") }
    else if !has_eps { (Expression::Thinking, "No episodes — strange.") }
    else            { (Expression::Happy,     "Pick an episode!") }
}

pub fn message_for_playing() -> (Expression, &'static str) {
    (Expression::Watching, "Enjoy the show~")
}

pub fn message_for_history(empty: bool) -> (Expression, &'static str) {
    if empty { (Expression::Thinking, "No history yet — let's fix that!") }
    else     { (Expression::Happy,    "Everything you've screened~") }
}

pub fn message_for_help() -> (Expression, &'static str) {
    (Expression::Waving, "Here's everything I know!")
}