mod metadata_panel;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::chrome;
use super::recommendations;
use crate::app::App;
use crate::theme::{self};

pub fn render(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let hints = [
        ("⏎", "play"),
        ("hjkl", "move"),
        ("J/K", "scroll info"),
        ("r", "recs"),
        ("d", "sub/dub"),
        ("U", "update"),
        ("R", "notes"),
        ("Esc", "back"),
    ]
    .iter()
    .flat_map(|(k, a)| theme::keyhint(k, a, t))
    .collect();

    let title = app
        .selected_anime
        .as_ref()
        .map(|a| a.title.clone())
        .unwrap_or_else(|| "detail".to_string());
    let stage = chrome::shell(f, app, &title, hints);

    // 1. Core Structural Layout: HERO | PROGRESS | CONTENT
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(18), // Hero Section (increased for padding)
            Constraint::Length(3),  // Watch Progress Bar
            Constraint::Min(0),     // Interactive Area (Episodes + Info)
        ])
        .split(stage);

    render_hero_section(f, root[0], app);
    render_wide_progress_bar(f, root[1], app);

    // 2. Interactive Area Split
    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // Episode selection
            Constraint::Length(1),      // Divider
            Constraint::Percentage(40), // Metadata / Synopsis / Recs
        ])
        .split(root[2]);

    render_episode_area(f, content[0], app);
    render_divider(f, content[1], t);
    render_info_area(f, content[2], app);
}

fn render_hero_section(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let jikan = match &app.jikan_anime {
        Some(j) => j,
        None => return,
    };

    // Add padding around the hero content
    let padded_area = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Percentage(100)])
        .split(area)[0];

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(28), // Poster
            Constraint::Length(2),  // Spacer
            Constraint::Min(0),     // Text info
        ])
        .split(padded_area);

    // 1. Poster
    if let Some(ref cover) = app.cover_art {
        let mut protocol = cover.protocol.borrow_mut();
        let image = ratatui_image::StatefulImage::default();
        f.render_stateful_widget(image, chunks[0], &mut *protocol);
    } else if app.cover_art_loading {
        let line = Line::from(vec![
            Span::styled(theme::spinner_frame(app.spinner_tick), theme::fg(t.gold)),
            Span::raw(" loading cover…"),
        ]);
        f.render_widget(Paragraph::new(line), chunks[0]);
    }

    // 2. Hero Text Info
    let mut hero_lines = Vec::new();

    // Title in bold primary color
    hero_lines.push(Line::from(vec![
        Span::styled(
            jikan.display_title().to_uppercase(),
            Style::default().fg(t.gold).add_modifier(Modifier::BOLD),
        ),
    ]));

    // Subtitle / Native title
    if let Some(jp) = &jikan.title_japanese {
        hero_lines.push(Line::from(vec![Span::styled(jp, theme::fg(t.text_dim))]));
    }
    hero_lines.push(Line::raw(""));

    // Badge row
    let mut badges = Vec::new();
    if let Some(score) = jikan.score {
        badges.push(Span::styled(format!(" ★ {:.1} ", score), Style::default().bg(t.gold).fg(t.bg).add_modifier(Modifier::BOLD)));
        badges.push(Span::raw("  "));
    }
    if let Some(atype) = &jikan.anime_type {
        badges.push(Span::styled(format!(" {} ", atype), Style::default().bg(t.moon).fg(t.bg).add_modifier(Modifier::BOLD)));
        badges.push(Span::raw("  "));
    }
    if let Some(status) = &jikan.status {
        let color = if jikan.is_currently_airing() { t.sage } else { t.moon };
        badges.push(Span::styled(format!(" {} ", status.to_uppercase()), Style::default().bg(color).fg(t.bg).add_modifier(Modifier::BOLD)));
    }
    hero_lines.push(Line::from(badges));
    hero_lines.push(Line::raw(""));

    // Genres
    let genres = jikan.genre_names();
    if !genres.is_empty() {
        let mut spans = Vec::new();
        for (i, g) in genres.iter().enumerate() {
            if i > 0 { spans.push(Span::styled(" · ", theme::dim(t.text_subtle))); }
            spans.push(Span::styled(g.to_string(), theme::fg(t.moon)));
        }
        hero_lines.push(Line::from(spans));
    }
    hero_lines.push(Line::raw(""));

    // Snippet of synopsis
    if let Some(syn) = &jikan.synopsis {
        let max_w = chunks[2].width as usize;
        let snippet: String = syn.chars().take(max_w * 3).collect();
        let words: Vec<&str> = snippet.split_whitespace().collect();
        let mut line = String::new();
        let mut count = 0;
        for word in words {
            if line.len() + word.len() > max_w && count < 3 {
                hero_lines.push(Line::from(vec![Span::styled(line.clone(), theme::fg(t.text_dim))]));
                line.clear();
                count += 1;
            }
            if count >= 3 { break; }
            if !line.is_empty() { line.push(' '); }
            line.push_str(word);
        }
        if !line.is_empty() && count < 3 {
            hero_lines.push(Line::from(vec![Span::styled(line + "…", theme::fg(t.text_dim))]));
        }
    }

    f.render_widget(Paragraph::new(hero_lines), chunks[2]);
}

