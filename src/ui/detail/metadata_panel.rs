use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};


use crate::domain::jikan::JikanAnime;
use crate::theme::{self};









pub fn append_info_block(
    lines: &mut Vec<Line<'static>>,
    jikan: &JikanAnime,
    t: &theme::Theme,
    max_w: usize,
) {
    let studios = jikan.studio_names();
    if !studios.is_empty() {
        lines.push(info_row("Studio", &studios.join(", "), t, max_w));
    }

    let producers: Vec<&str> = jikan.producers.iter().map(|p| p.name.as_str()).collect();
    if !producers.is_empty() {
        lines.push(info_row("Producer", &producers.join(", "), t, max_w));
    }

    let licensors: Vec<&str> = jikan.licensors.iter().map(|l| l.name.as_str()).collect();
    if !licensors.is_empty() {
        lines.push(info_row("Licensor", &licensors.join(", "), t, max_w));
    }

    if let Some(duration) = &jikan.duration {
        lines.push(info_row("Duration", duration, t, max_w));
    }

    if let Some(rating) = &jikan.rating {
        lines.push(info_row("Rating", rating, t, max_w));
    }

    if let Some(source) = &jikan.source {
        lines.push(info_row("Source", source, t, max_w));
    }

    if let Some(eps) = jikan.episodes {
        lines.push(info_row("Episodes", &eps.to_string(), t, max_w));
    }

    let season_str = match (&jikan.season, jikan.year) {
        (Some(s), Some(y)) => Some(format!("{} {}", capitalize_first(s), y)),
        (Some(s), None) => Some(capitalize_first(s)),
        (None, Some(y)) => Some(y.to_string()),
        (None, None) => None,
    };
    if let Some(ref s) = season_str {
        lines.push(info_row("Season", s, t, max_w));
    }

    if let Some(ref aired_str) = jikan.aired.string {
        if !aired_str.is_empty() {
            lines.push(info_row("Aired", aired_str, t, max_w));
        }
    }

    if let Some(ref bs) = jikan.broadcast.string {
        if !bs.is_empty() {
            lines.push(info_row("Broadcast", bs, t, max_w));
        }
    }
}





pub fn append_relations_block(
    lines: &mut Vec<Line<'static>>,
    jikan: &JikanAnime,
    t: &theme::Theme,
    max_w: usize,
) {
    if jikan.relations.is_empty() {
        return;
    }

    lines.push(section_header("Relations", t));
    lines.push(Line::raw(""));

    for rel in &jikan.relations {
        for entry in &rel.entry {
            let rel_label = &rel.relation;
            let name = theme::truncate(&entry.name, max_w.saturating_sub(14));
            let type_badge = if !entry.entry_type.is_empty() {
                format!(" {}", entry.entry_type)
            } else {
                String::new()
            };

            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    format!("{:<10}", theme::truncate(rel_label, 10)),
                    theme::fg(t.text_subtle),
                ),
                Span::raw(" "),
                Span::styled(name, theme::fg(t.text)),
                if !type_badge.is_empty() {
                    Span::styled(type_badge, theme::dim(t.text_subtle))
                } else {
                    Span::raw("")
                },
            ]));
        }
    }
    lines.push(Line::raw(""));
}

pub fn append_themes_block(
    lines: &mut Vec<Line<'static>>,
    jikan: &JikanAnime,
    t: &theme::Theme,
    max_w: usize,
) {
    let theme_data = match &jikan.theme {
        Some(th) if !th.openings.is_empty() || !th.endings.is_empty() => th,
        _ => return,
    };

    lines.push(section_header("Music", t));
    lines.push(Line::raw(""));

    if !theme_data.openings.is_empty() {
        lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled("OP".to_string(), theme::fg(t.gold)),
        ]));
        for op in &theme_data.openings {
            let clean = clean_theme_string(op);
            let trunc = theme::truncate(&clean, max_w.saturating_sub(4));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("♪ ".to_string(), theme::dim(t.text_subtle)),
                Span::styled(trunc, theme::fg(t.text_dim)),
            ]));
        }
    }

    if !theme_data.endings.is_empty() {
        if !theme_data.openings.is_empty() {
            lines.push(Line::raw(""));
        }
        lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled("ED".to_string(), theme::fg(t.moon)),
        ]));
        for ed in &theme_data.endings {
            let clean = clean_theme_string(ed);
            let trunc = theme::truncate(&clean, max_w.saturating_sub(4));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("♪ ".to_string(), theme::dim(t.text_subtle)),
                Span::styled(trunc, theme::fg(t.text_dim)),
            ]));
        }
    }

    lines.push(Line::raw(""));
}

fn section_header(label: &str, t: &theme::Theme) -> Line<'static> {
    Line::from(Span::styled(
        label.to_string(),
        Style::default()
            .fg(t.text_subtle)
            .add_modifier(Modifier::BOLD),
    ))
}

fn info_row(label: &str, value: &str, t: &theme::Theme, max_w: usize) -> Line<'static> {
    let label_w = 10;
    let val_max = max_w.saturating_sub(label_w + 3);
    let val = theme::truncate(value, val_max);
    Line::from(vec![
        Span::raw(" "),
        Span::styled(
            format!("{:>w$}", label, w = label_w),
            theme::fg(t.text_subtle),
        ),
        Span::raw(" "),
        Span::styled(val, theme::fg(t.text)),
    ])
}



fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => s.to_string(),
    }
}

/// Strip Jikan theme numbering like "1: " prefix from OP/ED strings
fn clean_theme_string(s: &str) -> String {
    let trimmed = s.trim();
    // Pattern: "1: Song Title by Artist" → "Song Title by Artist"
    if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
        let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
        if let Some(rest) = rest.strip_prefix(':') {
            return rest.trim_start().to_string();
        }
    }
    trimmed.to_string()
}
