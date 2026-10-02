pub mod clipboard;
pub mod event;
pub mod input;
pub mod layout;
pub mod state;
pub mod term;
pub mod widgets;

use std::sync::mpsc::Sender;
use std::time::Duration;

use crossterm::event::Event as CrosstermEvent;

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

fn handle_event(app: &mut AppState, event: Event) -> bool {
    match event {
        Event::Key(key) => input::handle_key(app, &key),
        Event::Mouse(mouse) => {
            input::handle_mouse(app, &mouse);
            false
        }
        Event::Tick => false,
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

    let result = (|| -> anyhow::Result<()> {
        loop {
            let event = rx.recv().map_err(|_| anyhow::anyhow!("event bus closed"))?;
            if handle_event(&mut app, event) {
                break;
            }
            terminal.draw(|frame| layout::render(frame, &mut app))?;
        }
        Ok(())
    })();

    term::leave(terminal)?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

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
        for (code, mods) in [
            (KeyCode::Char('q'), KeyModifiers::NONE),
            (KeyCode::Char('й'), KeyModifiers::NONE),
            (KeyCode::Char('c'), KeyModifiers::CONTROL),
        ] {
            assert!(handle_event(&mut app, key(code, mods)));
        }
        assert!(!handle_event(
            &mut app,
            key(KeyCode::Char('q'), KeyModifiers::CONTROL)
        ));
        assert!(!handle_event(&mut app, Event::Tick));
    }

    #[test]
    fn net_sample_feeds_ring() {
        let mut app = AppState::default();
        assert!(!handle_event(&mut app, Event::NetSample { up: 7, down: 9 }));
        assert_eq!(app.traffic.back(), Some(&(7, 9)));
    }
}
