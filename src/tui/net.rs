use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use rclash_core_manager::api::ProxyMode;

use super::event::Event;
use super::state::ProxyNode;

pub const TEST_URL: &str = "http://www.gstatic.com/generate_204";
pub const PING_TIMEOUT_MS: u64 = 5000;
pub const IP_API_URL: &str = "http://ip-api.com/json/?fields=status,message,query,countryCode";
pub const WS_RECONNECT_SECS: u64 = 3;

pub fn core_mode_to_proxy(mode: rclash_config::CoreMode) -> ProxyMode {
    match mode {
        rclash_config::CoreMode::Rule => ProxyMode::Rule,
        rclash_config::CoreMode::Global => ProxyMode::Global,
        rclash_config::CoreMode::Direct => ProxyMode::Direct,
    }
}

pub fn proxy_mode_to_core(mode: ProxyMode) -> rclash_config::CoreMode {
    match mode {
        ProxyMode::Rule => rclash_config::CoreMode::Rule,
        ProxyMode::Global => rclash_config::CoreMode::Global,
        ProxyMode::Direct => rclash_config::CoreMode::Direct,
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProxiesData {
    pub nodes: Vec<ProxyNode>,
    pub groups: Vec<String>,
    pub group: String,
    pub now: String,
}

fn is_url_test(group_type: &str) -> bool {
    group_type.to_ascii_lowercase().replace(['-', '_'], "") == "urltest"
}

fn entry_str(entry: &serde_json::Value, key: &str) -> String {
    entry
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned()
}

pub fn parse_proxies(value: &serde_json::Value, prev_group: &str, mode: ProxyMode) -> ProxiesData {
    let mut out = ProxiesData::default();
    let Some(root) = value.get("proxies").and_then(|v| v.as_object()) else {
        return out;
    };
    let mut groups: Vec<(String, Vec<String>, String, String)> = Vec::new();
    for (name, entry) in root {
        if name == "GLOBAL" {
            continue;
        }
        let Some(all) = entry.get("all").and_then(|a| a.as_array()) else {
            continue;
        };
        let members: Vec<String> = all
            .iter()
            .filter_map(|m| m.as_str().map(|s| s.to_owned()))
            .collect();
        groups.push((
            name.clone(),
            members,
            entry_str(entry, "now"),
            entry_str(entry, "type"),
        ));
    }
    let global_now = root
        .get("GLOBAL")
        .map(|g| entry_str(g, "now"))
        .unwrap_or_default();
    let has_global = root.contains_key("GLOBAL");

    let mut names: Vec<String> = groups.iter().map(|(g, _, _, _)| g.clone()).collect();
    if has_global {
        names.push("GLOBAL".to_owned());
    }
    out.groups = names;

    if mode == ProxyMode::Global && has_global {
        out.group = "GLOBAL".to_owned();
        out.now = global_now;
    } else if groups.iter().any(|(g, _, _, _)| g == prev_group) {
        out.group = prev_group.to_owned();
        out.now = groups
            .iter()
            .find(|(g, _, _, _)| g == prev_group)
            .map(|(_, _, n, _)| n.clone())
            .unwrap_or_default();
    } else if let Some((g, _, n, _)) = groups
        .iter()
        .filter(|(_, _, _, t)| !is_url_test(t))
        .min_by(|a, b| a.0.cmp(&b.0))
        .or(groups.first())
    {
        out.group = g.clone();
        out.now = n.clone();
    }
    let members = groups
        .iter()
        .find(|(g, _, _, _)| *g == out.group)
        .map(|(_, m, _, _)| m.clone())
        .or_else(|| {
            root.get("GLOBAL")
                .and_then(|g| g.get("all"))
                .and_then(|a| a.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|m| m.as_str().map(|s| s.to_owned()))
                        .collect()
                })
        })
        .unwrap_or_default();
    out.nodes = members
        .into_iter()
        .map(|name| {
            let proto = root
                .get(&name)
                .map(|e| entry_str(e, "type"))
                .unwrap_or_default();
            let selected = !out.now.is_empty() && name == out.now;
            ProxyNode {
                name: name.clone(),
                group: out.group.clone(),
                proto,
                delay_ms: None,
                selected,
            }
        })
        .collect();
    out
}

