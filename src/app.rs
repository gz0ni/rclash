use rclash_config::profile::ProfileStore;
use rclash_config::{AppConfig, Theme};

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

impl RClashApp {
    pub fn new(cc: &eframe::CreationContext<'_>, tray: Option<crate::tray::TrayHandle>) -> Self {
        let app_config = rclash_config::load_app_config();
        cc.egui_ctx.set_visuals(theme_visuals(app_config.theme));
        setup_fonts(&cc.egui_ctx);
        let profile_store = rclash_config::profile::load_profile_store();
        let active_profile = profile_store.active.clone();
        let groups = vec!["PROXY".to_owned(), "Germany".to_owned(), "Auto".to_owned()];
        let proxies = mock_proxies();
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

        let panel_fill = panel_fill_for(self.app_config.theme);
        let card_fill = card_fill_for(self.app_config.theme);
        let border = border_color_for(self.app_config.theme);

        if self.add_menu_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.add_menu_open = false;
        }

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
                                        for label in ["конфиг", "логи", "настройки"] {
                                            let resp = ui.add_sized(
                                                egui::vec2(100.0, 24.0),
                                                egui::Button::new(
                                                    egui::RichText::new(label).size(11.0),
                                                ),
                                            );
                                            if resp.clicked() {
                                                log::info!("tab {label}");
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
                                    ui.set_min_size(egui::vec2(295.0, 66.0));
                                    ui.set_max_size(egui::vec2(295.0, 66.0));
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                                        ui.add_space(12.5);
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
                                                    log::info!("open profile manager");
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
                                                        egui::vec2(avail_w, 28.0),
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
                                                                                12.0_f32, 12.0_f32,
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
                                                        log::info!("add {}", kind);
                                                        self.add_menu_open = false;
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
    }
}
