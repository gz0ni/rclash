pub mod clipboard;
pub mod event;
pub mod state;
pub mod term;

use std::sync::mpsc::Sender;
use std::time::Duration;

use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyModifiers};
use ratatui::widgets::Paragraph;

use event::Event;
use state::AppState;

const TICK_MS: u64 = 33;

fn spawn_input(tx: Sender<Event>) {
    std::thread::spawn(move || loop {
        if !crossterm::event::poll(Duration::from_millis(50)).unwrap_or(false) {
            continue;
        }
        let mapped = match crossterm::event::read() {
            Ok(CrosstermEvent::Key(key)) => Some(Event::Key(key)),
            Ok(CrosstermEvent::Mouse(mouse)) => Some(Event::Mouse(mouse)),
            Ok(CrosstermEvent::Resize(_, _)) => Some(Event::Tick),
            _ => None,
        };
        if let Some(event) = mapped {
            if tx.send(event).is_err() {
                return;
            }
        }
    });
}

fn spawn_tick(tx: Sender<Event>) {
    std::thread::spawn(move || {
        while tx.send(Event::Tick).is_ok() {
            std::thread::sleep(Duration::from_millis(TICK_MS));
        }
    });
}

fn quit_requested(key: &crossterm::event::KeyEvent) -> bool {
    matches!(
        (key.code, key.modifiers),
        (KeyCode::Char('c'), KeyModifiers::CONTROL)
            | (KeyCode::Char('q') | KeyCode::Char('й'), KeyModifiers::NONE)
    )
}

fn handle_event(app: &mut AppState, ticks: &mut u64, keys: &mut u64, event: Event) -> bool {
    match event {
        Event::Key(key) => {
            *keys += 1;
            if quit_requested(&key) {
                return true;
            }
            app.move_cursor(0);
            false
        }
        Event::Tick => {
            *ticks += 1;
            false
        }
        Event::Mouse(_) => false,
        Event::NetSample { up, down } => {
            app.push_traffic(up, down);
            false
        }
        Event::CoreLog { level, payload } => {
            app.push_core_log(level, payload);
            false
        }
        Event::CorePoll => false,
    }
}

pub fn run() -> anyhow::Result<()> {
    let mut terminal = term::enter()?;
    let (tx, rx) = event::bus();
    spawn_input(tx.clone());
    spawn_tick(tx);

    let mut app = AppState::default();
    let mut ticks: u64 = 0;
    let mut keys: u64 = 0;

    let result = (|| -> anyhow::Result<()> {
        loop {
            let event = rx.recv().map_err(|_| anyhow::anyhow!("event bus closed"))?;
            if handle_event(&mut app, &mut ticks, &mut keys, event) {
                break;
            }
            terminal.draw(|frame| {
                let text = format!(
                    "RClash TUI — step 2 event loop\nticks: {ticks}  keys: {keys}\nq — quit"
                );
                frame.render_widget(Paragraph::new(text), frame.area());
            })?;
        }
        Ok(())
    })();

    term::leave(terminal)?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState};

    fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        })
    }

    #[test]
    fn q_and_ctrl_c_quit() {
        let mut app = AppState::default();
        let (mut ticks, mut keys) = (0, 0);
        for (code, mods) in [
            (KeyCode::Char('q'), KeyModifiers::NONE),
            (KeyCode::Char('й'), KeyModifiers::NONE),
            (KeyCode::Char('c'), KeyModifiers::CONTROL),
        ] {
            assert!(handle_event(
                &mut app,
                &mut ticks,
                &mut keys,
                key(code, mods)
            ));
        }
        assert!(!handle_event(
            &mut app,
            &mut ticks,
            &mut keys,
            key(KeyCode::Char('q'), KeyModifiers::CONTROL)
        ));
        assert!(!handle_event(&mut app, &mut ticks, &mut keys, Event::Tick));
        assert_eq!(ticks, 1);
    }

    #[test]
    fn net_sample_feeds_ring() {
        let mut app = AppState::default();
        let (mut ticks, mut keys) = (0, 0);
        assert!(!handle_event(
            &mut app,
            &mut ticks,
            &mut keys,
            Event::NetSample { up: 7, down: 9 }
        ));
        assert_eq!(app.traffic.back(), Some(&(7, 9)));
    }
}
