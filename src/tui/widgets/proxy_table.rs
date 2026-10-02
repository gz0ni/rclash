use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use super::super::state::AppState;
use rclash_core_manager::api::delay_color;

pub const SELECTED_MARK: &str = "●";
pub const IDLE_MARK: &str = "○";

pub fn truncate_name(name: &str, max_width: usize) -> String {
    if name.width() <= max_width {
        return name.to_owned();
    }
    let mut width = 0;
    let mut end = 0;
    for (i, ch) in name.char_indices() {
        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + w + 1 > max_width {
            break;
        }
        width += w;
        end = i + ch.len_utf8();
    }
    format!("{}…", &name[..end])
}

pub fn ping_text(delay_ms: Option<u64>) -> String {
    match delay_ms {
        Some(d) => format!("{d} ms"),
        None => "—".to_owned(),
    }
}

pub fn ping_color(delay_ms: Option<u64>) -> Color {
    let (r, g, b) = delay_color(delay_ms);
    Color::Rgb(r, g, b)
}
pub fn render_table(frame: &mut Frame, area: Rect, app: &mut AppState, focused: bool) {
    let title = format!("Proxies — {}", app.proxies.len());
    if app.proxies.is_empty() {
        let empty = Paragraph::new("Waiting for data…").block(frame_block(title, focused));
        frame.render_widget(empty, area);
        return;
    }
    app.clamp_scroll();
    let rows_rect = Rect::new(
        area.x + 3,
        area.y + 2,
        area.width.saturating_sub(4),
        area.height.saturating_sub(3),
    );
    app.proxy_visible = rows_rect.height as usize;
    app.proxy_rows_rect = Some(rows_rect);
    let name_width = (area.width.saturating_sub(24)).max(8) as usize;
    let rows: Vec<Row> = app
        .proxies
        .iter()
        .skip(app.proxy_scroll)
        .map(|n| {
            let mark = if n.selected { SELECTED_MARK } else { IDLE_MARK };
            let mark_style = if n.selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(truncate_name(&n.name, name_width)),
                Cell::from(n.proto.clone()),
                Cell::from(ping_text(n.delay_ms))
                    .style(Style::default().fg(ping_color(n.delay_ms))),
                Cell::from(mark).style(mark_style),
            ])
        })
        .collect();

    let header = Row::new(vec!["Node", "Type", "Ping", "Act"]).style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
    let widths = [
        Constraint::Min(10),
        Constraint::Length(10),
        Constraint::Length(9),
        Constraint::Length(3),
    ];
    let table = Table::new(rows, widths)
        .header(header)
        .block(frame_block(title, focused))
        .row_highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("» ");

    let mut state = TableState::default();
    state.select(Some(app.proxy_cursor.saturating_sub(app.proxy_scroll)));
    frame.render_stateful_widget(table, area, &mut state);
}

fn frame_block(title: String, focused: bool) -> Block<'static> {
    use ratatui::widgets::Borders;
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(if focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_respects_double_width() {
        assert_eq!(truncate_name("abc", 10), "abc");
        assert_eq!(truncate_name("abcdef", 5), "abcd…");
        assert_eq!(truncate_name("日本語test", 7), "日本語…");
    }

    #[test]
    fn ping_text_and_color_thresholds() {
        assert_eq!(ping_text(None), "—");
        assert_eq!(ping_text(Some(42)), "42 ms");
        assert_eq!(ping_color(None), Color::Rgb(120, 120, 120));
        assert_eq!(ping_color(Some(50)), Color::Rgb(80, 200, 120));
        assert_eq!(ping_color(Some(150)), Color::Rgb(220, 180, 60));
        assert_eq!(ping_color(Some(500)), Color::Rgb(220, 80, 80));
    }
}
