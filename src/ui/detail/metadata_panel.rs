use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::domain::jikan::JikanAnime;
use crate::theme::{self, *};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;

    if app.jikan_loading {
        let line = Line::from(vec![
            Span::raw(" "),
            Span::styled(
                theme::spinner_frame(app.spinner_tick).to_string(),
                theme::fg(t.gold),
            ),
            Span::raw(" "),
            Span::styled("loading metadata…".to_string(), theme::italic(t.text_dim)),
        ]);
        f.render_widget(Paragraph::new(line), area);
        return;
    }

    let jikan = match &app.jikan_anime {
        Some(j) => j,
        None => {
            let line = Line::from(Span::styled(
                " no metadata available".to_string(),
                theme::italic(t.text_dim),
            ));
            f.render_widget(Paragraph::new(line), area);
            return;
        }
    };

    let max_w = area.width as usize;
    let visible_h = area.height as usize;
    let mut lines: Vec<Line<'static>> = Vec::new();

    append_title_block(&mut lines, jikan, t, max_w);
    lines.push(Line::raw(""));
    append_score_block(&mut lines, jikan, t);
    lines.push(Line::raw(""));
    append_info_block(&mut lines, jikan, t, max_w);
    lines.push(Line::raw(""));
    append_genres_block(&mut lines, jikan, t);
    lines.push(Line::raw(""));
    append_streaming_block(&mut lines, jikan, t);
    lines.push(Line::raw(""));
    append_synopsis(&mut lines, jikan, t, max_w.saturating_sub(2));

    let max_scroll = lines.len().saturating_sub(visible_h);
    let scroll = app.synopsis_scroll.min(max_scroll);
    let visible: Vec<Line<'static>> = lines.into_iter().skip(scroll).take(visible_h).collect();
    f.render_widget(Paragraph::new(visible), area);
}

fn append_title_block(
    lines: &mut Vec<Line<'static>>,
    jikan: &JikanAnime,
    t: &theme::Theme,
    max_w: usize,
) {
    let title = theme::truncate(jikan.display_title(), max_w.saturating_sub(2));
    lines.push(Line::from(vec![
        Span::styled(SPARKLE.to_string(), theme::fg(t.gold)),
        Span::raw(" "),
        Span::styled(
            title,
            Style::default().fg(t.text).add_modifier(Modifier::BOLD),
        ),
    ]));

    if let Some(jp) = &jikan.title_japanese {
        if !jp.is_empty() {
            let trunc = theme::truncate(jp, max_w.saturating_sub(2));
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(trunc, theme::fg(t.text_subtle)),
            ]));
        }
    }

    let mut badges: Vec<Span<'static>> = vec![Span::raw(" ")];
    if let Some(status) = &jikan.status {
        let (label, color) = if jikan.is_currently_airing() {
            ("AIRING", t.sage)
        } else if status.contains("Finished") {
            ("FINISHED", t.moon)
        } else {
            ("UPCOMING", t.gold)
        };
        badges.push(pill(label, color));
        badges.push(Span::raw(" "));
    }
    if let Some(atype) = &jikan.anime_type {
        badges.push(pill(atype, t.moon));
        badges.push(Span::raw(" "));
    }
    if badges.len() > 1 {
        lines.push(Line::from(badges));
    }
}