#[derive(Debug)]
pub enum Action {
    Poll {
        mode: ProxyMode,
        group: String,
    },
    Select {
        group: String,
        name: String,
        mode: ProxyMode,
    },
    SetMode {
        mode: ProxyMode,
        group: String,
    },
    SetMaster(bool),
    LookupIp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsKind {
    Traffic,
    Logs,
}

struct CoreCtx {
    base: String,
    secret: String,
    child: Option<std::process::Child>,
}

impl CoreCtx {
    fn refresh_conn(&mut self) {
        let cfg = rclash_config::load_app_config();
        self.base = rclash_config::api_base(&cfg.external_controller);
        self.secret = cfg.core_secret.clone().unwrap_or_default();
    }

    fn poll_snapshot(&self, tx: &Sender<Event>, mode: ProxyMode, group: &str) -> bool {
        let proxies =
            match rclash_core_manager::supervisor::proxies_blocking(&self.base, &self.secret) {
                Ok(v) => v,
                Err(e) => {
                    log::error!("proxies poll failed: {e}");
                    return send(tx, Event::ToastMsg(format!("proxies poll failed: {e}")));
                }
            };
        let data = parse_proxies(&proxies, group, mode);
        send(tx, Event::Proxies(data))
    }
}

fn send(tx: &Sender<Event>, event: Event) -> bool {
    tx.send(event).is_err()
}

fn toast(tx: &Sender<Event>, text: impl Into<String>) -> bool {
    send(tx, Event::ToastMsg(text.into()))
}

fn worker_loop(rx: &Receiver<Action>, tx: &Sender<Event>) {
    let mut ctx = CoreCtx {
        base: String::new(),
        secret: String::new(),
        child: None,
    };
    ctx.refresh_conn();
    for action in rx {
        let gone = match action {
            Action::Poll { mode, group } => do_poll(&mut ctx, tx, mode, &group),
            Action::Select { group, name, mode } => do_select(&mut ctx, tx, &group, &name, mode),
            Action::SetMode { mode, group } => do_set_mode(&mut ctx, tx, mode, &group),
            Action::SetMaster(on) => do_master(&mut ctx, tx, on),
            Action::LookupIp => do_lookup_ip(&mut ctx, tx),
        };
        if gone {
            break;
        }
    }
}

pub fn spawn_worker(event_tx: Sender<Event>) -> Sender<Action> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || worker_loop(&rx, &event_tx));
    tx
}

fn do_poll(ctx: &mut CoreCtx, tx: &Sender<Event>, mode: ProxyMode, group: &str) -> bool {
    ctx.refresh_conn();
    match rclash_core_manager::supervisor::version_blocking(&ctx.base, &ctx.secret) {
        Ok(version) => {
            if send(
                tx,
                Event::CoreStatus {
                    alive: true,
                    version,
                },
            ) {
                return true;
            }
            ctx.poll_snapshot(tx, mode, group)
        }
        Err(_) => send(
            tx,
            Event::CoreStatus {
                alive: false,
                version: String::new(),
            },
        ),
    }
}

fn do_select(
    ctx: &mut CoreCtx,
    tx: &Sender<Event>,
    group: &str,
    name: &str,
    mode: ProxyMode,
) -> bool {
    ctx.refresh_conn();
    match rclash_core_manager::supervisor::select_proxy_blocking(
        &ctx.base,
        &ctx.secret,
        group,
        name,
    ) {
        Ok(()) => {
            if let Err(e) = rclash_core_manager::supervisor::close_all_connections_blocking(
                &ctx.base,
                &ctx.secret,
            ) {
                log::warn!("close connections after select failed: {e}");
            }
            if toast(tx, format!("Selected {name}")) {
                return true;
            }
            ctx.poll_snapshot(tx, mode, group)
        }
        Err(e) => {
            log::error!("select {name} in {group} failed: {e}");
            toast(tx, format!("select failed: {e}"))
        }
    }
}

