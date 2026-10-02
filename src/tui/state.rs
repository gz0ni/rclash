#![allow(dead_code)]

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use ratatui::layout::Rect;
use rclash_core_manager::api::ProxyMode;

pub const TRAFFIC_CAP: usize = 100;
pub const LOG_CAP: usize = 2000;
pub const TOAST_TTL: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    #[default]
    ProxyList,
    Side,
    Modal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    SubscriptionUrl,
    RawLink,
    ImportText,
    FilePath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalKind {
    Profiles,
    ConfigEditor,
    Logs,
    Settings,
    Input(InputKind),
}

#[derive(Debug, Clone, Default)]
pub struct ProxyNode {
    pub name: String,
    pub group: String,
    pub flag: String,
    pub proto: String,
    pub delay_ms: Option<u64>,
    pub selected: bool,
}

#[derive(Debug, Clone)]
pub struct CoreLogLine {
    pub level: String,
    pub payload: String,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub text: String,
    pub until: Instant,
}

#[derive(Debug)]
pub struct AppState {
    pub focus: Focus,
    pub modal: Option<ModalKind>,
    pub core_alive: bool,
    pub core_version: String,
    pub master_enabled: bool,
    pub proxy_enabled: bool,
    pub tun_enabled: bool,
    pub mode: ProxyMode,
    pub proxies: Vec<ProxyNode>,
    pub proxy_cursor: usize,
    pub proxy_scroll: usize,
    pub proxy_visible: usize,
    pub proxy_panel: Option<Rect>,
    pub proxy_rows_rect: Option<Rect>,
    pub side_panel: Option<Rect>,
    pub ping_requested: bool,
    pub selected_proxy: String,
    pub selected_group: String,
    pub traffic: VecDeque<(u64, u64)>,
    pub up_total: u64,
    pub down_total: u64,
    pub ip_text: String,
    pub ip_country: String,
    pub core_logs: VecDeque<CoreLogLine>,
    pub toast: Option<Toast>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            focus: Focus::default(),
            modal: None,
            core_alive: false,
            core_version: String::new(),
            master_enabled: false,
            proxy_enabled: false,
            tun_enabled: false,
            mode: ProxyMode::Rule,
            proxies: Vec::new(),
            proxy_cursor: 0,
            proxy_scroll: 0,
            proxy_visible: 0,
            proxy_panel: None,
            proxy_rows_rect: None,
            side_panel: None,
            ping_requested: false,
            selected_proxy: String::new(),
            selected_group: String::new(),
            traffic: VecDeque::with_capacity(TRAFFIC_CAP),
            up_total: 0,
            down_total: 0,
            ip_text: String::from("—"),
            ip_country: String::new(),
            core_logs: VecDeque::with_capacity(LOG_CAP),
            toast: None,
        }
    }
}

impl AppState {
    pub fn push_traffic(&mut self, up: u64, down: u64) {
        if self.traffic.len() >= TRAFFIC_CAP {
            self.traffic.pop_front();
        }
        self.traffic.push_back((up, down));
    }

    pub fn push_core_log(&mut self, level: String, payload: String) {
        if self.core_logs.len() >= LOG_CAP {
            self.core_logs.pop_front();
        }
        self.core_logs.push_back(CoreLogLine { level, payload });
    }

    pub fn set_toast(&mut self, text: impl Into<String>) {
        self.toast = Some(Toast {
            text: text.into(),
            until: Instant::now() + TOAST_TTL,
        });
    }

    pub fn visible_toast(&mut self) -> Option<&str> {
        let expired = self
            .toast
            .as_ref()
            .is_some_and(|t| Instant::now() >= t.until);
        if expired {
            self.toast = None;
        }
        self.toast.as_ref().map(|t| t.text.as_str())
    }

    pub fn move_cursor(&mut self, delta: isize) {
        if self.proxies.is_empty() {
            self.proxy_cursor = 0;
            return;
        }
        let next = self.proxy_cursor as isize + delta;
        self.proxy_cursor = next.clamp(0, self.proxies.len() as isize - 1) as usize;
        self.clamp_scroll();
    }

    pub fn clamp_scroll(&mut self) {
        let visible = self.proxy_visible.max(1);
        if self.proxy_cursor < self.proxy_scroll {
            self.proxy_scroll = self.proxy_cursor;
        } else if self.proxy_cursor >= self.proxy_scroll + visible {
            self.proxy_scroll = self.proxy_cursor - visible + 1;
        }
        let max_scroll = self.proxies.len().saturating_sub(visible);
        self.proxy_scroll = self.proxy_scroll.min(max_scroll);
    }

    pub fn cursor_node(&self) -> Option<&ProxyNode> {
        self.proxies.get(self.proxy_cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str) -> ProxyNode {
        ProxyNode {
            name: name.to_owned(),
            ..ProxyNode::default()
        }
    }

    #[test]
    fn traffic_ring_caps_and_orders() {
        let mut s = AppState::default();
        for i in 0..(TRAFFIC_CAP + 10) as u64 {
            s.push_traffic(i, i * 2);
        }
        assert_eq!(s.traffic.len(), TRAFFIC_CAP);
        assert_eq!(s.traffic.front(), Some(&(10, 20)));
        assert_eq!(s.traffic.back(), Some(&(109, 218)));
    }

    #[test]
    fn core_log_ring_caps() {
        let mut s = AppState::default();
        for i in 0..LOG_CAP + 5 {
            s.push_core_log("info".to_owned(), format!("line {i}"));
        }
        assert_eq!(s.traffic.len(), 0);
        assert_eq!(s.core_logs.len(), LOG_CAP);
        assert_eq!(s.core_logs.front().unwrap().payload, "line 5");
    }

    #[test]
    fn cursor_clamps_on_edges() {
        let mut s = AppState::default();
        s.move_cursor(1);
        assert_eq!(s.proxy_cursor, 0);
        s.proxies = vec![node("a"), node("b"), node("c")];
        s.move_cursor(99);
        assert_eq!(s.proxy_cursor, 2);
        s.move_cursor(-99);
        assert_eq!(s.proxy_cursor, 0);
        s.move_cursor(1);
        assert_eq!(s.cursor_node().unwrap().name, "b");
    }

    #[test]
    fn toast_shows_then_expires() {
        let mut s = AppState::default();
        assert_eq!(s.visible_toast(), None);
        s.set_toast("node switched");
        assert_eq!(s.visible_toast(), Some("node switched"));
        s.toast.as_mut().unwrap().until = Instant::now() - Duration::from_secs(1);
        assert_eq!(s.visible_toast(), None);
    }
}
