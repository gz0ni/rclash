#![allow(dead_code)]

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::state::{AppState, Focus};
use super::widgets::{chart, proxy_table};
use rclash_core_manager::api::{format_bytes, ProxyMode};

pub const MIN_W: u16 = 80;
pub const MIN_H: u16 = 24;

fn status_span(app: &AppState) -> Span<'static> {
    if app.core_alive {
        Span::styled("● core up", Style::default().fg(Color::Green))
    } else {
        Span::styled("● core down", Style::default().fg(Color::Red))
    }
}

fn mode_line(mode: ProxyMode) -> Line<'static> {
    let item = |m: ProxyMode, label: &'static str| {
        if m == mode {
            Span::styled(
                format!("[{label}]"),
                Style::default().fg(Color::Yellow).bold(),
            )
        } else {
            Span::raw(format!(" {label} "))
        }
    };
    Line::from(vec![
        Span::raw("Mode:"),
        item(ProxyMode::Rule, "Rule"),
        item(ProxyMode::Global, "Global"),
        item(ProxyMode::Direct, "Direct"),
    ])
}

fn toggle(label: &'static str, on: bool) -> Span<'static> {
    if on {
        Span::styled(format!("[{label}: on]"), Style::default().fg(Color::Green))
    } else {
        Span::raw(format!("[{label}: off]"))
    }
}

fn master_line(on: bool) -> Line<'static> {
    let style = if on {
        Style::default().fg(Color::Black).bg(Color::Green).bold()
    } else {
        Style::default().fg(Color::Black).bg(Color::Red).bold()
    };
    Line::from(vec![Span::styled(
        if on {
            "  MASTER: ON  "
        } else {
            "  MASTER: OFF "
        },
        style,
    )])
}

fn title_line(app: &AppState) -> Line<'static> {
    let version = if app.core_version.is_empty() {
        String::new()
    } else {
        format!(" {}", app.core_version)
    };
    Line::from(vec![
        Span::styled("RClash v0.1.0", Style::default().fg(Color::White).bold()),
        Span::raw(version),
        Span::raw("  "),
        status_span(app),
    ])
}

fn traffic_summary(app: &AppState) -> Vec<Line<'static>> {
    let (up, down) = app.traffic.back().copied().unwrap_or((0, 0));
    vec![
        Line::from(vec![
            Span::raw("Up "),
            Span::styled(format_bytes(up), Style::default().fg(Color::Blue)),
            Span::raw("   Down "),
            Span::styled(format_bytes(down), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::raw("IP "),
            Span::raw(app.ip_text.clone()),
            Span::raw(" "),
            Span::raw(app.ip_country.clone()),
        ]),
    ]
}

fn block(title: &'static str, focused: bool) -> Block<'static> {
    let border = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border)
}

pub fn render(frame: &mut Frame, app: &AppState) {
    let area = frame.area();
    if area.width < MIN_W || area.height < MIN_H {
        let warn = Paragraph::new("Window too small — expand to at least 80x24")
            .style(Style::default().fg(Color::Red));
        frame.render_widget(warn, area);
        return;
    }

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);
    frame.render_widget(Paragraph::new(title_line(app)), root[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(root[1]);

    render_left(frame, app, cols[0]);
    render_right(frame, app, cols[1]);
}

fn render_left(frame: &mut Frame, app: &AppState, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let profile = Paragraph::new(Line::from(vec![
        Span::raw("Profile: "),
        Span::raw(app.selected_group.clone()),
    ]))
    .block(block("Profile / Group", app.focus == Focus::ProxyList));
    frame.render_widget(profile, rows[0]);

    proxy_table::render_table(frame, rows[1], app, app.focus == Focus::ProxyList);

    let counter = Paragraph::new(format!("Locations: {}", app.proxies.len()));
    frame.render_widget(counter, rows[2]);
}

fn render_right(frame: &mut Frame, app: &AppState, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let tabs = Paragraph::new(Line::from(vec![
        Span::raw("Config  "),
        Span::raw("Logs  "),
        Span::raw("Settings"),
    ]));
    frame.render_widget(tabs, rows[0]);

    chart::render_chart(frame, rows[1], app, app.focus == Focus::Side);

    let summary = Paragraph::new(traffic_summary(app));
    frame.render_widget(summary, rows[2]);

    frame.render_widget(Paragraph::new(mode_line(app.mode)), rows[3]);

    let toggles = Paragraph::new(Line::from(vec![
        toggle("Proxy", app.proxy_enabled),
        Span::raw(" "),
        toggle("TUN", app.tun_enabled),
    ]));
    frame.render_widget(toggles, rows[4]);

    frame.render_widget(Paragraph::new(master_line(app.master_enabled)), rows[5]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn harness(w: u16, h: u16) -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(w, h)).unwrap()
    }

    fn text_of(term: &Terminal<TestBackend>) -> String {
        term.backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    #[test]
    fn small_window_warns() {
        let mut term = harness(70, 20);
        let app = AppState::default();
        term.draw(|f| render(f, &app)).unwrap();
        assert!(text_of(&term).contains("too small"));
    }

    #[test]
    fn normal_window_renders_title_and_zones() {
        let mut term = harness(100, 30);
        let mut app = AppState {
            core_alive: true,
            core_version: "vtest".to_owned(),
            proxies: vec![super::super::state::ProxyNode {
                name: "node-1".to_owned(),
                selected: true,
                ..Default::default()
            }],
            ..Default::default()
        };
        app.traffic.push_back((10, 20));
        term.draw(|f| render(f, &app)).unwrap();
        let text = text_of(&term);
        assert!(text.contains("RClash"));
        assert!(text.contains("core up"));
        assert!(text.contains("node-1"));
        assert!(text.contains("Locations: 1"));
        assert!(text.contains("MASTER: OFF"));
    }

    #[test]
    fn empty_state_waits_for_data() {
        let mut term = harness(100, 30);
        let app = AppState::default();
        term.draw(|f| render(f, &app)).unwrap();
        let text = text_of(&term);
        assert!(text.contains("Waiting for data"));
        assert!(text.contains("core down"));
    }
}
