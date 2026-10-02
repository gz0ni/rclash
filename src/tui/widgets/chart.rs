use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::symbols::Marker;
use ratatui::text::Span;
use ratatui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph};
use ratatui::Frame;

use super::super::state::AppState;
use rclash_core_manager::api::format_bytes;

pub fn chart_max(samples: &[(u64, u64)]) -> f64 {
    samples
        .iter()
        .map(|(up, down)| (*up).max(*down) as f64)
        .fold(1.0, f64::max)
}

pub fn render_chart(frame: &mut Frame, area: Rect, app: &AppState, focused: bool) {
    if app.traffic.is_empty() {
        let empty = Paragraph::new("Waiting for data…").block(chart_block(focused, 0.0));
        frame.render_widget(empty, area);
        return;
    }
    let samples: Vec<(u64, u64)> = app.traffic.iter().copied().collect();
    let max = chart_max(&samples);
    let len = samples.len();
    let down: Vec<(f64, f64)> = samples
        .iter()
        .enumerate()
        .map(|(i, s)| (i as f64, s.1 as f64))
        .collect();
    let up: Vec<(f64, f64)> = samples
        .iter()
        .enumerate()
        .map(|(i, s)| (i as f64, s.0 as f64))
        .collect();
    let datasets = vec![
        Dataset::default()
            .name("down")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Green))
            .data(&down),
        Dataset::default()
            .name("up")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Blue))
            .data(&up),
    ];
    let chart = Chart::new(datasets)
        .block(chart_block(focused, max))
        .x_axis(
            Axis::default()
                .bounds([0.0, len as f64])
                .style(Style::default().fg(Color::DarkGray)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, max])
                .labels([Span::raw("0"), Span::raw(format_bytes(max as u64))])
                .style(Style::default().fg(Color::DarkGray)),
        );
    frame.render_widget(chart, area);
}

fn chart_block(focused: bool, max: f64) -> Block<'static> {
    let title = if max > 0.0 {
        format!("Traffic — peak {}", format_bytes(max as u64))
    } else {
        "Traffic".to_owned()
    };
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
    fn max_autoscales_with_floor() {
        assert_eq!(chart_max(&[]), 1.0);
        assert_eq!(chart_max(&[(0, 0)]), 1.0);
        assert_eq!(chart_max(&[(10, 40), (100, 5)]), 100.0);
    }
}