fn do_set_mode(ctx: &mut CoreCtx, tx: &Sender<Event>, mode: ProxyMode, group: &str) -> bool {
    ctx.refresh_conn();
    match rclash_core_manager::supervisor::set_mode_blocking(&ctx.base, &ctx.secret, mode.as_str())
    {
        Ok(()) => {
            if toast(tx, format!("Mode: {}", mode.as_str())) {
                return true;
            }
            ctx.poll_snapshot(tx, mode, group)
        }
        Err(e) => {
            log::error!("mode switch failed: {e}");
            toast(tx, format!("mode switch failed: {e}"))
        }
    }
}

pub fn spawn_ping_burst(event_tx: Sender<Event>, base: String, secret: String, names: Vec<String>) {
    if names.is_empty() {
        return;
    }
    let total = names.len();
    if toast(&event_tx, format!("Pinging {total} nodes…")) {
        return;
    }
    let done = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    for name in names {
        let tx = event_tx.clone();
        let (base, secret) = (base.clone(), secret.clone());
        let done = done.clone();
        std::thread::spawn(move || {
            let delay = rclash_core_manager::supervisor::test_delay_blocking(
                &base,
                &secret,
                &name,
                TEST_URL,
                PING_TIMEOUT_MS,
            );
            let finished = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            send(&tx, Event::PingDone(vec![(name, delay)]));
            if finished >= total {
                toast(&tx, "Ping done");
            }
        });
    }
}

fn pick_fallback_config(configs: &[rclash_db::Config]) -> Option<String> {
    let mut sorted: Vec<&rclash_db::Config> = configs.iter().collect();
    sorted.sort_by_key(|c| std::cmp::Reverse(c.updated_at));
    sorted
        .iter()
        .find(|c| c.name != "custom")
        .or(sorted.first())
        .map(|c| c.name.clone())
}

fn active_profile_content() -> Option<(String, Option<String>)> {
    let (name, auto): (String, bool) = rclash_db::with_shared(|conn| {
        if let Some(active) = rclash_db::get_active(conn)? {
            return Ok(Some((active.name, false)));
        }
        let Some(pick) = pick_fallback_config(&rclash_db::list_configs(conn)?) else {
            return Ok(None);
        };
        rclash_db::set_active(conn, &pick)?;
        Ok(Some((pick, true)))
    })
    .ok()??;
    let (content, raws): (Option<String>, Vec<String>) = rclash_db::with_shared(|conn| {
        Ok((
            rclash_db::get_active(conn)?.map(|c| c.content),
            rclash_db::list_raw_keys(conn)?
                .iter()
                .map(|k| k.parsed_yaml.clone())
                .collect(),
        ))
    })
    .ok()?;
    let content = content?;
    let extras: Vec<serde_yaml::Value> = raws
        .iter()
        .filter_map(|r| serde_yaml::from_str(r).ok())
        .collect();
    let merged = if extras.is_empty() {
        content
    } else {
        rclash_config::runtime::merge_extra_proxies(&content, &extras).ok()?
    };
    Some((merged, auto.then(|| name.clone())))
}

pub fn loopback_busy(port: u16) -> bool {
    std::net::TcpListener::bind(format!("127.0.0.1:{port}")).is_err()
}

fn controller_port(base: &str) -> Option<u16> {
    base.rsplit(':')
        .next()
        .and_then(|p| p.trim_end_matches('/').parse().ok())
}

pub fn looks_like_ours(version: &str) -> bool {
    version.to_ascii_lowercase().contains("rclash")
}

