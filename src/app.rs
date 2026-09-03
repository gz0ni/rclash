use rclash_config::profile::ProfileStore;
use rclash_config::{AppConfig, LogLevel, Theme, UpdateInterval};
use rclash_core_manager::api::log_level_color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Config,
    Logs,
    Settings,
    Profiles,
    Dashboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Application,
    Core,
    Dns,
    Network,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogSource {
    Core,
    App,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfilesSection {
    Profiles,
    RawKeys,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputContext {
    Clipboard,
    Url,
    Raw,
}

pub struct RClashApp {
    pub app_config: AppConfig,
    tray: Option<crate::tray::TrayHandle>,
    profile_store: ProfileStore,
    active_profile: Option<String>,
    add_menu_open: bool,
    groups: Vec<String>,
    selected_group: String,
    proxies: Vec<ProxyItem>,
    selected_proxy: Option<String>,
    selected_mode: String,
    proxy_enabled: bool,
    tun_enabled: bool,
    core_enabled: bool,
    active_tab: Tab,
    config_content: String,
    config_interval: UpdateInterval,
    log_level: LogLevel,
    log_source: LogSource,
    log_autoscroll: bool,
    settings_tab: SettingsTab,
    profiles_section: ProfilesSection,
    input_open: bool,
    input_ctx: InputContext,
    input_text: String,
    input_error: String,
    sub_fetch: Option<poll_promise::Promise<Result<(String, String), String>>>,
    core_default_mode: String,
    core_mixed_port: String,
    core_socks_port: String,
    core_external_controller: String,
    core_keep_alive: String,
    core_geodata_loader: String,
    dns_enable: bool,
    dns_mode: String,
    dns_listen: String,
    dns_ipv6: bool,
    dns_fakeip_range: String,
    dns_nameservers_count: usize,
    dns_fallback_count: usize,
    net_bypass_count: usize,
    net_append_system_dns: bool,
    net_hosts_count: usize,
}

#[derive(Clone)]
struct ProxyItem {
    name: String,
    proxy_type: String,
    group: String,
    delay: Option<u64>,
}

const R6: u8 = 6;
const R4: u8 = 4;

const NEKO_ACCENT: egui::Color32 = egui::Color32::from_rgb(0x3B, 0x82, 0xF6);
#[allow(dead_code)]
const NEKO_ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(0x4A, 0x90, 0xFF);
#[allow(dead_code)]
const NEKO_GREEN: egui::Color32 = egui::Color32::from_rgb(0x27, 0xAE, 0x60);
#[allow(dead_code)]
const NEKO_RED: egui::Color32 = egui::Color32::from_rgb(0xE7, 0x4C, 0x3C);
const NEKO_DELAY_FAST: egui::Color32 = egui::Color32::from_rgb(0x50, 0xB4, 0x78);
const NEKO_DELAY_MID: egui::Color32 = egui::Color32::from_rgb(0xC8, 0xB4, 0x5A);
const NEKO_DELAY_SLOW: egui::Color32 = egui::Color32::from_rgb(0xE0, 0x60, 0x60);

fn country_code(name: &str) -> &'static str {
    let l = name.to_lowercase();
    if l.contains("germany") || l.contains("de-") || l.contains("germani") {
        "DE"
    } else if l.contains("poland") || l.contains("pl-") {
        "PL"
    } else if l.contains("singapore") {
        "SG"
    } else if l.contains("netherlands") || l.contains("nl-") {
        "NL"
    } else {
        "•"
    }
}

fn delay_color_for(delay: Option<u64>, theme: Theme) -> egui::Color32 {
    match delay {
        None => NEKO_RED,
        Some(d) => match theme {
            Theme::Light => {
                if d < 100 {
                    egui::Color32::from_rgb(0x0A, 0x7A, 0x3A)
                } else if d < 300 {
                    egui::Color32::from_rgb(0x8A, 0x6D, 0x00)
                } else {
                    egui::Color32::from_rgb(0xC0, 0x39, 0x2B)
                }
            }
            Theme::Dark => {
                if d < 100 {
                    NEKO_DELAY_FAST
                } else if d < 300 {
                    NEKO_DELAY_MID
                } else {
                    NEKO_DELAY_SLOW
                }
            }
        },
    }
}

fn delay_text(delay: Option<u64>) -> String {
    match delay {
        None => "n/a".to_owned(),
        Some(d) => format!("{d} ms"),
    }
}

fn log_severity(level: &str) -> u8 {
    match level.to_ascii_lowercase().as_str() {
        "error" => 3,
        "warning" | "warn" => 2,
        "info" => 1,
        _ => 0,
    }
}

fn log_threshold(level: LogLevel) -> u8 {
    match level {
        LogLevel::Silent => 4,
        LogLevel::Error => 3,
        LogLevel::Warning => 2,
        LogLevel::Info => 1,
        LogLevel::Debug => 0,
    }
}

fn log_source_matches(source: LogSource, target: &str) -> bool {
    match source {
        LogSource::All => true,
        LogSource::Core => target.to_ascii_lowercase().contains("core"),
        LogSource::App => !target.to_ascii_lowercase().contains("core"),
    }
}

fn log_entry_visible(source: LogSource, threshold: u8, entry: &crate::logger::AppLogEntry) -> bool {
    if !log_source_matches(source, &entry.target) {
        return false;
    }
    log_severity(&entry.level) >= threshold
}

fn mock_config_yaml() -> String {
    "proxies:\n  - name: Germany02-Hysteria2\n    type: hysteria2\n    server: de01.skill-up.store\n    port: 8443\nproxy-groups:\n  - name: PROXY\n    type: select\n    proxies: [Germany02-Hysteria2]\nrules:\n  - MATCH,PROXY\n"
        .to_owned()
}

fn mock_proxies() -> Vec<ProxyItem> {
    vec![
        ProxyItem {
            name: "Germany02-Hysteria2".to_owned(),
            proxy_type: "hysteria2".to_owned(),
            group: "PROXY".to_owned(),
            delay: Some(59),
        },
        ProxyItem {
            name: "Germany01-Hysteria2".to_owned(),
            proxy_type: "hysteria2".to_owned(),
            group: "PROXY".to_owned(),
            delay: Some(77),
        },
        ProxyItem {
            name: "Germany01-Trojan".to_owned(),
            proxy_type: "trojan".to_owned(),
            group: "PROXY".to_owned(),
            delay: Some(76),
        },
        ProxyItem {
            name: "Germany02-Trojan".to_owned(),
            proxy_type: "trojan".to_owned(),
            group: "PROXY".to_owned(),
            delay: None,
        },
        ProxyItem {
            name: "Germani01-Vless".to_owned(),
            proxy_type: "vless".to_owned(),
            group: "PROXY".to_owned(),
            delay: None,
        },
        ProxyItem {
            name: "Singapore01-Vmess".to_owned(),
            proxy_type: "vmess".to_owned(),
            group: "PROXY".to_owned(),
            delay: Some(142),
        },
        ProxyItem {
            name: "Poland01-Shadowsocks".to_owned(),
            proxy_type: "ss".to_owned(),
            group: "PROXY".to_owned(),
            delay: Some(88),
        },
        ProxyItem {
            name: "Netherlands01-Hysteria2".to_owned(),
            proxy_type: "hysteria2".to_owned(),
            group: "Auto".to_owned(),
            delay: Some(44),
        },
    ]
}

fn proxy_item_from_value(v: &serde_yaml::Value) -> Option<ProxyItem> {
    let m = v.as_mapping()?;
    let name = m
        .get(serde_yaml::Value::String("name".into()))?
        .as_str()?
        .to_owned();
    let proxy_type = m
        .get(serde_yaml::Value::String("type".into()))?
        .as_str()?
        .to_owned();
    Some(ProxyItem {
        name,
        proxy_type,
        group: "PROXY".to_owned(),
        delay: None,
    })
}

fn profile_name_from_url(url: &str, fallback: &str) -> String {
    let name = url
        .split('?')
        .next()
        .unwrap_or("")
        .rsplit('/')
        .next()
        .unwrap_or("")
        .trim();
    let name = if name.is_empty() { fallback } else { name };
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe = safe.trim_matches('.').trim();
    if safe.is_empty() {
        fallback.to_owned()
    } else {
        safe.to_owned()
    }
}

fn fetch_subscription(url: String, name: String) -> Result<(String, String), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("clash-verge/v2.10.2")
        .build()
        .map_err(|e| format!("client: {e}"))?;
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = resp.text().map_err(|e| format!("body: {e}"))?;
    Ok((name, text))
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Inter".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Inter.ttf")).into(),
    );
    fonts.font_data.insert(
        "JetBrainsMono".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/JetBrainsMono.ttf")).into(),
    );
    fonts.font_data.insert(
        "NotoSansSymbols2".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../assets/fonts/NotoSansSymbols2-Regular.ttf"
        ))
        .into(),
    );
    fonts.families.insert(
        egui::FontFamily::Proportional,
        vec!["Inter".to_owned(), "NotoSansSymbols2".to_owned()],
    );
    fonts.families.insert(
        egui::FontFamily::Monospace,
        vec!["JetBrainsMono".to_owned()],
    );
    ctx.set_fonts(fonts);
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(11.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(11.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::new(10.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(10.0, egui::FontFamily::Monospace),
    );
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.interact_size.y = 22.0;
    ctx.set_style(style);
}

fn card_fill_for(theme: Theme) -> egui::Color32 {
    match theme {
        Theme::Light => egui::Color32::WHITE,
        Theme::Dark => egui::Color32::from_rgb(0x2D, 0x2F, 0x33),
    }
}
fn panel_fill_for(theme: Theme) -> egui::Color32 {
    match theme {
        Theme::Light => egui::Color32::from_rgb(0xF0, 0xF2, 0xF5),
        Theme::Dark => egui::Color32::from_rgb(0x1E, 0x1E, 0x1E),
    }
}

fn theme_visuals(theme: Theme) -> egui::Visuals {
    let mut v = match theme {
        Theme::Light => {
            let mut x = egui::Visuals::light();
            x.panel_fill = panel_fill_for(theme);
            x.window_fill = card_fill_for(theme);
            x.extreme_bg_color = egui::Color32::from_rgb(0xE8, 0xEA, 0xED);
            x.faint_bg_color = panel_fill_for(theme);
            x.code_bg_color = card_fill_for(theme);
            x
        }
        Theme::Dark => {
            let mut x = egui::Visuals::dark();
            x.panel_fill = panel_fill_for(theme);
            x.window_fill = card_fill_for(theme);
            x.extreme_bg_color = egui::Color32::from_rgb(0x2A, 0x2C, 0x2F);
            x.faint_bg_color = panel_fill_for(theme);
            x.code_bg_color = card_fill_for(theme);
            x
        }
    };
    let border = border_color_for(theme);
    let card = card_fill_for(theme);
    let text = match theme {
        Theme::Light => egui::Color32::BLACK,
        Theme::Dark => egui::Color32::WHITE,
    };
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = egui::CornerRadius::same(R6);
        w.bg_stroke = egui::Stroke::new(1.0_f32, border);
        w.fg_stroke = egui::Stroke::new(1.0_f32, text);
        w.bg_fill = card;
    }
    v.widgets.hovered.bg_fill = match theme {
        Theme::Light => egui::Color32::from_rgb(0xE8, 0xEA, 0xED),
        Theme::Dark => egui::Color32::from_rgb(0x3A, 0x3D, 0x45),
    };
    v.selection.bg_fill = egui::Color32::from_rgb(0x3B, 0x82, 0xF6);
    v.selection.stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x4A, 0x90, 0xFF));
    v.override_text_color = Some(text);
    v.weak_text_color = Some(match theme {
        Theme::Light => egui::Color32::from_rgb(0x6B, 0x72, 0x80),
        Theme::Dark => egui::Color32::from_rgb(0x9A, 0xA0, 0xA6),
    });
    v
}

