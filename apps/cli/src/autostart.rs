//! Start WhisprCatch on login.
//! Linux: XDG autostart (~/.config/autostart/whisper-catch.desktop).
//! macOS: a LaunchAgent (~/Library/LaunchAgents/com.whisprcatch.agent.plist).

use anyhow::{Context, Result};
use std::path::PathBuf;

#[cfg(target_os = "macos")]
pub const LAUNCH_AGENT_LABEL: &str = "com.whisprcatch.agent";

#[cfg(target_os = "linux")]
fn desktop_path() -> PathBuf {
    dirs::config_dir()
        .expect("no config dir on this platform")
        .join("autostart")
        .join("whisper-catch.desktop")
}

/// Path to the file that, when present, makes WhisprCatch start on login.
pub fn autostart_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .expect("no home dir")
            .join("Library/LaunchAgents")
            .join(format!("{LAUNCH_AGENT_LABEL}.plist"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        #[cfg(target_os = "linux")]
        {
            desktop_path()
        }
        #[cfg(not(target_os = "linux"))]
        {
            dirs::config_dir().unwrap_or_default().join("whisper-catch-autostart")
        }
    }
}

/// True when start-on-login is currently enabled.
pub fn is_enabled() -> bool {
    autostart_path().exists()
}

pub fn enable() -> Result<()> {
    let exe = std::env::current_exe().context("resolving own binary path")?;
    let path = autostart_path();
    std::fs::create_dir_all(path.parent().unwrap())?;

    #[cfg(target_os = "macos")]
    {
        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
        <string>ptt</string>
    </array>
    <key>RunAtLoad</key><true/>
    <key>ProcessType</key><string>Interactive</string>
    <key>LimitLoadToSessionType</key><string>Aqua</string>
</dict>
</plist>
"#,
            label = LAUNCH_AGENT_LABEL,
            exe = exe.display(),
        );
        std::fs::write(&path, plist).with_context(|| format!("writing {}", path.display()))?;
        // (Re)load so it takes effect this session too; ignore load errors
        // (already-loaded is common).
        let _ = std::process::Command::new("launchctl")
            .args(["unload", &path.to_string_lossy()])
            .status();
        let _ = std::process::Command::new("launchctl")
            .args(["load", "-w", &path.to_string_lossy()])
            .status();
        log::info!("autostart enabled: {}", path.display());
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        std::fs::write(
            &path,
            format!(
                "[Desktop Entry]\n\
                 Type=Application\n\
                 Name=WhisprCatch\n\
                 Comment=Local push-to-talk dictation\n\
                 Exec={} ptt\n\
                 Icon=whisper-catch\n\
                 Terminal=false\n\
                 X-GNOME-Autostart-enabled=true\n",
                exe.display()
            ),
        )
        .with_context(|| format!("writing {}", path.display()))?;
        log::info!("autostart enabled: {}", path.display());
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        anyhow::bail!("autostart not supported on this platform")
    }
}

pub fn disable() -> Result<()> {
    let path = autostart_path();

    #[cfg(target_os = "macos")]
    if path.exists() {
        let _ = std::process::Command::new("launchctl")
            .args(["unload", "-w", &path.to_string_lossy()])
            .status();
    }

    match std::fs::remove_file(&path) {
        Ok(()) => log::info!("autostart disabled"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            log::info!("autostart was not enabled")
        }
        Err(e) => return Err(e).with_context(|| format!("removing {}", path.display())),
    }
    Ok(())
}
