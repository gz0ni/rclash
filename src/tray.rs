use tray_icon::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

pub struct TrayHandle {
    _tray: TrayIcon,
    pub show_id: String,
    pub quit_id: String,
}

fn build_icon() -> Option<Icon> {
    let bytes = include_bytes!("../assets/icons/tray/tray-32.png");
    let (rgba, w, h) = crate::icon::decode_png_rgba(bytes).ok()?;
    Icon::from_rgba(rgba, w, h).ok()
}

pub fn init_tray() -> Option<TrayHandle> {
    let icon = build_icon()?;
    let show = MenuItem::new("Показать RClash", true, None);
    let quit = MenuItem::new("Выход", true, None);
    let sep = PredefinedMenuItem::separator();
    let menu = Menu::with_items(&[&show, &sep, &quit]).ok()?;
    let show_id = show.id().0.clone();
    let quit_id = quit.id().0.clone();
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("RClash")
        .with_icon(icon)
        .build()
        .ok()?;
    Some(TrayHandle {
        _tray: tray,
        show_id,
        quit_id,
    })
}

pub fn poll_tray(handle: &TrayHandle, ctx: &egui::Context) {
    use tray_icon::menu::MenuEvent;
    if let Ok(event) = MenuEvent::receiver().try_recv() {
        if event.id.0 == handle.quit_id {
            std::process::exit(0);
        }
        if event.id.0 == handle.show_id {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
    }
    use tray_icon::TrayIconEvent;
    if let Ok(event) = TrayIconEvent::receiver().try_recv() {
        if matches!(event.click_type, tray_icon::ClickType::Double) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
    }
}