pub fn border_color_for(theme: Theme) -> egui::Color32 {
    match theme {
        Theme::Light => egui::Color32::from_rgb(0xD0, 0xD3, 0xD8),
        Theme::Dark => egui::Color32::from_rgb(0x3C, 0x3F, 0x41),
    }
}

fn settings_row(
    ui: &mut egui::Ui,
    border: egui::Color32,
    label: &str,
    desc: Option<&str>,
    add_control: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
        ui.label(egui::RichText::new(label).size(11.0));
        if let Some(d) = desc {
            ui.label(egui::RichText::new("?").size(10.0).weak())
                .on_hover_text(d);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(12.0);
            add_control(ui);
        });
    });
    if let Some(d) = desc {
        ui.label(egui::RichText::new(d).size(10.0).weak());
    }
    ui.add_space(6.0);
    let sep_w = ui.available_width();
    let (sep_rect, _) = ui.allocate_exact_size(egui::vec2(sep_w, 1.0), egui::Sense::hover());
    ui.painter().hline(
        sep_rect.x_range(),
        sep_rect.center().y,
        egui::Stroke::new(1.0_f32, border),
    );
    ui.add_space(6.0);
}

impl RClashApp {
    fn render_settings_screen(&mut self, ui: &mut egui::Ui) {
        let theme = self.app_config.theme;
        let card_fill = card_fill_for(theme);
        let border = border_color_for(theme);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
            if ui
                .add_sized(
                    [64.0, 22.0],
                    egui::Button::new(egui::RichText::new("< Назад").size(11.0)),
                )
                .clicked()
            {
                self.active_tab = Tab::Dashboard;
            }
            ui.label(egui::RichText::new("Настройки").size(11.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
                for (tab, label) in [
                    (SettingsTab::Network, "сеть"),
                    (SettingsTab::Dns, "dns"),
                    (SettingsTab::Core, "ядро"),
                    (SettingsTab::Application, "приложение"),
                ] {
                    let active = self.settings_tab == tab;
                    let fill = if active { NEKO_ACCENT } else { card_fill };
                    let txt_color = if active {
                        egui::Color32::WHITE
                    } else {
                        match theme {
                            Theme::Light => egui::Color32::BLACK,
                            Theme::Dark => egui::Color32::WHITE,
                        }
                    };
                    let btn =
                        egui::Button::new(egui::RichText::new(label).size(11.0).color(txt_color))
                            .fill(fill)
                            .stroke(egui::Stroke::new(
                                1.0_f32,
                                if active { NEKO_ACCENT } else { border },
                            ))
                            .corner_radius(egui::CornerRadius::same(R6));
                    if ui.add(btn).clicked() {
                        self.settings_tab = tab;
                    }
                }
            });
        });
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match self.settings_tab {
                SettingsTab::Application => self.render_settings_application(ui),
                SettingsTab::Core => self.render_settings_core(ui),
                SettingsTab::Dns => self.render_settings_dns(ui),
                SettingsTab::Network => self.render_settings_network(ui),
            });
    }

    fn render_settings_application(&mut self, ui: &mut egui::Ui) {
        let theme = self.app_config.theme;
        let card_fill = card_fill_for(theme);
        let border = border_color_for(theme);
        settings_row(ui, border, "Тема", None, |ui| {
            for th in [Theme::Dark, Theme::Light] {
                let active = theme == th;
                let fill = if active { NEKO_ACCENT } else { card_fill };
                let txt_color = if active {
                    egui::Color32::WHITE
                } else {
                    match theme {
                        Theme::Light => egui::Color32::BLACK,
                        Theme::Dark => egui::Color32::WHITE,
                    }
                };
                let btn = egui::Button::new(
                    egui::RichText::new(th.label_ru())
                        .size(11.0)
                        .color(txt_color),
                )
                .fill(fill)
                .stroke(egui::Stroke::new(
                    1.0_f32,
                    if active { NEKO_ACCENT } else { border },
                ))
                .corner_radius(egui::CornerRadius::same(R6));
                if ui.add(btn).clicked() {
                    let ctx = ui.ctx().clone();
                    self.set_theme(th, &ctx);
                }
            }
        });
        settings_row(ui, border, "Показывать график", None, |ui| {
            if ui
                .checkbox(&mut self.app_config.show_traffic_graph, "")
                .changed()
            {
                let _ = rclash_config::save_app_config(&self.app_config);
                log::info!("show_traffic_graph {}", self.app_config.show_traffic_graph);
            }
        });
        settings_row(ui, border, "Свернуть в трей", None, |ui| {
            if ui
                .checkbox(&mut self.app_config.minimize_to_tray, "")
                .changed()
            {
                let _ = rclash_config::save_app_config(&self.app_config);
                log::info!("minimize_to_tray {}", self.app_config.minimize_to_tray);
            }
        });
        settings_row(ui, border, "Уровень логов", None, |ui| {
            let before = self.app_config.log_level;
            egui::ComboBox::from_id_salt("settings_log_level")
                .selected_text(egui::RichText::new(before.as_str()).size(11.0))
                .width(90.0)
                .show_ui(ui, |ui| {
                    for lv in [
                        LogLevel::Debug,
                        LogLevel::Info,
                        LogLevel::Warning,
                        LogLevel::Error,
                        LogLevel::Silent,
                    ] {
                        ui.selectable_value(
                            &mut self.app_config.log_level,
                            lv,
                            egui::RichText::new(lv.as_str()).size(11.0),
                        );
                    }
                });
            if self.app_config.log_level != before {
                crate::logger::set_level(self.app_config.log_level.to_log_filter());
                let _ = rclash_config::save_app_config(&self.app_config);
            }
        });
    }

    fn render_settings_core(&mut self, ui: &mut egui::Ui) {
        let border = border_color_for(self.app_config.theme);
        settings_row(
            ui,
            border,
            "Режим по умолчанию",
            None,
            |ui| {
                let before = self.core_default_mode.clone();
                egui::ComboBox::from_id_salt("core_default_mode")
                    .selected_text(egui::RichText::new(&before).size(11.0))
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for m in ["rule", "global", "direct"] {
                            ui.selectable_value(
                                &mut self.core_default_mode,
                                m.to_owned(),
                                egui::RichText::new(m).size(11.0),
                            );
                        }
                    });
                if self.core_default_mode != before {
                    log::info!("default mode {}", self.core_default_mode);
                }
            },
        );
        settings_row(
            ui,
            border,
            "TUN",
            Some("Перехватывает весь трафик, требует прав администратора. Без него работает только системный прокси."),
            |ui| {
                if ui.checkbox(&mut self.app_config.tun_enabled, "").changed() {
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("tun_enabled {}", self.app_config.tun_enabled);
                }
            },
        );
        settings_row(
            ui,
            border,
            "Allow LAN",
            Some("Разрешить подключения из локальной сети (другие устройства смогут использовать прокси)."),
            |ui| {
                let mut v = self.app_config.allow_lan.unwrap_or(false);
                if ui.checkbox(&mut v, "").changed() {
                    self.app_config.allow_lan = Some(v);
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("allow_lan {v}");
                }
            },
        );
        settings_row(
            ui,
            border,
            "IPv6 трафик",
            Some("Пропускать IPv6 через прокси (выкл — только IPv4)."),
            |ui| {
                let mut v = self.app_config.ipv6.unwrap_or(false);
                if ui.checkbox(&mut v, "").changed() {
                    self.app_config.ipv6 = Some(v);
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("ipv6 {v}");
                }
            },
        );
        settings_row(
            ui,
            border,
            "Unified Delay",
            Some("Одна проверка задержки на все группы — экономит время, но менее точно."),
            |ui| {
                let mut v = self.app_config.unified_delay.unwrap_or(true);
                if ui.checkbox(&mut v, "").changed() {
                    self.app_config.unified_delay = Some(v);
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("unified_delay {v}");
                }
            },
        );
        settings_row(
            ui,
            border,
            "TCP Concurrent",
            Some("Параллельные подключения к прокси, быстрее но больше нагрузка."),
            |ui| {
                let mut v = self.app_config.tcp_concurrent.unwrap_or(true);
                if ui.checkbox(&mut v, "").changed() {
                    self.app_config.tcp_concurrent = Some(v);
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("tcp_concurrent {v}");
                }
            },
        );
        settings_row(
            ui,
            border,
            "Mixed Port",
            Some("Единый порт для HTTP и SOCKS (0 — выкл). Обычно 7890."),
            |ui| {
                let resp = ui.add_sized(
                    [80.0, 22.0],
                    egui::TextEdit::singleline(&mut self.core_mixed_port),
                );
                if resp.changed() {
                    if let Ok(v) = self.core_mixed_port.trim().parse::<u16>() {
                        self.app_config.mixed_port = Some(v);
                        let _ = rclash_config::save_app_config(&self.app_config);
                    }
                }
            },
        );
        settings_row(
            ui,
            border,
            "SOCKS Port",
            Some("Отдельный SOCKS-порт (0 — выкл, используется Mixed)."),
            |ui| {
                let resp = ui.add_sized(
                    [80.0, 22.0],
                    egui::TextEdit::singleline(&mut self.core_socks_port),
                );
                if resp.changed() {
                    if let Ok(v) = self.core_socks_port.trim().parse::<u16>() {
                        self.app_config.socks_port = Some(v);
                        let _ = rclash_config::save_app_config(&self.app_config);
                    }
                }
            },
        );
        settings_row(
            ui,
            border,
            "External Controller",
            Some("API ядра для управления (обычно 127.0.0.1:9090)."),
            |ui| {
                let resp = ui.add_sized(
                    [120.0, 22.0],
                    egui::TextEdit::singleline(&mut self.core_external_controller),
                );
                if resp.changed() && !self.core_external_controller.trim().is_empty() {
                    self.app_config.external_controller =
                        Some(self.core_external_controller.trim().to_owned());
                    let _ = rclash_config::save_app_config(&self.app_config);
                }
            },
        );
        settings_row(
            ui,
            border,
            "Keep Alive",
            Some("Интервал keep-alive в секундах для TCP."),
            |ui| {
                let resp = ui.add_sized(
                    [70.0, 22.0],
                    egui::TextEdit::singleline(&mut self.core_keep_alive),
                );
                if resp.changed() {
                    if let Ok(v) = self.core_keep_alive.trim().parse::<u32>() {
                        self.app_config.keep_alive_interval = Some(v);
                        let _ = rclash_config::save_app_config(&self.app_config);
                    }
                }
            },
        );
        settings_row(
            ui,
            border,
            "Geodata Loader",
            Some("Как грузить GeoIP/Geosite: Memory — меньше RAM, Standard — быстрее."),
            |ui| {
                let before = self.core_geodata_loader.clone();
                egui::ComboBox::from_id_salt("core_geodata_loader")
                    .selected_text(egui::RichText::new(&before).size(11.0))
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for m in ["Memory", "Standard"] {
                            ui.selectable_value(
                                &mut self.core_geodata_loader,
                                m.to_owned(),
                                egui::RichText::new(m).size(11.0),
                            );
                        }
                    });
                if self.core_geodata_loader != before {
                    self.app_config.geodata_loader = Some(self.core_geodata_loader.clone());
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("geodata_loader {}", self.core_geodata_loader);
                }
            },
        );
    }

    fn render_settings_dns(&mut self, ui: &mut egui::Ui) {
        let border = border_color_for(self.app_config.theme);
        settings_row(
            ui,
            border,
            "DNS Enable",
            Some("Включать встроенный DNS ядра."),
            |ui| {
                if ui.checkbox(&mut self.dns_enable, "").changed() {
                    log::info!("dns_enable {}", self.dns_enable);
                }
            },
        );
        settings_row(
            ui,
            border,
            "Режим",
            Some("FakeIP — выдаёт виртуальные IP и резолвит по запросу; RedirHost — подмена Host."),
            |ui| {
                let before = self.dns_mode.clone();
                egui::ComboBox::from_id_salt("dns_mode")
                    .selected_text(egui::RichText::new(&before).size(11.0))
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for m in ["FakeIP", "RedirHost"] {
                            ui.selectable_value(
                                &mut self.dns_mode,
                                m.to_owned(),
                                egui::RichText::new(m).size(11.0),
                            );
                        }
                    });
                if self.dns_mode != before {
                    log::info!("dns_mode {}", self.dns_mode);
                }
            },
        );
        settings_row(
            ui,
            border,
            "Listen",
            Some("Адрес:порт DNS сервера, например 0.0.0.0:1053."),
            |ui| {
                ui.add_sized(
                    [120.0, 22.0],
                    egui::TextEdit::singleline(&mut self.dns_listen),
                );
            },
        );
        settings_row(
            ui,
            border,
            "DNS IPv6 (AAAA)",
            Some("Отвечать AAAA записями (разрешение IPv6)."),
            |ui| {
                if ui.checkbox(&mut self.dns_ipv6, "").changed() {
                    log::info!("dns_ipv6 {}", self.dns_ipv6);
                }
            },
        );
        settings_row(
            ui,
            border,
            "FakeIP Range",
            Some("Диапазон фейк-IP, например 198.18.0.1/16."),
            |ui| {
                ui.add_sized(
                    [120.0, 22.0],
                    egui::TextEdit::singleline(&mut self.dns_fakeip_range),
                );
            },
        );
        settings_row(
            ui,
            border,
            "Nameserver",
            Some("Основные резолверы (список)."),
            |ui| {
                if ui
                    .add_sized(
                        [64.0, 22.0],
                        egui::Button::new(
                            egui::RichText::new(format!("{} ✎", self.dns_nameservers_count))
                                .size(11.0),
                        ),
                    )
                    .clicked()
                {
                    log::info!("edit nameservers");
                }
            },
        );
        settings_row(
            ui,
            border,
            "Fallback",
            Some("Резервные резолверы, если основные не ответили."),
            |ui| {
                if ui
                    .add_sized(
                        [64.0, 22.0],
                        egui::Button::new(
                            egui::RichText::new(format!("{} ✎", self.dns_fallback_count))
                                .size(11.0),
                        ),
                    )
                    .clicked()
                {
                    log::info!("edit fallback");
                }
            },
        );
    }

    fn render_settings_network(&mut self, ui: &mut egui::Ui) {
        let border = border_color_for(self.app_config.theme);
        settings_row(
            ui,
            border,
            "Bypass Domain",
            Some("Домены мимо прокси (прямое соединение)."),
            |ui| {
                if ui
                    .add_sized(
                        [64.0, 22.0],
                        egui::Button::new(
                            egui::RichText::new(format!("{} ✎", self.net_bypass_count)).size(11.0),
                        ),
                    )
                    .clicked()
                {
                    log::info!("edit bypass domains");
                }
            },
        );
        settings_row(
            ui,
            border,
            "Append System DNS",
            Some("Добавлять системные DNS к списку резолверов."),
            |ui| {
                if ui.checkbox(&mut self.net_append_system_dns, "").changed() {
                    log::info!("append_system_dns {}", self.net_append_system_dns);
                }
            },
        );
        settings_row(
            ui,
            border,
            "Hosts",
            Some("Статические записи hosts (домен — IP)."),
            |ui| {
                if ui
                    .add_sized(
                        [64.0, 22.0],
                        egui::Button::new(
                            egui::RichText::new(format!("{} ✎", self.net_hosts_count)).size(11.0),
                        ),
                    )
                    .clicked()
                {
                    log::info!("edit hosts");
                }
            },
        );
    }
}

