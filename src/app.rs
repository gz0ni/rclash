use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Profiles,
    Proxies,
    Connections,
    Logs,
    Settings,
}

impl Tab {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Dashboard => "Дашборд",
            Self::Profiles => "Профили",
            Self::Proxies => "Прокси",
            Self::Connections => "Соединения",
            Self::Logs => "Логи",
            Self::Settings => "Настройки",
        }
    }

    pub fn all() -> &'static [Tab] {
        &[
            Self::Dashboard,
            Self::Profiles,
            Self::Proxies,
            Self::Connections,
            Self::Logs,
            Self::Settings,
        ]
    }
}

pub struct RClashApp {
    current_tab: Tab,
}

impl RClashApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            current_tab: Tab::Dashboard,
        }
    }
}

impl eframe::App for RClashApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("side_panel")
            .resizable(false)
            .default_width(180.0)
            .show(ctx, |ui| {
                ui.heading("RClash");
                ui.separator();
                for tab in Tab::all() {
                    let selected = *tab == self.current_tab;
                    if ui.selectable_label(selected, tab.label()).clicked() {
                        self.current_tab = *tab;
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(egui::RichText::new("v0.1.0-rclash").weak().small());
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| match self.current_tab {
            Tab::Dashboard => ui::dashboard::show(ui),
            Tab::Profiles => ui::profiles::show(ui),
            Tab::Proxies => ui::proxies::show(ui),
            Tab::Connections => ui::connections::show(ui),
            Tab::Logs => ui::logs::show(ui),
            Tab::Settings => ui::settings::show(ui),
        });
    }
}