fn do_master(ctx: &mut CoreCtx, tx: &Sender<Event>, on: bool) -> bool {
    if !on {
        if ctx.child.is_none() {
            rclash_core_manager::supervisor::reap_orphans();
        }
        rclash_core_manager::supervisor::stop_child(&mut ctx.child);
        let cfg = rclash_config::load_app_config();
        let addr = format!("127.0.0.1:{}", cfg.mixed_port.unwrap_or(7890));
        match rclash_sys_proxy::disable_if_ours(&addr) {
            Ok(_) => {}
            Err(e) => {
                if toast(tx, format!("proxy disable failed: {e}")) {
                    return true;
                }
            }
        }
        rclash_config::guard::clear_guard();
        if toast(tx, "Core stopped") {
            return true;
        }
        return send(tx, Event::MasterDone { on: false });
    }
    let mut cfg = rclash_config::load_app_config();
    let secret = rclash_config::ensure_core_secret(&mut cfg).unwrap_or_default();
    let _ = rclash_config::save_app_config(&cfg);
    ctx.base = rclash_config::api_base(&cfg.external_controller);
    ctx.secret = secret.clone();
    if let Ok(version) = rclash_core_manager::supervisor::version_blocking(&ctx.base, &ctx.secret) {
        if looks_like_ours(&version) {
            if send(
                tx,
                Event::CoreStatus {
                    alive: true,
                    version: version.clone(),
                },
            ) {
                return true;
            }
            if toast(tx, format!("Core adopted {version}")) {
                return true;
            }
            if send(tx, Event::MasterDone { on: true }) {
                return true;
            }
            return ctx.poll_snapshot(tx, core_mode_to_proxy(cfg.mode), "");
        }
    }
    let addr = format!("127.0.0.1:{}", cfg.mixed_port.unwrap_or(7890));
    let Some((content, auto_name)) = active_profile_content() else {
        if toast(tx, "No profiles — import one") {
            return true;
        }
        return send(tx, Event::MasterDone { on: false });
    };
    if let Some(name) = auto_name {
        if toast(tx, format!("Activated profile {name}")) {
            return true;
        }
    }
    let runtime = match rclash_config::runtime::assemble_runtime_config(&content, &cfg, &secret) {
        Ok(r) => r,
        Err(e) => {
            if toast(tx, format!("runtime build failed: {e}")) {
                return true;
            }
            return send(tx, Event::MasterDone { on: false });
        }
    };
    let path = match rclash_config::runtime::write_runtime_config(&runtime) {
        Ok(p) => p,
        Err(e) => {
            if toast(tx, format!("runtime write failed: {e}")) {
                return true;
            }
            return send(tx, Event::MasterDone { on: false });
        }
    };
    let dir = match rclash_config::runtime::ensure_runtime_dir() {
        Ok(d) => d,
        Err(e) => {
            if toast(tx, format!("runtime dir failed: {e}")) {
                return true;
            }
            return send(tx, Event::MasterDone { on: false });
        }
    };
    let Some(binary) = rclash_core_manager::process::resolve_core_path() else {
        if toast(tx, "Core binary not found") {
            return true;
        }
        return send(tx, Event::MasterDone { on: false });
    };
    let mixed = cfg.mixed_port.unwrap_or(7890);
    if loopback_busy(mixed) {
        log::error!(
            "127.0.0.1:{mixed} is busy — another proxy holds it; stop it first (netstat -ano | findstr :{mixed})"
        );
        if toast(
            tx,
            format!("Port 127.0.0.1:{mixed} busy — stop the other proxy first"),
        ) {
            return true;
        }
        return send(tx, Event::MasterDone { on: false });
    }
    if let Some(port) = controller_port(&ctx.base) {
        if port != mixed && loopback_busy(port) {
            log::error!(
                "controller port 127.0.0.1:{port} is busy — another core holds it; stop it first"
            );
            if toast(tx, format!("Controller port 127.0.0.1:{port} busy")) {
                return true;
            }
            return send(tx, Event::MasterDone { on: false });
        }
    }
    if let Err(e) = rclash_updater::geodata::ensure_geodata(&dir, false) {
        log::warn!("geodata ensure failed: {e}");
    }
    let mut yaml = runtime;
    let mut stripped_total = 0usize;
    for _ in 0..60 {
        match rclash_core_manager::supervisor::precheck(&binary, &dir, &path) {
            Ok(()) => break,
            Err(rclash_core_manager::Error::ConfigTestFailed(tail)) => {
                let missing = rclash_config::runtime::parse_missing_geodata(&tail);
                if missing.is_empty() {
                    log::error!("core precheck failed: {tail}");
                    if toast(tx, format!("core start failed: config test failed: {tail}")) {
                        return true;
                    }
                    return send(tx, Event::MasterDone { on: false });
                }
                match rclash_config::runtime::strip_missing_geodata_rules(&yaml, &missing) {
                    Ok((_next, 0)) => {
                        log::error!("core precheck failed: {tail}");
                        if toast(tx, format!("core start failed: config test failed: {tail}")) {
                            return true;
                        }
                        return send(tx, Event::MasterDone { on: false });
                    }
                    Ok((next, n)) => {
                        yaml = next;
                        stripped_total += n;
                        if rclash_config::runtime::write_runtime_config(&yaml).is_err() {
                            log::error!("core precheck failed: {tail}");
                            if toast(tx, format!("core start failed: config test failed: {tail}")) {
                                return true;
                            }
                            return send(tx, Event::MasterDone { on: false });
                        }
                    }
                    Err(e) => {
                        log::error!("geodata strip failed: {e}");
                        if toast(tx, format!("core start failed: config test failed: {tail}")) {
                            return true;
                        }
                        return send(tx, Event::MasterDone { on: false });
                    }
                }
            }
            Err(e) => {
                log::error!("core precheck failed: {e}");
                if toast(tx, format!("core start failed: {e}")) {
                    return true;
                }
                return send(tx, Event::MasterDone { on: false });
            }
        }
    }
    if stripped_total > 0 && toast(tx, format!("Removed {stripped_total} stale geo lists")) {
        return true;
    }
    match rclash_core_manager::supervisor::spawn_and_wait(
        &binary,
        &dir,
        &path,
        &ctx.base,
        &ctx.secret,
    ) {
        Ok((child, version)) => {
            ctx.child = Some(child);
            if cfg.proxy_enabled {
                if let Err(e) =
                    rclash_sys_proxy::current().set(rclash_sys_proxy::ProxyState::Enabled, &addr)
                {
                    if toast(tx, format!("sys proxy failed: {e}")) {
                        return true;
                    }
                } else {
                    let _ = rclash_config::guard::write_guard(&addr);
                }
            }
            if send(
                tx,
                Event::CoreStatus {
                    alive: true,
                    version: version.clone(),
                },
            ) {
                return true;
            }
            if toast(tx, format!("Core started {version}")) {
                return true;
            }
            if send(tx, Event::MasterDone { on: true }) {
                return true;
            }
            ctx.poll_snapshot(tx, core_mode_to_proxy(cfg.mode), "")
        }
        Err(e) => {
            if toast(tx, format!("core start failed: {e}")) {
                return true;
            }
            send(tx, Event::MasterDone { on: false })
        }
    }
}