impl RClashApp {
    pub fn new(cc: &eframe::CreationContext<'_>, tray: Option<crate::tray::TrayHandle>) -> Self {
        let app_config = rclash_config::load_app_config();
        cc.egui_ctx.set_visuals(theme_visuals(app_config.theme));
        setup_fonts(&cc.egui_ctx);
        let profile_store = rclash_config::profile::load_profile_store();
        let active_profile = profile_store.active.clone();
        let groups = vec!["PROXY".to_owned(), "Germany".to_owned(), "Auto".to_owned()];
        let proxies = mock_proxies();
        let core_mixed_port = app_config
            .mixed_port
            .map(|p| p.to_string())
            .unwrap_or_else(|| "7890".to_owned());
        let core_socks_port = app_config
            .socks_port
            .map(|p| p.to_string())
            .unwrap_or_else(|| "0".to_owned());
        let core_external_controller = app_config
            .external_controller
            .clone()
            .unwrap_or_else(|| "127.0.0.1:9090".to_owned());
        let core_keep_alive = app_config
            .keep_alive_interval
            .map(|v| v.to_string())
            .unwrap_or_else(|| "30".to_owned());
        let core_geodata_loader = app_config
            .geodata_loader
            .clone()
            .unwrap_or_else(|| "Memory".to_owned());
        Self {
            app_config,
            tray,
            profile_store,
            active_profile,
            add_menu_open: false,
            groups,
            selected_group: "Все группы".to_owned(),
            proxies,
            selected_proxy: Some("Germany02-Hysteria2".to_owned()),
            selected_mode: "rule".to_owned(),
            proxy_enabled: true,
            tun_enabled: false,
            core_enabled: false,
            active_tab: Tab::Dashboard,
            config_content: mock_config_yaml(),
            config_interval: UpdateInterval::Auto,
            log_level: LogLevel::Info,
            log_source: LogSource::All,
            log_autoscroll: true,
            settings_tab: SettingsTab::Application,
            profiles_section: ProfilesSection::Profiles,
            input_open: false,
            input_ctx: InputContext::Url,
            input_text: String::new(),
            input_error: String::new(),
            sub_fetch: None,
            core_default_mode: "rule".to_owned(),
            core_mixed_port,
            core_socks_port,
            core_external_controller,
            core_keep_alive,
            core_geodata_loader,
            dns_enable: true,
            dns_mode: "FakeIP".to_owned(),
            dns_listen: "0.0.0.0:1053".to_owned(),
            dns_ipv6: false,
            dns_fakeip_range: "198.18.0.1/16".to_owned(),
            dns_nameservers_count: 0,
            dns_fallback_count: 0,
            net_bypass_count: 0,
            net_append_system_dns: false,
            net_hosts_count: 0,
        }
    }

