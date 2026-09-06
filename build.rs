fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icons/windows/icon.ico");
        if let Err(e) = res.compile() {
            eprintln!("winres failed: {e}");
        }
    }
}