fn render_wide_progress_bar(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let anime = match app.selected_anime.as_ref() {
        Some(a) => a,
        None => return,
    };

    let watched = watched_eps_for(app);
    let watched_count = watched.len();
    let total = anime.episode_count as usize;
    let ratio = if total > 0 { watched_count as f64 / total as f64 } else { 0.0 };

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1),  // Spacer
            Constraint::Min(0),     // Bar
            Constraint::Length(30), // Stats
            Constraint::Length(1),  // Spacer
        ])
        .split(area);

    let bar = theme::progress_bar(ratio, layout[1].width as usize);
    f.render_widget(Paragraph::new(Line::from(Span::styled(bar, theme::fg(t.gold)))), layout[1]);

    let stats = Line::from(vec![
        Span::styled(format!("  {:.0}% COMPLETED", ratio * 100.0), Style::default().fg(t.gold).add_modifier(Modifier::BOLD)),
        Span::styled(format!("  ({}/{})", watched_count, total), theme::fg(t.text_dim)),
    ]);
    f.render_widget(Paragraph::new(stats), layout[2]);
}

fn render_info_area(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    if app.show_recommendations {
        recommendations::render(f, area, app);
        return;
    }

    let jikan = match &app.jikan_anime {
        Some(j) => j,
        None => return,
    };

    let mut lines = Vec::new();
    let max_w = area.width as usize;

    metadata_panel::append_info_block(&mut lines, jikan, t, max_w);
    lines.push(Line::raw(""));
    metadata_panel::append_relations_block(&mut lines, jikan, t, max_w);
    lines.push(Line::raw(""));
    metadata_panel::append_themes_block(&mut lines, jikan, t, max_w);

    let visible_h = area.height as usize;
    let scroll = app.synopsis_scroll.min(lines.len().saturating_sub(visible_h));
    let visible: Vec<Line<'static>> = lines.into_iter().skip(scroll).take(visible_h).collect();

    f.render_widget(Paragraph::new(visible), area);
}



/// The right panel: episodes on top (compact), synopsis below filling the rest.






fn render_divider(f: &mut Frame, area: Rect, t: &crate::theme::Theme) {
    let divider_x = area.x;
    let divider_top = area.y;
    for y in 0..area.height {
        let dot = Span::styled("│", theme::dim(t.text_subtle));
        let area = Rect::new(divider_x, divider_top + y, 1, 1);
        f.render_widget(Paragraph::new(Line::from(dot)), area);
    }
}



fn render_episode_area(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;

    if app.episodes_loading {
        let line = Line::from(vec![
            Span::raw(" "),
            Span::styled(theme::spinner_frame(app.spinner_tick), theme::fg(t.gold)),
            Span::raw(" "),
            Span::styled("gathering episodes…", theme::italic(t.text_dim)),
        ]);
        f.render_widget(Paragraph::new(line), area);
        return;
    }
    if app.episodes.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " nothing here yet",
                theme::italic(t.text_dim),
            ))),
            area,
        );
        return;
    }


    let watched = watched_eps_for(app);
    let cell_w: usize = 9;
    let inner_w = (area.width as usize).saturating_sub(4);
    let cols = (inner_w / cell_w).max(1);

    let mut lines: Vec<Line> = Vec::with_capacity(area.height as usize);
    let total = app.episodes.len();
    let rows = total.div_ceil(cols);
    let visible_grid_rows = (area.height as usize) / 2;
    let sel_row = app.episode_selected / cols;
    
    // Smooth scrolling logic
    let start_row = if sel_row >= visible_grid_rows {
        sel_row.saturating_sub(visible_grid_rows.saturating_sub(1))
    } else {
        0
    };
    let end_row = (start_row + visible_grid_rows).min(rows);

    for row in start_row..end_row {
        let mut cells: Vec<Span> = vec![Span::raw(" ")];
        let mut floor: Vec<Span> = vec![Span::raw(" ")];
        for col in 0..cols {
            let i = row * cols + col;
            if i >= total {
                break;
            }
            let ep = &app.episodes[i];
            let is_sel = i == app.episode_selected;
            let is_watched = watched.contains(ep);
            let label = if is_sel {
                format!(" EP{:^4} ", short_ep(ep))
            } else {
                format!("   {:^4} ", short_ep(ep))
            };
            let style = match (is_sel, is_watched) {
                (true, _) => Style::default()
                    .fg(t.bg)
                    .bg(t.gold)
                    .add_modifier(Modifier::BOLD),
                (false, true) => theme::fg(t.sage),
                (false, false) => theme::fg(t.text_dim),
            };
            cells.push(Span::styled(label, style));
            cells.push(Span::raw(" "));

            let mark = if is_sel {
                "▔▔▔▔▔▔▔▔"
            } else if is_watched {
                "········"
            } else {
                "        "
            };
            let mark_style = if is_sel {
                theme::fg(t.gold)
            } else {
                theme::dim(t.text_subtle)
            };
            floor.push(Span::styled(mark.to_string(), mark_style));
            floor.push(Span::raw(" "));
        }
        lines.push(Line::from(cells));
        lines.push(Line::from(floor));
    }

    f.render_widget(Paragraph::new(lines), area);
}






fn short_ep(s: &str) -> String {
    s.to_string()
}


fn watched_eps_for(app: &App) -> Vec<String> {
    let id = match app.selected_anime.as_ref() {
        Some(a) => a.id.clone(),
        None => return vec![],
    };
    app.history
        .iter()
        .filter(|h| h.anime_id == id)
        .map(|h| h.episode.clone())
        .collect()
}
