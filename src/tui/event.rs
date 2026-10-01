#![allow(dead_code)]

use std::sync::mpsc::{channel, Receiver, Sender};

use crossterm::event::{KeyEvent, MouseEvent};

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Tick,
    NetSample { up: u64, down: u64 },
    CoreLog { level: String, payload: String },
    CorePoll,
}

pub fn bus() -> (Sender<Event>, Receiver<Event>) {
    channel()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bus_roundtrips_event() {
        let (tx, rx) = bus();
        tx.send(Event::Tick).unwrap();
        tx.send(Event::NetSample { up: 1, down: 2 }).unwrap();
        assert!(matches!(rx.recv().unwrap(), Event::Tick));
        assert!(matches!(
            rx.recv().unwrap(),
            Event::NetSample { up: 1, down: 2 }
        ));
    }
}
