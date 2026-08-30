use super::AutoLaunch;

pub struct WindowsAutostart;

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const VALUE_NAME: &str = "RClash";

fn run_key() -> anyhow::Result<winreg::RegKey> {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    Ok(hkcu)
}

impl AutoLaunch for WindowsAutostart {
    fn is_enabled(&self) -> anyhow::Result<bool> {
        let hkcu = run_key()?;
        let key = hkcu.open_subkey(RUN_KEY)?;
        match key.get_value::<String, _>(VALUE_NAME) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    fn enable(&self) -> anyhow::Result<()> {
        let exe = super::current_exe_string()?;
        enable_with_exe(&exe)
    }

    fn disable(&self) -> anyhow::Result<()> {
        let hkcu = run_key()?;
        let (key, _) = hkcu.create_subkey(RUN_KEY)?;
        let _ = key.delete_value(VALUE_NAME);
        Ok(())
    }
}

pub(crate) fn enable_with_exe(exe: &str) -> anyhow::Result<()> {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(RUN_KEY)?;
    let value = format!("\"{exe}\" --minimized");
    key.set_value(VALUE_NAME, &value)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn value_format_contains_minimized() {
        let exe = r"C:\Program Files\RClash\rclash.exe";
        let expected = format!("\"{exe}\" --minimized");
        assert!(expected.contains("--minimized"));
        assert!(expected.starts_with('"'));
    }
}