fn append_score_block(lines: &mut Vec<Line<'static>>, jikan: &JikanAnime, t: &theme::Theme) {
    if jikan.score.is_none() && jikan.rank.is_none() {
        return;
    }

    let mut spans: Vec<Span<'static>> = vec![Span::raw(" ")];

    if let Some(score) = jikan.score {
        let star_count = (score / 2.0).round() as usize;
        let stars = "★".repeat(star_count) + &"☆".repeat(5 - star_count);
        spans.push(Span::styled(stars, theme::fg(t.gold)));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            format!("{:.1}", score),
            Style::default().fg(t.text).add_modifier(Modifier::BOLD),
        ));
        if let Some(sb) = jikan.scored_by {
            spans.push(Span::styled(
                format!(" ({})", format_number(sb)),
                theme::fg(t.text_dim),
            ));
        }
        spans.push(Span::raw("  "));
    }

    if let Some(rank) = jikan.rank {
        spans.push(Span::styled(
            format!("#{}", rank),
            Style::default().fg(t.moon).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
    }

    if let Some(pop) = jikan.popularity {
        spans.push(Span::styled(format!("pop #{}", pop), theme::fg(t.text_dim)));
        spans.push(Span::raw(" "));
    }

    if let Some(members) = jikan.members {
        spans.push(Span::styled(
            format!("{} members", format_number(members)),
            theme::fg(t.text_dim),
        ));
    }

    lines.push(Line::from(spans));
}

fn append_info_block(
    lines: &mut Vec<Line<'static>>,
    jikan: &JikanAnime,
    t: &theme::Theme,
    max_w: usize,
) {
    let studios = jikan.studio_names();
    if !studios.is_empty() {
        lines.push(info_row("Studio", &studios.join(", "), t, max_w));
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

    let season_str = match (&jikan.season, jikan.year) {
        (Some(s), Some(y)) => Some(format!("{} {}", capitalize_first(s), y)),
        (Some(s), None) => Some(capitalize_first(s)),
        (None, Some(y)) => Some(y.to_string()),
        (None, None) => None,
    };
    if let Some(ref s) = season_str {
        lines.push(info_row("Season", s, t, max_w));
    }

    if let Some(ref bs) = jikan.broadcast.string {
        if !bs.is_empty() {
            lines.push(info_row("Broadcast", bs, t, max_w));
        }
    }
}

fn append_genres_block(lines: &mut Vec<Line<'static>>, jikan: &JikanAnime, t: &theme::Theme) {
    let genres = jikan.genre_names();
    let themes = jikan.theme_names();
    let demos = jikan.demographic_names();

    if genres.is_empty() && themes.is_empty() && demos.is_empty() {
        return;
    }

    if !genres.is_empty() {
        let mut spans = vec![Span::raw(" ")];
        for (i, g) in genres.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" · ".to_string(), theme::dim(t.text_subtle)));
            }
            spans.push(Span::styled(g.to_string(), theme::fg(t.moon)));
        }
        lines.push(Line::from(spans));
    }

    if !themes.is_empty() {
        let mut spans = vec![Span::raw(" ")];
        for (i, th) in themes.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" · ".to_string(), theme::dim(t.text_subtle)));
            }
            spans.push(Span::styled(th.to_string(), theme::fg(t.text_dim)));
        }
        lines.push(Line::from(spans));
    }

    if !demos.is_empty() {
        let mut spans = vec![Span::raw(" ")];
        for (i, d) in demos.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" · ".to_string(), theme::dim(t.text_subtle)));
            }
            spans.push(Span::styled(d.to_string(), theme::fg(t.text_dim)));
        }
        lines.push(Line::from(spans));
    }
}

fn append_streaming_block(lines: &mut Vec<Line<'static>>, jikan: &JikanAnime, t: &theme::Theme) {
    if jikan.streaming.is_empty() {
        return;
    }
    let mut spans = vec![Span::raw(" ")];
    for (i, s) in jikan.streaming.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        spans.push(pill(&s.name, t.sage));
    }
    lines.push(Line::from(spans));
}

fn append_synopsis(
    lines: &mut Vec<Line<'static>>,
    jikan: &JikanAnime,
    t: &theme::Theme,
    max_w: usize,
) {
    let synopsis = match &jikan.synopsis {
        Some(s) if !s.is_empty() => s,
        _ => return,
    };

    lines.push(Line::from(Span::styled(
        "Synopsis".to_string(),
        Style::default()
            .fg(t.text_subtle)
            .add_modifier(Modifier::BOLD),
    )));

    let mut pos = 0;
    let chars: Vec<char> = synopsis.chars().collect();
    while pos < chars.len() {
        let end = (pos + max_w).min(chars.len());
        let chunk: String = chars[pos..end].iter().collect();
        lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled(chunk, theme::fg(t.text_dim)),
        ]));
        pos = end;
    }
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

fn pill(label: &str, color: ratatui::style::Color) -> Span<'static> {
    Span::styled(
        format!(" {} ", label),
        Style::default()
            .fg(ratatui::style::Color::Black)
            .bg(color)
            .add_modifier(Modifier::BOLD),
    )
}

fn format_number(n: u32) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => s.to_string(),
    }
}
