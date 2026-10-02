#![allow(dead_code)]

use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use super::state::{AppState, Focus, ModalKind};

pub fn handle_key(app: &mut AppState, key: &KeyEvent) -> bool {
    if key.kind != KeyEventKind::Press {
        return false;
    }
    if matches!(key.code, KeyCode::Esc) {
        if app.modal.take().is_some() {
            app.focus = Focus::ProxyList;
        }
        return false;
    }
    if app.modal.is_some() {
        return quit_combo(key);
    }
    match (key.code, key.modifiers) {
        (KeyCode::Tab, KeyModifiers::NONE) => {
            app.focus = match app.focus {
                Focus::ProxyList => Focus::Side,
                Focus::Side => Focus::ProxyList,
                Focus::Modal => Focus::ProxyList,
            };
        }
        (KeyCode::BackTab, _) => {
            app.focus = match app.focus {
                Focus::ProxyList => Focus::Side,
                Focus::Side => Focus::ProxyList,
                Focus::Modal => Focus::ProxyList,
            };
        }
        (KeyCode::Up, _) => app.move_cursor(-1),
        (KeyCode::Down, _) => app.move_cursor(1),
        (KeyCode::Enter, _) => {
            app.master_dirty = true;
            app.set_toast("Toggling master…");
        }
        (KeyCode::Char(' '), KeyModifiers::NONE) => apply_cursor_node(app),
        (KeyCode::Char('1'), KeyModifiers::NONE) => {
            set_mode(app, rclash_core_manager::api::ProxyMode::Rule)
        }
        (KeyCode::Char('2'), KeyModifiers::NONE) => {
            set_mode(app, rclash_core_manager::api::ProxyMode::Global)
        }
        (KeyCode::Char('3'), KeyModifiers::NONE) => {
            set_mode(app, rclash_core_manager::api::ProxyMode::Direct)
        }
        (KeyCode::Char('l') | KeyCode::Char('д'), KeyModifiers::NONE) => {
            open_modal(app, ModalKind::Logs)
        }
        (KeyCode::Char('s') | KeyCode::Char('ы'), KeyModifiers::NONE) => {
            open_modal(app, ModalKind::Settings)
        }
        (KeyCode::Char('c') | KeyCode::Char('с'), KeyModifiers::NONE) => {
            open_modal(app, ModalKind::ConfigEditor)
        }
        (KeyCode::Char('p') | KeyCode::Char('з'), KeyModifiers::NONE) => {
            app.ping_requested = true;
            app.set_toast("Ping requested");
        }
        _ => return quit_combo(key),
    }
    false
}

fn quit_combo(key: &KeyEvent) -> bool {
    matches!(
        (key.code, key.modifiers),
        (KeyCode::Char('c'), KeyModifiers::CONTROL)
            | (KeyCode::Char('q') | KeyCode::Char('й'), KeyModifiers::NONE)
    )
}

fn set_mode(app: &mut AppState, mode: rclash_core_manager::api::ProxyMode) {
    app.mode = mode;
    app.mode_dirty = true;
    app.set_toast(format!("Mode: {}", mode.as_str()));
}

fn open_modal(app: &mut AppState, modal: ModalKind) {
    app.modal = Some(modal);
    app.focus = Focus::Modal;
}

fn apply_cursor_node(app: &mut AppState) {
    let node = match app.cursor_node() {
        Some(n) => n.clone(),
        None => return,
    };
    for n in &mut app.proxies {
        n.selected = n.name == node.name;
    }
    app.selected_proxy = node.name.clone();
    let group = if app.mode == rclash_core_manager::api::ProxyMode::Global {
        "GLOBAL".to_owned()
    } else if app.selected_group.is_empty() {
        node.group.clone()
    } else {
        app.selected_group.clone()
    };
    app.select_request = Some((group, node.name.clone()));
    app.set_toast(format!("Selected {}", node.name));
}