    fn render_config_editor(&mut self, ui: &mut egui::Ui) {
        let theme = self.app_config.theme;
        let card_fill = card_fill_for(theme);
        let border = border_color_for(theme);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
            if ui
                .add_sized(
                    [64.0, 22.0],
                    egui::Button::new(egui::RichText::new("< Назад").size(11.0)),
                )
                .clicked()
            {
                self.active_tab = Tab::Dashboard;
            }
            ui.label(egui::RichText::new("Редактор конфига").size(11.0).strong());
            let interval = self.config_interval;
            egui::ComboBox::from_id_salt("config_interval")
                .selected_text(egui::RichText::new(interval.label_ru()).size(11.0))
                .width(110.0)
                .show_ui(ui, |ui| {
                    for iv in UpdateInterval::all() {
                        if ui
                            .selectable_label(
                                interval == *iv,
                                egui::RichText::new(iv.label_ru()).size(11.0),
                            )
                            .clicked()
                        {
                            self.config_interval = *iv;
                        }
                    }
                });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let save_btn = egui::Button::new(
                    egui::RichText::new("Сохранить")
                        .size(11.0)
                        .color(egui::Color32::WHITE),
                )
                .fill(NEKO_ACCENT)
                .stroke(egui::Stroke::new(1.0_f32, NEKO_ACCENT))
                .corner_radius(egui::CornerRadius::same(R6));
                if ui.add_sized([90.0, 22.0], save_btn).clicked() {
                    log::info!("config save");
                }
                if ui
                    .add_sized(
                        [80.0, 22.0],
                        egui::Button::new(egui::RichText::new("Сбросить").size(11.0)),
                    )
                    .clicked()
                {
                    self.config_content = mock_config_yaml();
                }
                if ui
                    .add_sized(
                        [80.0, 22.0],
                        egui::Button::new(egui::RichText::new("Открыть ↗").size(11.0)),
                    )
                    .clicked()
                {
                    log::info!("config open in editor");
                }
            });
        });
        ui.add_space(8.0);
        let editor_frame = egui::Frame::new()
            .fill(card_fill)
            .stroke(egui::Stroke::new(1.0_f32, border))
            .corner_radius(egui::CornerRadius::same(R6))
            .inner_margin(egui::Margin::same(8));
        editor_frame.show(ui, |ui| {
            let h = ui.available_height().max(120.0);
            ui.set_min_height(h);
            ui.set_max_height(h);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.config_content)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY),
                    );
                });
        });
    }

    fn render_logs_screen(&mut self, ui: &mut egui::Ui) {
        let theme = self.app_config.theme;
        let card_fill = card_fill_for(theme);
        let border = border_color_for(theme);
        let threshold = log_threshold(self.log_level);
        let source = self.log_source;
        let entries: Vec<crate::logger::AppLogEntry> = crate::logger::snapshot()
            .into_iter()
            .filter(|e| log_entry_visible(source, threshold, e))
            .collect();

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
            if ui
                .add_sized(
                    [64.0, 22.0],
                    egui::Button::new(egui::RichText::new("< Назад").size(11.0)),
                )
                .clicked()
            {
                self.active_tab = Tab::Dashboard;
            }
            ui.label(egui::RichText::new("Логи").size(11.0).strong());
            ui.add_space(8.0);
            for (src, label) in [
                (LogSource::Core, "Ядро"),
                (LogSource::App, "Приложение"),
                (LogSource::All, "Все"),
            ] {
                let active = self.log_source == src;
                let fill = if active { NEKO_ACCENT } else { card_fill };
                let txt_color = if active {
                    egui::Color32::WHITE
                } else {
                    match theme {
                        Theme::Light => egui::Color32::BLACK,
                        Theme::Dark => egui::Color32::WHITE,
                    }
                };
                let text = egui::RichText::new(label).size(11.0).color(txt_color);
                let btn = egui::Button::new(text)
                    .fill(fill)
                    .stroke(egui::Stroke::new(
                        1.0_f32,
                        if active { NEKO_ACCENT } else { border },
                    ))
                    .corner_radius(egui::CornerRadius::same(R6));
                if ui.add(btn).clicked() {
                    self.log_source = src;
                }
            }
            ui.add_space(8.0);
            if ui
                .add_sized(
                    [70.0, 22.0],
                    egui::Button::new(egui::RichText::new("Очистить").size(11.0)),
                )
                .clicked()
            {
                crate::logger::clear();
            }
            ui.checkbox(
                &mut self.log_autoscroll,
                egui::RichText::new("Авто").size(11.0),
            );
            if ui
                .add_sized(
                    [80.0, 22.0],
                    egui::Button::new(egui::RichText::new("Копировать").size(11.0)),
                )
                .clicked()
            {
                let text = entries
                    .iter()
                    .map(|e| format!("{}  {}  [{}]  {}", e.time, e.level, e.target, e.message))
                    .collect::<Vec<_>>()
                    .join("\n");
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    let _ = cb.set_text(text);
                }
                log::info!("logs copied {} entries", entries.len());
            }
            if ui
                .add_sized(
                    [70.0, 22.0],
                    egui::Button::new(egui::RichText::new("Экспорт").size(11.0)),
                )
                .clicked()
            {
                let text = entries
                    .iter()
                    .map(|e| format!("{}  {}  [{}]  {}", e.time, e.level, e.target, e.message))
                    .collect::<Vec<_>>()
                    .join("\n");
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Text", &["txt", "log"])
                    .set_file_name("rclash-logs.txt")
                    .save_file()
                {
                    if std::fs::write(&path, text).is_ok() {
                        log::info!("logs exported to {}", path.display());
                    } else {
                        log::warn!("logs export failed");
                    }
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::ComboBox::from_id_salt("log_level")
                    .selected_text(egui::RichText::new(self.log_level.as_str()).size(11.0))
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for lv in [
                            LogLevel::Debug,
                            LogLevel::Info,
                            LogLevel::Warning,
                            LogLevel::Error,
                            LogLevel::Silent,
                        ] {
                            ui.selectable_value(
                                &mut self.log_level,
                                lv,
                                egui::RichText::new(lv.as_str()).size(11.0),
                            );
                        }
                    });
            });
        });
        ui.add_space(8.0);
        let log_frame = egui::Frame::new()
            .fill(card_fill)
            .stroke(egui::Stroke::new(1.0_f32, border))
            .corner_radius(egui::CornerRadius::same(R4))
            .inner_margin(egui::Margin::same(8));
        log_frame.show(ui, |ui| {
            let h = ui.available_height().max(120.0);
            ui.set_min_height(h);
            ui.set_max_height(h);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(self.log_autoscroll)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 2.0);
                    if entries.is_empty() {
                        ui.add_space(24.0);
                        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("Нет записей").size(11.0).weak());
                        });
                    } else {
                        for e in &entries {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                                ui.add_sized(
                                    [78.0, 16.0],
                                    egui::Label::new(
                                        egui::RichText::new(&e.time).monospace().size(10.0).weak(),
                                    ),
                                );
                                ui.add_sized(
                                    [52.0, 16.0],
                                    egui::Label::new(
                                        egui::RichText::new(format!("{:<7}", e.level))
                                            .monospace()
                                            .size(10.0)
                                            .color(log_level_color(&e.level)),
                                    ),
                                );
                                ui.label(
                                    egui::RichText::new(format!("[{}]", e.target))
                                        .monospace()
                                        .size(10.0),
                                );
                                ui.add(
                                    egui::Label::new(egui::RichText::new(&e.message).size(11.0))
                                        .truncate(),
                                );
                            });
                        }
                    }
                });
        });
    }

    fn open_input(&mut self, ctx: InputContext, text: String) {
        self.input_ctx = ctx;
        self.input_text = text;
        self.input_error.clear();
        self.input_open = true;
        self.add_menu_open = false;
    }

    fn close_input(&mut self) {
        self.input_open = false;
        self.input_text.clear();
        self.input_error.clear();
    }

    fn input_title(&self) -> &'static str {
        match self.input_ctx {
            InputContext::Clipboard => "Вставь из буфера",
            InputContext::Url => "Введи URL подписки",
            InputContext::Raw => "Введи сырую ссылку",
        }
    }

    fn refresh_groups(&mut self) {
        let mut groups = vec!["PROXY".to_owned()];
        for p in &self.proxies {
            if p.group != "PROXY" && !groups.contains(&p.group) {
                groups.push(p.group.clone());
            }
        }
        self.groups = groups;
        if self.selected_group != "Все группы" && !self.groups.contains(&self.selected_group)
        {
            self.selected_group = "Все группы".to_owned();
        }
    }

    fn add_raw_proxies(&mut self, text: &str) -> usize {
        let parsed = rclash_subscription::parse_text_links(text).unwrap_or_default();
        let mut added = 0;
        for v in &parsed {
            if let Some(item) = proxy_item_from_value(v) {
                if !self.proxies.iter().any(|p| p.name == item.name) {
                    self.proxies.push(item);
                    added += 1;
                }
            }
        }
        if added > 0 {
            self.refresh_groups();
        }
        added
    }

    fn process_text(&mut self, text: &str, suggested_name: &str) {
        let t = text.trim();
        if t.is_empty() {
            self.input_error = "Вставь текст".to_owned();
            return;
        }
        if t.starts_with("http://") || t.starts_with("https://") {
            let url = t.to_owned();
            let name = profile_name_from_url(&url, suggested_name);
            self.input_error = "Загрузка…".to_owned();
            self.sub_fetch = Some(poll_promise::Promise::spawn_thread(
                "fetch-subscription",
                move || fetch_subscription(url, name),
            ));
            return;
        }
        if t.contains("://") {
            let added = self.add_raw_proxies(t);
            if added == 0 {
                self.input_error = "Не распознано".to_owned();
            } else {
                log::info!("raw keys added {added}");
                self.close_input();
            }
            return;
        }
        match rclash_config::profile::import_profile_content(t, suggested_name) {
            Ok(profile) => {
                self.profile_store.add_or_replace(profile.clone());
                self.active_profile = Some(profile.name.clone());
                self.profile_store.active = Some(profile.name.clone());
                let _ = rclash_config::profile::save_profile_store(&self.profile_store);
                log::info!("profile imported {}", profile.name);
                self.close_input();
            }
            Err(_) => {
                self.input_error = "Не распознано".to_owned();
            }
        }
    }

    fn submit_input(&mut self) {
        let text = self.input_text.clone();
        let fallback = match self.input_ctx {
            InputContext::Clipboard => "clipboard",
            InputContext::Url => "subscription",
            InputContext::Raw => "raw",
        };
        self.process_text(&text, fallback);
    }

    fn poll_sub_fetch(&mut self) {
        let ready = self.sub_fetch.as_ref().is_some_and(|p| p.ready().is_some());
        if !ready {
            return;
        }
        if let Some(p) = self.sub_fetch.take() {
            match p.block_and_take() {
                Ok((name, content)) => {
                    let content = content.clone();
                    if content.contains("proxies:")
                        || rclash_subscription::detect_format(&content)
                            == rclash_subscription::DetectedFormat::Yaml
                    {
                        match rclash_config::profile::import_profile_content(&content, &name) {
                            Ok(profile) => {
                                self.profile_store.add_or_replace(profile.clone());
                                self.active_profile = Some(profile.name.clone());
                                self.profile_store.active = Some(profile.name.clone());
                                let _ =
                                    rclash_config::profile::save_profile_store(&self.profile_store);
                                log::info!("subscription imported {}", profile.name);
                                self.close_input();
                            }
                            Err(e) => {
                                self.input_error = format!("Профиль: {e}");
                            }
                        }
                    } else {
                        let added = self.add_raw_proxies(&content);
                        if added == 0 {
                            self.input_error = "Не распознано".to_owned();
                        } else {
                            log::info!("subscription raw keys added {added}");
                            self.close_input();
                        }
                    }
                }
                Err(e) => {
                    self.input_error = e;
                }
            }
        }
    }

    fn render_input_dialog(&mut self, ctx: &egui::Context) {
        if !self.input_open {
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.close_input();
            return;
        }
        let theme = self.app_config.theme;
        let card_fill = card_fill_for(theme);
        let border = border_color_for(theme);
        let title = self.input_title();
        let loading = self.sub_fetch.is_some();
        let mut submit = false;
        let mut cancel = false;
        egui::Area::new("input_dialog_v2".into())
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(card_fill)
                    .stroke(egui::Stroke::new(1.0_f32, border))
                    .corner_radius(egui::CornerRadius::same(R6))
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(394.0, 0.0));
                        ui.set_max_width(394.0);
                        ui.label(egui::RichText::new(title).size(12.0).strong());
                        ui.add_space(8.0);
                        let hint = egui::RichText::new("https://… или hysteria2://…")
                            .size(11.0)
                            .weak();
                        ui.add_sized(
                            [394.0, 58.0],
                            egui::TextEdit::multiline(&mut self.input_text)
                                .hint_text(hint)
                                .font(egui::TextStyle::Monospace),
                        );
                        if loading {
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new("Загрузка…").size(11.0).weak());
                        }
                        if !self.input_error.is_empty() && !loading {
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(&self.input_error)
                                    .size(11.0)
                                    .color(NEKO_RED),
                            );
                        }
                        ui.add_space(8.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                            let ok_btn = egui::Button::new(
                                egui::RichText::new("OK")
                                    .size(11.0)
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(NEKO_ACCENT)
                            .stroke(egui::Stroke::new(1.0_f32, NEKO_ACCENT))
                            .corner_radius(egui::CornerRadius::same(R6));
                            if ui.add_enabled(!loading, ok_btn).clicked() {
                                submit = true;
                            }
                            if ui
                                .add_sized(
                                    [80.0, 22.0],
                                    egui::Button::new(egui::RichText::new("Отмена").size(11.0)),
                                )
                                .clicked()
                            {
                                cancel = true;
                            }
                        });
                    });
            });
        if cancel {
            self.close_input();
        } else if submit {
            self.submit_input();
        }
    }

    fn render_profiles_screen(&mut self, ui: &mut egui::Ui) {
        let theme = self.app_config.theme;
        let card_fill = card_fill_for(theme);
        let border = border_color_for(theme);
        let profiles = self.profile_store.profiles.clone();
        let raw_keys = self.proxies.clone();

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
            if ui
                .add_sized(
                    [64.0, 22.0],
                    egui::Button::new(egui::RichText::new("< Назад").size(11.0)),
                )
                .clicked()
            {
                self.active_tab = Tab::Dashboard;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
                for (section, label) in [
                    (ProfilesSection::RawKeys, "Сырые ключи"),
                    (ProfilesSection::Profiles, "Профили"),
                ] {
                    let active = self.profiles_section == section;
                    let fill = if active { NEKO_ACCENT } else { card_fill };
                    let txt_color = if active {
                        egui::Color32::WHITE
                    } else {
                        match theme {
                            Theme::Light => egui::Color32::BLACK,
                            Theme::Dark => egui::Color32::WHITE,
                        }
                    };
                    let btn =
                        egui::Button::new(egui::RichText::new(label).size(11.0).color(txt_color))
                            .fill(fill)
                            .stroke(egui::Stroke::new(
                                1.0_f32,
                                if active { NEKO_ACCENT } else { border },
                            ))
                            .corner_radius(egui::CornerRadius::same(R6));
                    if ui.add(btn).clicked() {
                        self.profiles_section = section;
                    }
                }
            });
        });
        ui.add_space(8.0);
        let section = self.profiles_section;
        let mut to_delete_profile: Option<String> = None;
        let mut to_delete_raw: Option<usize> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 8.0);
                if section == ProfilesSection::Profiles {
                    let profiles_frame = egui::Frame::new()
                        .fill(card_fill)
                        .stroke(egui::Stroke::new(1.0_f32, border))
                        .corner_radius(egui::CornerRadius::same(R4))
                        .inner_margin(egui::Margin::same(8));
                    profiles_frame.show(ui, |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 6.0);
                        ui.label(
                            egui::RichText::new(format!("Профили — {} шт.", profiles.len()))
                                .size(11.0)
                                .strong(),
                        );
                        if profiles.is_empty() {
                            ui.add_space(24.0);
                            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new("Профилей нет").size(11.0).weak());
                            });
                        } else {
                            for p in &profiles {
                                let row_frame = egui::Frame::new()
                                    .fill(card_fill)
                                    .stroke(egui::Stroke::new(1.0_f32, border))
                                    .corner_radius(egui::CornerRadius::same(R6))
                                    .inner_margin(egui::Margin::symmetric(8, 6));
                                row_frame.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                                        ui.vertical(|ui| {
                                            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&p.name).size(11.0),
                                                )
                                                .truncate(),
                                            );
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&p.path)
                                                        .monospace()
                                                        .size(10.0)
                                                        .weak(),
                                                )
                                                .truncate(),
                                            );
                                        });
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let del_btn = egui::Button::new(
                                                    egui::RichText::new("×")
                                                        .size(11.0)
                                                        .color(egui::Color32::WHITE),
                                                )
                                                .fill(NEKO_RED)
                                                .stroke(egui::Stroke::new(1.0_f32, NEKO_RED))
                                                .corner_radius(egui::CornerRadius::same(R4));
                                                if ui
                                                    .add_sized([22.0, 22.0], del_btn)
                                                    .on_hover_text("Удалить")
                                                    .clicked()
                                                {
                                                    to_delete_profile = Some(p.name.clone());
                                                }
                                            },
                                        );
                                    });
                                });
                            }
                        }
                    });
                }
                let raw_frame = egui::Frame::new()
                    .fill(card_fill)
                    .stroke(egui::Stroke::new(1.0_f32, border))
                    .corner_radius(egui::CornerRadius::same(R4))
                    .inner_margin(egui::Margin::same(8));
                if section == ProfilesSection::RawKeys {
                    raw_frame.show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(format!("Сырые ключи — {} шт.", raw_keys.len()))
                                .size(11.0)
                                .strong(),
                        );
                        ui.add_space(6.0);
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0);
                        if raw_keys.is_empty() {
                            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new("Пока пусто").size(11.0).weak());
                            });
                        } else {
                            for (i, p) in raw_keys.iter().enumerate() {
                                let row_frame = egui::Frame::new()
                                    .fill(card_fill)
                                    .stroke(egui::Stroke::new(1.0_f32, border))
                                    .corner_radius(egui::CornerRadius::same(R4))
                                    .inner_margin(egui::Margin::symmetric(6, 4));
                                row_frame.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                                        ui.label(
                                            egui::RichText::new(format!("{}", i + 1))
                                                .size(10.0)
                                                .weak(),
                                        );
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(format!(
                                                    "{}#{}",
                                                    p.name, p.proxy_type
                                                ))
                                                .monospace()
                                                .size(10.0),
                                            )
                                            .truncate(),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let del_btn = egui::Button::new(
                                                    egui::RichText::new("×")
                                                        .size(11.0)
                                                        .color(egui::Color32::WHITE),
                                                )
                                                .fill(NEKO_RED)
                                                .stroke(egui::Stroke::new(1.0_f32, NEKO_RED))
                                                .corner_radius(egui::CornerRadius::same(R4));
                                                if ui
                                                    .add_sized([22.0, 22.0], del_btn)
                                                    .on_hover_text("Удалить")
                                                    .clicked()
                                                {
                                                    to_delete_raw = Some(i);
                                                }
                                            },
                                        );
                                    });
                                });
                            }
                        }
                    });
                }
            });
        if let Some(name) = to_delete_profile {
            if self.profile_store.remove(&name) {
                if self.active_profile.as_deref() == Some(&name) {
                    self.active_profile = self.profile_store.active.clone();
                }
                let _ = rclash_config::profile::save_profile_store(&self.profile_store);
                log::info!("profile deleted {name}");
            }
        }
        if let Some(i) = to_delete_raw {
            if i < self.proxies.len() {
                let removed = self.proxies.remove(i);
                log::info!("raw key deleted {}", removed.name);
            }
        }
    }

    #[allow(dead_code)]
    pub fn set_theme(&mut self, theme: Theme, ctx: &egui::Context) {
        self.app_config.theme = theme;
        ctx.set_visuals(theme_visuals(theme));
        let mut style = (*ctx.style()).clone();
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        style.spacing.interact_size.y = 22.0;
        ctx.set_style(style);
        let _ = rclash_config::save_app_config(&self.app_config);
        log::info!("theme changed to {:?}", theme);
    }
}

