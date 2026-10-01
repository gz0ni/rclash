pub mod clipboard;
pub mod event;
pub mod state;

pub fn run() -> anyhow::Result<()> {
    let (tx, rx) = event::bus();
    let mut app = state::AppState::default();
    tx.send(event::Event::Tick).unwrap();
    if let Ok(event::Event::Tick) = rx.try_recv() {
        app.push_traffic(0, 0);
        app.set_toast("RClash TUI scaffold — event loop lands in step 2");
        let _ = app.visible_toast();
    }
    if let Some(prefill) = clipboard::read_clipboard_text() {
        app.ip_text = prefill;
    }
    println!("RClash TUI scaffold — event loop lands in step 2");
    Ok(())
}