pub fn handle_mouse(app: &mut AppState, mouse: &MouseEvent) {
    if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
        return;
    }
    let (x, y) = (mouse.column, mouse.row);
    if let Some(rect) = app.proxy_rows_rect {
        if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
            app.focus = Focus::ProxyList;
            let idx = app.proxy_scroll + (y - rect.y) as usize;
            if idx < app.proxies.len() {
                app.proxy_cursor = idx;
                app.clamp_scroll();
            }
            return;
        }
    }
    if let Some(rect) = app.proxy_panel {
        if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
            app.focus = Focus::ProxyList;
            return;
        }
    }
    if let Some(rect) = app.side_panel {
        if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
            app.focus = Focus::Side;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::state::ProxyNode;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    fn mouse_at(x: u16, y: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::empty(),
        }
    }

    fn state_with_nodes() -> AppState {
        AppState {
            proxies: (0..5)
                .map(|i| ProxyNode {
                    name: format!("n{i}"),
                    ..Default::default()
                })
                .collect(),
            proxy_visible: 10,
            proxy_panel: Some(ratatui::layout::Rect::new(0, 1, 60, 28)),
            proxy_rows_rect: Some(ratatui::layout::Rect::new(1, 4, 50, 5)),
            side_panel: Some(ratatui::layout::Rect::new(60, 1, 40, 28)),
            ..Default::default()
        }
    }

    #[test]
    fn release_and_repeat_are_ignored() {
        let mut app = AppState::default();
        for kind in [KeyEventKind::Release, KeyEventKind::Repeat] {
            let key = KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
                kind,
                state: KeyEventState::empty(),
            };
            assert!(!handle_key(&mut app, &key));
        }
        assert_eq!(app.proxy_cursor, 0);
    }

    #[test]
    fn tab_cycles_panels() {
        let mut app = AppState::default();
        assert_eq!(app.focus, Focus::ProxyList);
        assert!(!handle_key(
            &mut app,
            &key(KeyCode::Tab, KeyModifiers::NONE)
        ));
        assert_eq!(app.focus, Focus::Side);
        assert!(!handle_key(
            &mut app,
            &key(KeyCode::Tab, KeyModifiers::NONE)
        ));
        assert_eq!(app.focus, Focus::ProxyList);
        assert!(!handle_key(
            &mut app,
            &key(KeyCode::BackTab, KeyModifiers::SHIFT)
        ));
        assert_eq!(app.focus, Focus::Side);
    }

    #[test]
    fn arrows_move_and_space_selects() {
        let mut app = state_with_nodes();
        assert!(!handle_key(
            &mut app,
            &key(KeyCode::Down, KeyModifiers::NONE)
        ));
        assert!(!handle_key(
            &mut app,
            &key(KeyCode::Down, KeyModifiers::NONE)
        ));
        assert_eq!(app.proxy_cursor, 2);
        assert!(!handle_key(&mut app, &key(KeyCode::Up, KeyModifiers::NONE)));
        assert_eq!(app.proxy_cursor, 1);
        assert!(!handle_key(
            &mut app,
            &key(KeyCode::Char(' '), KeyModifiers::NONE)
        ));
        assert_eq!(app.selected_proxy, "n1");
        assert!(app.proxies[1].selected);
        assert!(!app.proxies[0].selected);
    }

    #[test]
    fn enter_requests_master_and_digits_set_mode() {
        let mut app = AppState::default();
        assert!(!handle_key(
            &mut app,
            &key(KeyCode::Enter, KeyModifiers::NONE)
        ));
        assert!(app.master_dirty);
        assert!(!app.master_enabled);
        for (ch, mode) in [
            ('1', rclash_core_manager::api::ProxyMode::Rule),
            ('2', rclash_core_manager::api::ProxyMode::Global),
            ('3', rclash_core_manager::api::ProxyMode::Direct),
        ] {
            assert!(!handle_key(
                &mut app,
                &key(KeyCode::Char(ch), KeyModifiers::NONE)
            ));
            assert_eq!(app.mode, mode);
        }
    }

    #[test]
    fn letters_open_modals_in_both_layouts() {
        for ch in ['l', 'д', 's', 'ы', 'c', 'с'] {
            let mut app = AppState::default();
            assert!(!handle_key(
                &mut app,
                &key(KeyCode::Char(ch), KeyModifiers::NONE)
            ));
            assert!(app.modal.is_some());
            assert_eq!(app.focus, Focus::Modal);
            assert!(!handle_key(
                &mut app,
                &key(KeyCode::Esc, KeyModifiers::NONE)
            ));
            assert!(app.modal.is_none());
        }
        let mut app = AppState::default();
        assert!(!handle_key(
            &mut app,
            &key(KeyCode::Char('p'), KeyModifiers::NONE)
        ));
        assert!(app.ping_requested);
        let mut app = AppState::default();
        assert!(!handle_key(
            &mut app,
            &key(KeyCode::Char('з'), KeyModifiers::NONE)
        ));
        assert!(app.ping_requested);
    }

    #[test]
    fn mouse_click_selects_row_and_panels() {
        let mut app = state_with_nodes();
        handle_mouse(&mut app, &mouse_at(5, 6));
        assert_eq!(app.focus, Focus::ProxyList);
        assert_eq!(app.proxy_cursor, 2);
        handle_mouse(&mut app, &mouse_at(70, 10));
        assert_eq!(app.focus, Focus::Side);
        handle_mouse(&mut app, &mouse_at(5, 2));
        assert_eq!(app.focus, Focus::ProxyList);
    }

    #[test]
    fn ctrl_c_always_quits_never_modal() {
        let mut app = AppState::default();
        assert!(handle_key(
            &mut app,
            &key(KeyCode::Char('c'), KeyModifiers::CONTROL)
        ));
        assert!(app.modal.is_none());
    }

    #[test]
    fn scroll_follows_cursor() {
        let mut app = state_with_nodes();
        app.proxy_visible = 3;
        app.move_cursor(4);
        assert_eq!(app.proxy_scroll, 2);
        app.move_cursor(-4);
        assert_eq!(app.proxy_scroll, 0);
    }
}
