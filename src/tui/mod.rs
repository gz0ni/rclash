pub mod clipboard;
pub mod event;
pub mod input;
pub mod layout;
pub mod net;
pub mod state;
pub mod term;
pub mod widgets;

use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use crossterm::event::Event as CrosstermEvent;

use event::Event;
use state::AppState;

const TICK_MS: u64 = 33;
const POLL_SECS: u64 = 2;

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
        Event::NetSample {
            up,
            down,
            up_total,
            down_total,
        } => {
            app.push_traffic(up, down);
            app.up_total = up_total;
            app.down_total = down_total;
            false
        }
        Event::CoreLog { level, payload } => {
            app.push_core_log(level, payload);
            false
        }
        Event::CorePoll => false,
        Event::CoreStatus { alive, version } => {
            let was_alive = app.core_alive;
            app.core_alive = alive;
            app.core_version = version;
            if was_alive && !alive && app.master_enabled {
                app.master_enabled = false;
                persist_master(false);
                app.set_toast("Core is down");
            }
            false
        }
        Event::Proxies(data) => {
            let delays: std::collections::HashMap<String, Option<u64>> = app
                .proxies
                .iter()
                .map(|n| (n.name.clone(), n.delay_ms))
                .collect();
            if app.selected_group != data.group {
                app.selected_group.clone_from(&data.group);
                persist_group(&app.selected_group);
            }
            app.selected_proxy = data.now.clone();
            app.proxies = data.nodes;
            for n in &mut app.proxies {
                if let Some(d) = delays.get(n.name.as_str()) {
                    n.delay_ms = *d;
                }
            }
            if !app.proxies.is_empty() {
                app.proxy_cursor = app.proxy_cursor.min(app.proxies.len() - 1);
                app.clamp_scroll();
            }
            false
        }
        Event::PingDone(results) => {
            for (name, delay) in results {
                if let Some(n) = app.proxies.iter_mut().find(|n| n.name == name) {
                    n.delay_ms = delay;
                }
            }
            app.ping_requested = false;
            false
        }
        Event::IpDone { ip, country } => {
            app.ip_text = ip;
            app.ip_country = country;
            false
        }
        Event::ToastMsg(text) => {
            app.set_toast(text);
            false
        }
        Event::MasterDone { on } => {
            app.master_enabled = on;
            persist_master(on);
            false
        }
    }
}

fn persist_master(on: bool) {
    let mut cfg = rclash_config::load_app_config();
    cfg.master_enabled = on;
    let _ = rclash_config::save_app_config(&cfg);
}

fn persist_mode(app: &AppState) {
    let mut cfg = rclash_config::load_app_config();
    cfg.mode = net::proxy_mode_to_core(app.mode);
    let _ = rclash_config::save_app_config(&cfg);
}

fn persist_group(group: &str) {
    let mut cfg = rclash_config::load_app_config();
    cfg.selected_group = Some(group.to_owned());
    let _ = rclash_config::save_app_config(&cfg);
}

fn drain_intents(
    app: &mut AppState,
    action_tx: &Sender<net::Action>,
    event_tx: &Sender<Event>,
    base: &str,
    secret: &str,
) {
    if let Some((group, name)) = app.select_request.take() {
        let _ = action_tx.send(net::Action::Select {
            group,
            name,
            mode: app.mode,
        });
    }
    if app.mode_dirty {
        app.mode_dirty = false;
        persist_mode(app);
        let _ = action_tx.send(net::Action::SetMode {
            mode: app.mode,
            group: app.selected_group.clone(),
        });
    }
    if app.master_dirty {
        app.master_dirty = false;
        let _ = action_tx.send(net::Action::SetMaster(!app.master_enabled));
    }
    if app.ping_requested {
        app.ping_requested = false;
        let names: Vec<String> = app.proxies.iter().map(|n| n.name.clone()).collect();
        net::spawn_ping_burst(event_tx.clone(), base.to_owned(), secret.to_owned(), names);
    }
}

pub fn run() -> anyhow::Result<()> {
    let mut terminal = term::enter()?;
    let (tx, rx) = event::bus();
    spawn_input(tx.clone());
    spawn_tick(tx.clone());
    let action_tx = net::spawn_worker(tx.clone());

    let cfg = rclash_config::load_app_config();
    let base = rclash_config::api_base(&cfg.external_controller);
    let secret = cfg.core_secret.clone().unwrap_or_default();
    net::spawn_ws(tx.clone(), &base, &secret, net::WsKind::Traffic);
    net::spawn_ws(tx.clone(), &base, &secret, net::WsKind::Logs);

    let mut app = AppState {
        master_enabled: cfg.master_enabled,
        mode: net::core_mode_to_proxy(cfg.mode),
        proxy_enabled: cfg.proxy_enabled,
        tun_enabled: cfg.tun_enabled,
        selected_group: cfg.selected_group.clone().unwrap_or_default(),
        ..Default::default()
    };

    let mut last_poll: Option<Instant> = None;
    let mut last_ip_proxy = String::new();
    let _ = action_tx.send(net::Action::Poll {
        mode: app.mode,
        group: app.selected_group.clone(),
    });
    if app.master_enabled {
        let _ = action_tx.send(net::Action::SetMaster(true));
    }

    let result = (|| -> anyhow::Result<()> {
        loop {
            let event = rx.recv().map_err(|_| anyhow::anyhow!("event bus closed"))?;
            if handle_event(&mut app, event) {
                break;
            }
            drain_intents(&mut app, &action_tx, &tx, &base, &secret);
            let due = last_poll.is_none_or(|t| t.elapsed() >= Duration::from_secs(POLL_SECS));
            if due {
                last_poll = Some(Instant::now());
                let _ = action_tx.send(net::Action::Poll {
                    mode: app.mode,
                    group: app.selected_group.clone(),
                });
            }
            if app.core_alive && app.selected_proxy != last_ip_proxy {
                last_ip_proxy = app.selected_proxy.clone();
                let _ = action_tx.send(net::Action::LookupIp);
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
        assert!(!handle_event(
            &mut app,
            Event::NetSample {
                up: 7,
                down: 9,
                up_total: 70,
                down_total: 90,
            }
        ));
        assert_eq!(app.traffic.back(), Some(&(7, 9)));
        assert_eq!(app.up_total, 70);
        assert_eq!(app.down_total, 90);
    }
}
