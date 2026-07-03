//! XDG autostart: ~/.config/autostart/whisper-catch.desktop

use anyhow::{Context, Result};
use std::path::PathBuf;

fn desktop_path() -> PathBuf {
    dirs::config_dir()
        .expect("no config dir on this platform")
        .join("autostart")
        .join("whisper-catch.desktop")
}

pub fn enable() -> Result<()> {
    let exe = std::env::current_exe().context("resolving own binary path")?;
    let path = desktop_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(
        &path,
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=WhisprCatch\n\
             Comment=Local push-to-talk dictation\n\
             Exec={} ptt\n\
             Icon=audio-input-microphone\n\
             Terminal=false\n\
             X-GNOME-Autostart-enabled=true\n",
            exe.display()
        ),
    )
    .with_context(|| format!("writing {}", path.display()))?;
    log::info!("autostart enabled: {}", path.display());
    Ok(())
}

pub fn disable() -> Result<()> {
    let path = desktop_path();
    match std::fs::remove_file(&path) {
        Ok(()) => log::info!("autostart disabled"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            log::info!("autostart was not enabled")
        }
        Err(e) => return Err(e).with_context(|| format!("removing {}", path.display())),
    }
    Ok(())
}