fn do_lookup_ip(ctx: &mut CoreCtx, tx: &Sender<Event>) -> bool {
    rclash_config::ensure_tls_provider();
    let cfg = rclash_config::load_app_config();
    let mixed = cfg.mixed_port.unwrap_or(7890);
    let mut builder = reqwest::blocking::Client::builder().timeout(Duration::from_secs(5));
    if let Ok(proxy) = reqwest::Proxy::http(format!("http://127.0.0.1:{mixed}")) {
        builder = builder.proxy(proxy);
    }
    let client = builder.build();
    let (ip, country) = match client {
        Ok(c) => match c.get(IP_API_URL).send().and_then(|r| r.error_for_status()) {
            Ok(resp) => {
                let v: serde_json::Value = resp.json().unwrap_or_default();
                let ok = v.get("status").and_then(|s| s.as_str()) == Some("success");
                if ok {
                    (
                        v.get("query")
                            .and_then(|q| q.as_str())
                            .unwrap_or("—")
                            .to_owned(),
                        v.get("countryCode")
                            .and_then(|q| q.as_str())
                            .unwrap_or("")
                            .to_owned(),
                    )
                } else {
                    ("—".to_owned(), String::new())
                }
            }
            Err(_) => ("—".to_owned(), String::new()),
        },
        Err(_) => ("—".to_owned(), String::new()),
    };
    let _ = &ctx.base;
    send(tx, Event::IpDone { ip, country })
}

