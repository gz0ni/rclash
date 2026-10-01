use std::io::Stdout;
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Context;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::tty::IsTty;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

static PANIC_HOOK_SET: AtomicBool = AtomicBool::new(false);

pub fn restore() {
    let _ = disable_raw_mode();
    let _ = execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
}

fn install_panic_hook() {
    if PANIC_HOOK_SET.swap(true, Ordering::SeqCst) {
        return;
    }
    let default = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore();
        default(info);
    }));
}

pub fn enter() -> anyhow::Result<Terminal<CrosstermBackend<Stdout>>> {
    install_panic_hook();
    if !std::io::stdin().is_tty() {
        anyhow::bail!("no terminal attached to stdin; run rclash in a real terminal");
    }
    enable_raw_mode().context("enable raw mode")?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).context("enter alternate screen")?;
    Terminal::new(CrosstermBackend::new(stdout)).context("create terminal")
}

pub fn leave(mut terminal: Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
    restore();
    terminal.show_cursor().context("show cursor")
}