impl eframe::App for RClashApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(handle) = &self.tray {
            crate::tray::poll_tray(handle, ctx);
        }
        if ctx.input(|i| i.viewport().close_requested())
            && self.app_config.minimize_to_tray
            && self.tray.is_some()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        if self.active_tab == Tab::Logs {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }

        let panel_fill = panel_fill_for(self.app_config.theme);
        let card_fill = card_fill_for(self.app_config.theme);
        let border = border_color_for(self.app_config.theme);

        if self.add_menu_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.add_menu_open = false;
        }
        self.poll_sub_fetch();

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(panel_fill)
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    let card_frame = egui::Frame::new()
                        .fill(card_fill)
                        .stroke(egui::Stroke::new(1.0_f32, border))
                        .corner_radius(egui::CornerRadius::same(R6))
                        .inner_margin(egui::Margin::same(8));

                    let mut add_btn_rect: Option<egui::Rect> = None;

                    let filtered: Vec<ProxyItem> = self
                        .proxies
                        .iter()
                        .filter(|p| {
                            if self.selected_group != "Все группы" && p.group != self.selected_group
                            {
                                return false;
                            }
                            true
                        })
                        .cloned()
                        .collect();
                    let count = filtered.len();
                    let proxy_selected = self.selected_proxy.clone();
                    let theme = self.app_config.theme;

                    if self.active_tab == Tab::Config {
                        self.render_config_editor(ui);
                    } else if self.active_tab == Tab::Logs {
                        self.render_logs_screen(ui);
                    } else if self.active_tab == Tab::Profiles {
                        self.render_profiles_screen(ui);
                    } else if self.active_tab == Tab::Settings {
                        self.render_settings_screen(ui);
                    } else {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;

                        ui.allocate_ui(egui::vec2(313.0, 519.0), |ui| {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(0.0, 8.0);
                                let tab_frame = egui::Frame::new()
                                    .fill(card_fill)
                                    .stroke(egui::Stroke::new(1.0_f32, border))
                                    .corner_radius(egui::CornerRadius::same(R4))
                                    .inner_margin(egui::Margin::symmetric(4, 4));
                                tab_frame.show(ui, |ui| {
                                    ui.set_min_size(egui::vec2(304.0, 24.0));
                                    ui.set_max_size(egui::vec2(304.0, 24.0));
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                                        for (tab, label) in
                                            [(Tab::Config, "конфиг"), (Tab::Logs, "логи"), (Tab::Settings, "настройки")]
                                        {
                                            let active = self.active_tab == tab;
                                            let mut rt =
                                                egui::RichText::new(label).size(11.0);
                                            if active {
                                                rt = rt.strong();
                                            }
                                            let btn = egui::Button::new(rt);
                                            if ui.add_sized(egui::vec2(100.0, 24.0), btn).clicked()
                                            {
                                                self.active_tab = tab;
                                            }
                                        }
                                    });
                                });
                                let chart_frame = egui::Frame::new()
                                    .fill(card_fill)
                                    .stroke(egui::Stroke::new(1.0_f32, border))
                                    .corner_radius(egui::CornerRadius::same(R4))
                                    .inner_margin(egui::Margin::same(8));
                                chart_frame.show(ui, |ui| {
                                    ui.set_min_size(egui::vec2(295.0, 143.0));
                                    ui.set_max_size(egui::vec2(295.0, 143.0));
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new("График").size(11.0));
                                        ui.add_space(4.0);
                                        let sep_w = ui.available_width();
                                        let (sep_rect, _) = ui.allocate_exact_size(
                                            egui::vec2(sep_w, 1.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().hline(
                                            sep_rect.x_range(),
                                            sep_rect.center().y,
                                            egui::Stroke::new(1.0_f32, border),
                                        );
                                        ui.add_space(32.0);
                                        ui.with_layout(
                                            egui::Layout::top_down(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    egui::RichText::new("Ожидание данных...")
                                                        .size(11.0)
                                                        .weak(),
                                                );
                                            },
                                        );
                                        ui.add_space(32.0);
                                        let (btm_rect, _) = ui.allocate_exact_size(
                                            egui::vec2(sep_w, 1.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().hline(
                                            btm_rect.x_range(),
                                            btm_rect.center().y,
                                            egui::Stroke::new(1.0_f32, border),
                                        );
                                        ui.add_space(1.0);
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing =
                                                egui::vec2(0.0, 0.0);
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing =
                                                    egui::vec2(4.0, 0.0);
                                                ui.label(
                                                    egui::RichText::new("●")
                                                        .size(10.0)
                                                        .color(NEKO_ACCENT),
                                                );
                                                ui.label(
                                                    egui::RichText::new("↑ 0 B/s")
                                                        .size(10.0)
                                                        .weak(),
                                                );
                                            });
                                            let after_left = ui.available_width();
                                            let right_w = 50.0;
                                            let center_w = 55.0;
                                            let spacer = ((after_left - right_w) / 2.0
                                                - center_w / 2.0)
                                                .max(0.0);
                                            ui.add_space(spacer);
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing =
                                                    egui::vec2(4.0, 0.0);
                                                ui.label(
                                                    egui::RichText::new("●")
                                                        .size(10.0)
                                                        .color(NEKO_DELAY_FAST),
                                                );
                                                ui.label(
                                                    egui::RichText::new("↓ 0 B/s")
                                                        .size(10.0)
                                                        .weak(),
                                                );
                                            });
                                            ui.with_layout(
                                                egui::Layout::right_to_left(
                                                    egui::Align::Center,
                                                ),
                                                |ui| {
                                                    ui.label(
                                                        egui::RichText::new("мах —")
                                                            .size(10.0)
                                                            .weak(),
                                                    );
                                                },
                                            );
                                        });
                                    });
                                });
                                let stats_frame = egui::Frame::new()
                                    .fill(card_fill)
                                    .stroke(egui::Stroke::new(1.0_f32, border))
                                    .corner_radius(egui::CornerRadius::same(R4))
                                    .inner_margin(egui::Margin::symmetric(8, 0));
                                stats_frame.show(ui, |ui| {
                                    ui.set_min_size(egui::vec2(295.0, 104.0));
                                    ui.set_max_size(egui::vec2(295.0, 104.0));
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                                        ui.allocate_ui(egui::vec2(295.0, 34.0), |ui| {
                                            ui.with_layout(
                                                egui::Layout::left_to_right(
                                                    egui::Align::Center,
                                                ),
                                                |ui| {
                                                    ui.label(
                                                        egui::RichText::new("приём")
                                                            .size(11.0),
                                                    );
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            ui.label(
                                                                egui::RichText::new("0 В")
                                                                    .size(11.0),
                                                            );
                                                        },
                                                    );
                                                },
                                            );
                                        });
                                        let sep_w = ui.available_width();
                                        let (sep_rect, _) = ui.allocate_exact_size(
                                            egui::vec2(sep_w, 1.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().hline(
                                            sep_rect.x_range(),
                                            sep_rect.center().y,
                                            egui::Stroke::new(1.0_f32, border),
                                        );
                                        ui.allocate_ui(egui::vec2(295.0, 34.0), |ui| {
                                            ui.with_layout(
                                                egui::Layout::left_to_right(
                                                    egui::Align::Center,
                                                ),
                                                |ui| {
                                                    ui.label(
                                                        egui::RichText::new("отдача")
                                                            .size(11.0),
                                                    );
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            ui.label(
                                                                egui::RichText::new("0 В")
                                                                    .size(11.0),
                                                            );
                                                        },
                                                    );
                                                },
                                            );
                                        });
                                        let (sep_rect2, _) = ui.allocate_exact_size(
                                            egui::vec2(sep_w, 1.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().hline(
                                            sep_rect2.x_range(),
                                            sep_rect2.center().y,
                                            egui::Stroke::new(1.0_f32, border),
                                        );
                                        ui.allocate_ui(egui::vec2(295.0, 34.0), |ui| {
                                            ui.with_layout(
                                                egui::Layout::left_to_right(
                                                    egui::Align::Center,
                                                ),
                                                |ui| {
                                                    ui.label(
                                                        egui::RichText::new("ip").size(11.0),
                                                    );
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            let ip_text = egui::RichText::new(
                                                                "195.133.54.31",
                                                            )
                                                            .size(11.0);
                                                            ui.label(ip_text);
                                                            ui.label(
                                                                egui::RichText::new("RU")
                                                                    .monospace()
                                                                    .size(10.0)
                                                                    .weak(),
                                                            );
                                                        },
                                                    );
                                                },
                                            );
                                        });
                                    });
                                });
                                let controls_frame = egui::Frame::new()
                                    .fill(card_fill)
                                    .stroke(egui::Stroke::new(1.0_f32, border))
                                    .corner_radius(egui::CornerRadius::same(R4))
                                    .inner_margin(egui::Margin {
                                        left: 7,
                                        right: 7,
                                        top: 6,
                                        bottom: 6,
                                    });
                                controls_frame.show(ui, |ui| {
                                    ui.set_min_size(egui::vec2(297.0, 98.0));
                                    ui.set_max_size(egui::vec2(297.0, 98.0));
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 8.0);
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing = egui::vec2(4.5, 0.0);
                                            for label in ["rule", "global", "direct"] {
                                                let active = self.selected_mode == label;
                                                let fill = if active { NEKO_ACCENT } else { card_fill };
                                                let txt_color = if active {
                                                    egui::Color32::WHITE
                                                } else {
                                                    match theme {
                                                        Theme::Light => egui::Color32::BLACK,
                                                        Theme::Dark => egui::Color32::WHITE,
                                                    }
                                                };
                                                let text = egui::RichText::new(label)
                                                    .size(11.0)
                                                    .color(txt_color);
                                                let btn = egui::Button::new(text)
                                                    .fill(fill)
                                                    .stroke(egui::Stroke::new(
                                                        1.0_f32,
                                                        if active { NEKO_ACCENT } else { border },
                                                    ))
                                                    .corner_radius(egui::CornerRadius::same(R6));
                                                if ui
                                                    .add_sized(
                                                        egui::vec2(96.0, 26.0),
                                                        btn,
                                                    )
                                                    .clicked()
                                                {
                                                    self.selected_mode = label.to_owned();
                                                }
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing = egui::vec2(3.0, 0.0);
                                            let proxy_fill = if self.proxy_enabled {
                                                NEKO_ACCENT
                                            } else {
                                                card_fill
                                            };
                                            let proxy_stroke = if self.proxy_enabled {
                                                NEKO_ACCENT
                                            } else {
                                                border
                                            };
                                            let proxy_txt = if self.proxy_enabled {
                                                egui::Color32::WHITE
                                            } else {
                                                match theme {
                                                    Theme::Light => egui::Color32::BLACK,
                                                    Theme::Dark => egui::Color32::WHITE,
                                                }
                                            };
                                            let proxy_label = "proxy";
                                            let proxy_btn = egui::Button::new(
                                                egui::RichText::new(proxy_label)
                                                    .size(11.0)
                                                    .color(proxy_txt),
                                            )
                                            .fill(proxy_fill)
                                            .stroke(egui::Stroke::new(1.0_f32, proxy_stroke))
                                            .corner_radius(egui::CornerRadius::same(R6));
                                            if ui
                                                .add_sized(egui::vec2(147.0, 26.0), proxy_btn)
                                                .clicked()
                                            {
                                                self.proxy_enabled = !self.proxy_enabled;
                                            }
                                            let tun_fill = if self.tun_enabled {
                                                NEKO_ACCENT
                                            } else {
                                                card_fill
                                            };
                                            let tun_stroke = if self.tun_enabled {
                                                NEKO_ACCENT
                                            } else {
                                                border
                                            };
                                            let tun_txt = if self.tun_enabled {
                                                egui::Color32::WHITE
                                            } else {
                                                match theme {
                                                    Theme::Light => egui::Color32::BLACK,
                                                    Theme::Dark => egui::Color32::WHITE,
                                                }
                                            };
                                            let tun_label = "tun";
                                            let tun_btn = egui::Button::new(
                                                egui::RichText::new(tun_label)
                                                    .size(11.0)
                                                    .color(tun_txt),
                                            )
                                            .fill(tun_fill)
                                            .stroke(egui::Stroke::new(1.0_f32, tun_stroke))
                                            .corner_radius(egui::CornerRadius::same(R6));
                                            if ui
                                                .add_sized(egui::vec2(147.0, 26.0), tun_btn)
                                                .clicked()
                                            {
                                                self.tun_enabled = !self.tun_enabled;
                                            }
                                        });
                                        let (tgl_label, tgl_fill) = if self.core_enabled {
                                            ("ВКЛ • Выключить", NEKO_GREEN)
                                        } else {
                                            ("ВЫКЛ • Включить", NEKO_RED)
                                        };
                                        let tgl_btn = egui::Button::new(
                                            egui::RichText::new(tgl_label)
                                                .size(11.0)
                                                .color(egui::Color32::WHITE),
                                        )
                                        .fill(tgl_fill)
                                        .stroke(egui::Stroke::new(1.0_f32, tgl_fill))
                                        .corner_radius(egui::CornerRadius::same(R6));
                                        if ui.add_sized(egui::vec2(297.0, 30.0), tgl_btn).clicked() {
                                            self.core_enabled = !self.core_enabled;
                                        }
                                    });
                                });
                                let update_frame = egui::Frame::new()
                                    .fill(egui::Color32::from_rgba_unmultiplied(
                                        0xE7, 0x4C, 0x3C, 40,
                                    ))
                                    .stroke(egui::Stroke::new(1.0_f32, NEKO_RED))
                                    .corner_radius(egui::CornerRadius::same(R4))
                                    .inner_margin(egui::Margin::same(8));
                                update_frame.show(ui, |ui| {
                                    ui.set_min_size(egui::vec2(295.0, 65.0));
                                    ui.set_max_size(egui::vec2(295.0, 65.0));
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                                        ui.add_space(11.5);
                                        ui.with_layout(
                                            egui::Layout::top_down(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    egui::RichText::new(
                                                        "Доступно обновление v0.2.8",
                                                    )
                                                    .size(11.0),
                                                );
                                                ui.add_space(8.0);
                                                ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing =
                                                egui::vec2(8.0, 0.0);
                                            ui.add_space(47.5);
                                            let upd_btn = egui::Button::new(
                                                egui::RichText::new("Обновить")
                                                    .size(11.0)
                                                    .color(egui::Color32::WHITE),
                                            )
                                            .fill(NEKO_ACCENT)
                                            .stroke(egui::Stroke::new(
                                                1.0_f32, NEKO_ACCENT,
                                            ))
                                            .corner_radius(egui::CornerRadius::same(R6));
                                            if ui
                                                .add_sized(
                                                    egui::vec2(96.0, 22.0),
                                                    upd_btn,
                                                )
                                                .clicked()
                                            {
                                                log::info!("update clicked");
                                            }
                                            let later_btn = egui::Button::new(
                                                egui::RichText::new("Позже").size(11.0),
                                            )
                                            .fill(match theme {
                                                Theme::Light => egui::Color32::from_rgb(
                                                    0xE8, 0xEA, 0xED,
                                                ),
                                                Theme::Dark => egui::Color32::from_rgb(
                                                    0x3A, 0x3D, 0x45,
                                                ),
                                            })
                                            .stroke(egui::Stroke::new(
                                                1.0_f32, border,
                                            ))
                                            .corner_radius(egui::CornerRadius::same(R6));
                                            if ui
                                                .add_sized(
                                                    egui::vec2(96.0, 22.0),
                                                    later_btn,
                                                )
                                                .clicked()
                                            {
                                                log::info!("later clicked");
                                            }
                                        });
                                    });
                                });
                            });
                            });
                        });

                        ui.add_space(6.0);

                        ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(0.0, 8.0);
                                card_frame.show(ui, |ui| {
                                    ui.set_width(469.0);
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing =
                                                egui::vec2(8.0, 0.0);

                                            let avail = ui.available_width();
                                            let combo_w =
                                                (avail - 26.0 - 26.0 - 16.0).max(120.0);

                                        let profiles = self.profile_store.profiles.clone();
                                        let active_label = self
                                            .active_profile
                                            .clone()
                                            .unwrap_or_else(|| "Выберите профиль".to_owned());

                                        egui::ComboBox::from_id_salt("profiles_combo")
                                            .selected_text(
                                                egui::RichText::new(active_label).size(11.0),
                                            )
                                            .width(combo_w)
                                            .show_ui(ui, |ui| {
                                                if profiles.is_empty() {
                                                    ui.label(
                                                        egui::RichText::new("Нет профилей")
                                                            .size(11.0)
                                                            .weak(),
                                                    );
                                                } else {
                                                    for p in &profiles {
                                                        let is_active = Some(&p.name)
                                                            == self.active_profile.as_ref();
                                                        let label = if is_active {
                                                            format!("{} • активен", p.name)
                                                        } else if p.is_raw {
                                                            format!("{} • сырые", p.name)
                                                        } else {
                                                            p.name.clone()
                                                        };
                                                        if ui
                                                            .selectable_label(
                                                                is_active,
                                                                egui::RichText::new(label)
                                                                    .size(11.0),
                                                            )
                                                            .clicked()
                                                        {
                                                            self.active_profile =
                                                                Some(p.name.clone());
                                                            self.profile_store.active =
                                                                Some(p.name.clone());
                                                            let _ =
                                                                rclash_config::profile::save_profile_store(
                                                                    &self.profile_store,
                                                                );
                                                            log::info!(
                                                                "profile selected {}",
                                                                p.name
                                                            );
                                                        }
                                                    }
                                                }
                                                ui.separator();
                                                if ui
                                                    .selectable_label(
                                                        false,
                                                        egui::RichText::new(
                                                            "Управление профилями…",
                                                        )
                                                        .size(11.0),
                                                    )
                                                    .clicked()
                                                {
                                                    self.active_tab = Tab::Profiles;
                                                }
                                            });

                                        if ui
                                            .add_sized(
                                                [26.0, 22.0],
                                                egui::Button::new(
                                                    egui::RichText::new("↻").size(11.0),
                                                ),
                                            )
                                            .on_hover_text("Обновить")
                                            .clicked()
                                        {
                                            log::info!(
                                                "refresh profile {:?}",
                                                self.active_profile
                                            );
                                        }

                                        let add_resp = ui
                                            .add_sized(
                                                [26.0, 22.0],
                                                egui::Button::new(
                                                    egui::RichText::new("+").size(11.0),
                                                ),
                                            )
                                            .on_hover_text("Добавить профиль");
                                        if add_resp.clicked() {
                                            self.add_menu_open = !self.add_menu_open;
                                        }
                                        add_btn_rect = Some(add_resp.rect);
                                    });

                                    ui.add_space(6.0);
                                    let sep_w = ui.available_width();
                                    let (sep_rect, _) = ui.allocate_exact_size(
                                        egui::vec2(sep_w, 1.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().hline(
                                        sep_rect.x_range(),
                                        sep_rect.center().y,
                                        egui::Stroke::new(1.0_f32, border),
                                    );
                                    ui.add_space(6.0);

                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                                        let avail = ui.available_width();
                                        let combo_w = (avail - 26.0 - 8.0).max(120.0);
                                        let groups = self.groups.clone();
                                        egui::ComboBox::from_id_salt("groups_combo")
                                            .selected_text(
                                                egui::RichText::new(self.selected_group.clone())
                                                    .size(11.0),
                                            )
                                            .width(combo_w)
                                            .show_ui(ui, |ui| {
                                                if ui
                                                    .selectable_label(
                                                        self.selected_group == "Все группы",
                                                        egui::RichText::new("Все группы")
                                                            .size(11.0),
                                                    )
                                                    .clicked()
                                                {
                                                    self.selected_group = "Все группы".to_owned();
                                                }
                                                for g in &groups {
                                                    if ui
                                                        .selectable_label(
                                                            self.selected_group == *g,
                                                            egui::RichText::new(g).size(11.0),
                                                        )
                                                        .clicked()
                                                    {
                                                        self.selected_group = g.clone();
                                                    }
                                                }
                                            });

                                        if ui
                                            .add_sized(
                                                [26.0, 22.0],
                                                egui::Button::new(
                                                    egui::RichText::new("◎").size(11.0),
                                                ),
                                            )
                                            .on_hover_text("Пинг видимых")
                                            .clicked()
                                        {
                                            log::info!(
                                                "ping visible group {}",
                                                self.selected_group
                                            );
                                        }
                                    });
                                });
                            });

                            let avail_h = ui.available_height().max(120.0);
                            let inner_h = (avail_h - 28.0 - 2.0).max(80.0);
                            card_frame.show(ui, |ui| {
                                ui.set_width(469.0);
                                ui.set_min_height(inner_h);
                                ui.set_max_height(inner_h);
                                ui.vertical(|ui| {
                                    ui.add_space(6.0);
                                    let scroll_h = (inner_h - 33.0).max(80.0);
                                    let mut to_select: Option<String> = None;
                                    egui::ScrollArea::vertical()
                                        .auto_shrink([false, false])
                                        .max_height(scroll_h)
                                        .show(ui, |ui| {
                                            ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0);
                                            if filtered.is_empty() {
                                                ui.add_space(24.0);
                                                ui.with_layout(
                                                    egui::Layout::top_down(egui::Align::Center),
                                                    |ui| {
                                                        ui.label(
                                                            egui::RichText::new("Нет локаций")
                                                                .size(11.0)
                                                                .weak(),
                                                        );
                                                    },
                                                );
                                            } else {
                                                for p in &filtered {
                                                    let is_sel =
                                                        Some(&p.name) == proxy_selected.as_ref();
                                                    let avail_w = ui.available_width();
                                                    let (rect, resp) = ui.allocate_exact_size(
                                                        egui::vec2(avail_w, 27.0),
                                                        egui::Sense::click(),
                                                    );
                                                    let hovered = resp.hovered();
                                                    let accent_bg =
                                                        egui::Color32::from_rgba_unmultiplied(
                                                            0x3B, 0x82, 0xF6, 36,
                                                        );
                                                    let hover_bg = match theme {
                                                        Theme::Light => {
                                                            egui::Color32::from_rgb(0xE8, 0xEA, 0xED)
                                                        }
                                                        Theme::Dark => {
                                                            egui::Color32::from_rgb(0x3A, 0x3D, 0x45)
                                                        }
                                                    };
                                                    let bg = if is_sel {
                                                        accent_bg
                                                    } else if hovered {
                                                        hover_bg
                                                    } else {
                                                        egui::Color32::TRANSPARENT
                                                    };
                                                    let stroke = if is_sel {
                                                        egui::Stroke::new(1.0_f32, NEKO_ACCENT)
                                                    } else {
                                                        egui::Stroke::NONE
                                                    };
                                                    ui.painter().rect(
                                                        rect,
                                                        egui::CornerRadius::same(R4),
                                                        bg,
                                                        stroke,
                                                        egui::StrokeKind::Inside,
                                                    );
                                                    if resp.clicked() {
                                                        to_select = Some(p.name.clone());
                                                    }
                                                    let inner =
                                                        rect.shrink2(egui::vec2(4.0_f32, 0.0_f32));
                                                    let name_color = if is_sel {
                                                        NEKO_ACCENT
                                                    } else {
                                                        match theme {
                                                            Theme::Light => egui::Color32::BLACK,
                                                            Theme::Dark => egui::Color32::WHITE,
                                                        }
                                                    };
                                                    let type_color = match theme {
                                                        Theme::Light => egui::Color32::BLACK,
                                                        Theme::Dark => egui::Color32::WHITE,
                                                    };
                                                    let delay_col = delay_color_for(p.delay, theme);
                                                    let delay_str = delay_text(p.delay);
                                                    ui.scope_builder(
                                                        egui::UiBuilder::new().max_rect(inner),
                                                        |ui| {
                                                            ui.columns(2, |cols| {
                                                                cols[0].horizontal(|ui| {
                                                                    ui.spacing_mut().item_spacing =
                                                                        egui::vec2(6.0_f32, 0.0_f32);
                                                                    let weak = match theme {
                                                                        Theme::Light => {
                                                                            egui::Color32::from_rgb(
                                                                                0x6B, 0x72, 0x80,
                                                                            )
                                                                        }
                                                                        Theme::Dark => {
                                                                            egui::Color32::from_rgb(
                                                                                0x9A, 0xA0, 0xA6,
                                                                            )
                                                                        }
                                                                    };
                                                                    ui.label(
                                                                        egui::RichText::new(
                                                                            country_code(&p.name),
                                                                        )
                                                                        .monospace()
                                                                        .size(10.0_f32)
                                                                        .color(weak),
                                                                    );
                                                                    let mut rt =
                                                                        egui::RichText::new(&p.name)
                                                                            .size(11.0_f32)
                                                                            .color(name_color);
                                                                    if is_sel {
                                                                        rt = rt.strong();
                                                                    }
                                                                    ui.add(
                                                                        egui::Label::new(rt).truncate(),
                                                                    );
                                                                });
                                                                cols[1].with_layout(
                                                                egui::Layout::right_to_left(
                                                                    egui::Align::Center,
                                                                ),
                                                                |ui| {
                                                                    ui.spacing_mut().item_spacing =
                                                                        egui::vec2(6.0_f32, 0.0_f32);
                                                                    let (rrect, _) = ui
                                                                        .allocate_exact_size(
                                                                            egui::vec2(
                                                                                14.0_f32, 14.0_f32,
                                                                            ),
                                                                            egui::Sense::hover(),
                                                                        );
                                                                    let ring = if is_sel {
                                                                        NEKO_ACCENT
                                                                    } else {
                                                                        egui::Color32::from_rgb(
                                                                            0x6B, 0x6F, 0x76,
                                                                        )
                                                                    };
                                                                    ui.painter_at(rrect).circle_stroke(
                                                                        rrect.center(),
                                                                        5.5_f32,
                                                                        egui::Stroke::new(
                                                                            1.0_f32, ring,
                                                                        ),
                                                                    );
                                                                    if is_sel {
                                                                        ui.painter_at(rrect)
                                                                            .circle_filled(
                                                                                rrect.center(),
                                                                                3.0_f32,
                                                                                NEKO_ACCENT,
                                                                            );
                                                                    }
                                                                    let delay_rt =
                                                                        egui::RichText::new(delay_str)
                                                                            .monospace()
                                                                            .size(11.0_f32)
                                                                            .color(delay_col);
                                                                    ui.add_sized(
                                                                        egui::vec2(50.0_f32, 14.0_f32),
                                                                        egui::Label::new(delay_rt),
                                                                    );
                                                                    let (sep_rect, _) = ui
                                                                        .allocate_exact_size(
                                                                            egui::vec2(
                                                                                8.0_f32, 14.0_f32,
                                                                            ),
                                                                            egui::Sense::hover(),
                                                                        );
                                                                    let sep_color = match theme {
                                                                        Theme::Light => {
                                                                            egui::Color32::from_rgb(
                                                                                0xD0, 0xD0, 0xD0,
                                                                            )
                                                                        }
                                                                        Theme::Dark => {
                                                                            egui::Color32::from_rgb(
                                                                                0x3C, 0x3F, 0x47,
                                                                            )
                                                                        }
                                                                    };
                                                                    ui.painter().vline(
                                                                        sep_rect.center().x,
                                                                        sep_rect.y_range(),
                                                                        egui::Stroke::new(
                                                                            1.0_f32, sep_color,
                                                                        ),
                                                                    );
                                                                    ui.label(
                                                                        egui::RichText::new(
                                                                            &p.proxy_type,
                                                                        )
                                                                        .monospace()
                                                                        .size(10.0_f32)
                                                                        .color(type_color),
                                                                    );
                                                                },
                                                            );
                                                            });
                                                        },
                                                    );
                                                }
                                            }
                                        });
                                    if let Some(n) = to_select {
                                        self.selected_proxy = Some(n.clone());
                                        log::info!("proxy selected {}", n);
                                    }
                                    ui.add_space(6.0);
                                    let sep_w = ui.available_width();
                                    let (sep_rect, _) = ui.allocate_exact_size(
                                        egui::vec2(sep_w, 1.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().hline(
                                        sep_rect.x_range(),
                                        sep_rect.center().y,
                                        egui::Stroke::new(1.0_f32, border),
                                    );
                                    ui.add_space(4.0);
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(format!("{count} локаций"))
                                                    .size(10.0)
                                                    .weak(),
                                            );
                                        },
                                    );
                                });
                            });
                        });
                    });
                    }

                    if self.add_menu_open {
                        if let Some(btn_rect) = add_btn_rect {
                            let pos = btn_rect.right_bottom() + egui::vec2(-160.0, 6.0);
                            egui::Area::new(egui::Id::new("add_menu"))
                                .fixed_pos(pos)
                                .order(egui::Order::Foreground)
                                .show(ctx, |ui| {
                                    egui::Frame::new()
                                        .fill(card_fill)
                                        .stroke(egui::Stroke::new(1.0_f32, border))
                                        .corner_radius(egui::CornerRadius::same(R6))
                                        .inner_margin(egui::Margin::same(6))
                                        .shadow(egui::Shadow {
                                            offset: [0, 10],
                                            blur: 30,
                                            spread: 0,
                                            color: egui::Color32::from_black_alpha(80),
                                        })
                                        .show(ui, |ui| {
                                            ui.set_min_width(160.0);
                                            ui.vertical(|ui| {
                                                ui.spacing_mut().item_spacing =
                                                    egui::vec2(0.0, 4.0);
                                                let items: [(&str, &str); 4] = [
                                                    ("Из буфера", "clipboard"),
                                                    ("URL", "url"),
                                                    ("Файл", "file"),
                                                    ("⌨ Сырой ключ", "raw"),
                                                ];
                                                let panel_bg =
                                                    panel_fill_for(self.app_config.theme);
                                                let hover_bg = match self.app_config.theme {
                                                    Theme::Light => {
                                                        egui::Color32::from_rgb(0xE8, 0xEA, 0xED)
                                                    }
                                                    Theme::Dark => {
                                                        egui::Color32::from_rgb(0x3A, 0x3D, 0x45)
                                                    }
                                                };
                                                ui.style_mut().visuals.widgets.inactive.bg_fill =
                                                    panel_bg;
                                                ui.style_mut().visuals.widgets.hovered.bg_fill =
                                                    hover_bg;
                                                ui.style_mut().visuals.widgets.active.bg_fill =
                                                    hover_bg;
                                                for (label, kind) in items {
                                                    if ui
                                                        .add_sized(
                                                            [160.0, 28.0],
                                                            egui::Button::new(
                                                                egui::RichText::new(label)
                                                                    .size(11.0),
                                                            ),
                                                        )
                                                        .clicked()
                                                    {
                                                        match kind {
                                                            "clipboard" => {
                                                                let text = arboard::Clipboard::new()
                                                                    .and_then(|mut cb| cb.get_text())
                                                                    .unwrap_or_default();
                                                                self.open_input(
                                                                    InputContext::Clipboard,
                                                                    text,
                                                                );
                                                            }
                                                            "url" => self.open_input(
                                                                InputContext::Url,
                                                                String::new(),
                                                            ),
                                                            "raw" => self.open_input(
                                                                InputContext::Raw,
                                                                String::new(),
                                                            ),
                                                            _ => {
                                                                self.add_menu_open = false;
                                                                if let Some(path) =
                                                                    rfd::FileDialog::new()
                                                                        .add_filter(
                                                                            "Config",
                                                                            &["yaml", "yml", "txt"],
                                                                        )
                                                                        .pick_file()
                                                                {
                                                                    match std::fs::read_to_string(
                                                                        &path,
                                                                    ) {
                                                                        Ok(content) => {
                                                                            let stem = path
                                                                                .file_stem()
                                                                                .and_then(|s| {
                                                                                    s.to_str()
                                                                                })
                                                                                .unwrap_or(
                                                                                    "import",
                                                                                )
                                                                                .to_owned();
                                                                            self.process_text(
                                                                                &content, &stem,
                                                                            );
                                                                            if !self.input_error.is_empty()
                                                                            {
                                                                                self.input_open =
                                                                                    true;
                                                                                self.input_ctx =
                                                                                    InputContext::Raw;
                                                                                self.input_text =
                                                                                    content;
                                                                            }
                                                                        }
                                                                        Err(e) => {
                                                                            log::warn!(
                                                                                "import file failed: {e}"
                                                                            );
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            });
                                        });
                                });
                            if ctx.input(|i| i.pointer.any_click()) {
                                if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                                    let menu_rect = egui::Rect::from_min_max(
                                        pos - egui::vec2(200.0, 200.0),
                                        pos + egui::vec2(200.0, 200.0),
                                    );
                                    let _ = menu_rect;
                                    if !btn_rect.contains(pos) {
                                        let menu_p =
                                            btn_rect.right_bottom() + egui::vec2(-160.0, 6.0);
                                        let menu_r = egui::Rect::from_min_size(
                                            menu_p,
                                            egui::vec2(172.0, 130.0),
                                        );
                                        if !menu_r.contains(pos) {
                                            self.add_menu_open = false;
                                        }
                                    }
                                }
                            }
                        }
                    }

                });
            });
        self.render_input_dialog(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logger::AppLogEntry;

    fn entry(level: &str, target: &str, message: &str) -> AppLogEntry {
        AppLogEntry {
            time: "18:26:50.771".to_owned(),
            level: level.to_owned(),
            target: target.to_owned(),
            message: message.to_owned(),
        }
    }

    #[test]
    fn log_visible_source_level() {
        let e = entry("info", "rclash_core_manager", "proxy selected");
        assert!(log_entry_visible(LogSource::All, 1, &e));
        assert!(log_entry_visible(LogSource::Core, 1, &e));
        assert!(!log_entry_visible(LogSource::App, 1, &e));
        assert!(!log_entry_visible(LogSource::All, 2, &e));
    }

    #[test]
    fn log_silent_hides_all() {
        let e = entry("error", "rclash", "boom");
        assert!(!log_entry_visible(LogSource::All, 4, &e));
        assert!(log_entry_visible(LogSource::All, 3, &e));
    }
}