fn ws_url(base: &str, secret: &str, path: &str) -> String {
    let mut url = base.replacen("http", "ws", 1) + path;
    if !secret.is_empty() {
        url.push_str(&format!("?token={secret}"));
    }
    url
}

fn handle_ws_text(text: &str, kind: WsKind, tx: &Sender<Event>) -> bool {
    let v: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return false,
    };
    match kind {
        WsKind::Traffic => {
            let up = v.get("up").and_then(|u| u.as_u64()).unwrap_or(0);
            let down = v.get("down").and_then(|d| d.as_u64()).unwrap_or(0);
            let up_total = v.get("upTotal").and_then(|u| u.as_u64()).unwrap_or(0);
            let down_total = v.get("downTotal").and_then(|d| d.as_u64()).unwrap_or(0);
            send(
                tx,
                Event::NetSample {
                    up,
                    down,
                    up_total,
                    down_total,
                },
            )
        }
        WsKind::Logs => {
            let level = v
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("info")
                .to_owned();
            let payload = v
                .get("payload")
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .to_owned();
            send(tx, Event::CoreLog { level, payload })
        }
    }
}

pub fn spawn_ws(event_tx: Sender<Event>, base: &str, secret: &str, kind: WsKind) {
    let url = ws_url(
        base,
        secret,
        match kind {
            WsKind::Traffic => "/traffic",
            WsKind::Logs => "/logs?level=info",
        },
    );
    std::thread::spawn(move || loop {
        if let Ok((mut socket, _)) = tungstenite::connect(&url) {
            loop {
                match socket.read() {
                    Ok(tungstenite::Message::Text(text)) => {
                        if handle_ws_text(&text, kind, &event_tx) {
                            return;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        }
        std::thread::sleep(Duration::from_secs(WS_RECONNECT_SECS));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> serde_json::Value {
        serde_json::json!({
            "proxies": {
                "GLOBAL": {"all": ["aa", "bb"], "now": "bb", "type": "Relay"},
                "vpn": {"all": ["aa", "bb"], "now": "aa", "type": "Selector"},
                "aa": {"name": "aa", "type": "Vless"},
                "bb": {"name": "bb", "type": "Trojan"}
            }
        })
    }

    #[test]
    fn parse_picks_visible_group_and_now() {
        let data = parse_proxies(&fixture(), "", ProxyMode::Rule);
        assert_eq!(data.group, "vpn");
        assert_eq!(data.now, "aa");
        assert_eq!(data.nodes.len(), 2);
        assert_eq!(data.nodes[0].name, "aa");
        assert_eq!(data.nodes[0].proto, "Vless");
        assert!(data.nodes[0].selected);
        assert!(!data.nodes[1].selected);
    }

    #[test]
    fn parse_global_mode_uses_global() {
        let data = parse_proxies(&fixture(), "", ProxyMode::Global);
        assert_eq!(data.group, "GLOBAL");
        assert_eq!(data.now, "bb");
        assert!(data.nodes[1].selected);
    }

    #[test]
    fn parse_keeps_prev_group_in_response_order() {
        let data = parse_proxies(&fixture(), "vpn", ProxyMode::Direct);
        assert_eq!(data.group, "vpn");
        assert_eq!(data.groups, vec!["vpn".to_owned(), "GLOBAL".to_owned()]);
        assert_eq!(data.nodes[0].name, "aa");
        assert_eq!(data.nodes[1].name, "bb");
    }

    #[test]
    fn parse_empty_value_is_safe() {
        let data = parse_proxies(&serde_json::json!({}), "", ProxyMode::Rule);
        assert!(data.nodes.is_empty());
        assert!(data.group.is_empty());
    }

    fn real_shape_fixture() -> serde_json::Value {
        serde_json::json!({
            "proxies": {
                "DIRECT": {"name": "DIRECT", "type": "Direct", "udp": true},
                "REJECT": {"name": "REJECT", "type": "Reject", "udp": false},
                "GLOBAL": {"all": ["DIRECT", "REJECT", "aa", "bb", "vpn"], "now": "DIRECT", "type": "Selector"},
                "vpn": {"all": ["aa", "bb"], "now": "bb", "type": "Selector"},
                "aa": {"name": "aa", "type": "Shadowsocks"},
                "bb": {"name": "bb", "type": "Shadowsocks"}
            }
        })
    }

    #[test]
    fn parse_real_core_shape() {
        let data = parse_proxies(&real_shape_fixture(), "", ProxyMode::Rule);
        assert_eq!(data.groups, vec!["vpn".to_owned(), "GLOBAL".to_owned()]);
        assert_eq!(data.group, "vpn");
        assert_eq!(data.now, "bb");
        assert_eq!(data.nodes.len(), 2);
        assert_eq!(data.nodes[0].proto, "Shadowsocks");
        assert!(data.nodes[1].selected);
        assert!(!data.nodes.iter().any(|n| n.name == "DIRECT"));
        let global = parse_proxies(&real_shape_fixture(), "", ProxyMode::Global);
        assert_eq!(global.group, "GLOBAL");
        assert!(global.nodes.iter().any(|n| n.name == "DIRECT"));
    }

    #[test]
    fn nodes_keep_raw_names_verbatim() {
        let v = serde_json::json!({"proxies": {
            "vpn": {"all": ["DE Germany01-Hysteria2", "nl-ams-02", "Bobr_7"], "now": "Bobr_7", "type": "Selector"},
            "DE Germany01-Hysteria2": {"name": "DE Germany01-Hysteria2", "type": "Hysteria2"},
            "nl-ams-02": {"name": "nl-ams-02", "type": "Trojan"},
            "Bobr_7": {"name": "Bobr_7", "type": "Vless"}
        }});
        let data = parse_proxies(&v, "", ProxyMode::Rule);
        let names: Vec<&str> = data.nodes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, vec!["DE Germany01-Hysteria2", "nl-ams-02", "Bobr_7"]);
    }

    #[test]
    fn fallback_prefers_newest_non_custom() {
        let cfg = |name: &str, updated: i64| rclash_db::Config {
            id: 0,
            name: name.to_owned(),
            url: None,
            content: String::new(),
            hash: String::new(),
            is_active: false,
            created_at: 0,
            updated_at: updated,
        };
        assert_eq!(pick_fallback_config(&[]), None);
        let only_custom = vec![cfg("custom", 5)];
        assert_eq!(
            pick_fallback_config(&only_custom),
            Some("custom".to_owned())
        );
        let mixed = vec![cfg("custom", 9), cfg("old", 1), cfg("sub", 7)];
        assert_eq!(pick_fallback_config(&mixed), Some("sub".to_owned()));
    }

    #[test]
    fn default_group_skips_urltest_deterministically() {
        let v = serde_json::json!({"proxies": {
            "zz": {"all": ["a"], "now": "a", "type": "URLTest"},
            "mm": {"all": ["a"], "now": "a", "type": "Selector"},
            "aa": {"all": ["a"], "now": "a", "type": "Selector"},
            "a": {"name": "a", "type": "Vless"}
        }});
        let data = parse_proxies(&v, "", ProxyMode::Rule);
        assert_eq!(data.group, "aa");
    }

    #[test]
    fn busy_port_detected() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(loopback_busy(port));
        drop(listener);
        assert!(!loopback_busy(port));
    }

    #[test]
    fn controller_port_parsed() {
        assert_eq!(controller_port("http://127.0.0.1:9090"), Some(9090));
        assert_eq!(controller_port("http://127.0.0.1:9090/"), Some(9090));
        assert_eq!(controller_port("garbage"), None);
    }

    #[test]
    fn ours_version_recognized() {
        assert!(looks_like_ours("v0.1.0-rclash"));
        assert!(looks_like_ours("RClash dev"));
        assert!(!looks_like_ours("v1.19.0"));
        assert!(!looks_like_ours(""));
    }

    #[test]
    fn ws_url_carries_token() {
        assert_eq!(
            ws_url("http://127.0.0.1:9090", "s3", "/traffic"),
            "ws://127.0.0.1:9090/traffic?token=s3"
        );
        assert_eq!(
            ws_url("http://127.0.0.1:9090", "", "/logs?level=info"),
            "ws://127.0.0.1:9090/logs?level=info"
        );
    }
}
