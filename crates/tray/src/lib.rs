//! System tray (StatusNotifierItem via ksni on Linux).
//! GNOME needs the AppIndicator extension; app stays usable without a tray.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use anyhow::Result;
use wc_core::state::AppState;

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use ksni::blocking::{Handle, TrayMethods};
    use ksni::menu::{CheckmarkItem, MenuItem, StandardItem};

    pub struct WcTray {
        state: Arc<AppState>,
        /// Own binary path, resolved at startup — current_exe() goes stale
        /// ("… (deleted)") once a package upgrade replaces the binary.
        exe: std::path::PathBuf,
    }

    impl ksni::Tray for WcTray {
        fn id(&self) -> String {
            "whisper-catch".into()
        }

        fn icon_name(&self) -> String {
            if self.state.recording.load(Ordering::Relaxed) {
                "media-record-symbolic".into()
            } else if self.state.is_enabled() {
                // Own app icon; SNI resolves it from hicolor when installed via
                // the .deb. Running uninstalled, GNOME falls back to a generic
                // glyph — acceptable for dev runs.
                "whisper-catch".into()
            } else {
                "microphone-sensitivity-muted-symbolic".into()
            }
        }

        fn title(&self) -> String {
            "WhisprCatch".into()
        }

        fn tool_tip(&self) -> ksni::ToolTip {
            let s = *self.state.stats.lock().unwrap();
            ksni::ToolTip {
                title: "WhisprCatch".into(),
                description: format!(
                    "{} words · {} utterances · {:.0}s audio",
                    s.words, s.utterances, s.audio_secs
                ),
                ..Default::default()
            }
        }

        fn menu(&self) -> Vec<MenuItem<Self>> {
            let s = *self.state.stats.lock().unwrap();
            vec![
                CheckmarkItem {
                    label: "Listening".into(),
                    checked: self.state.is_enabled(),
                    activate: Box::new(|this: &mut Self| {
                        let now = !this.state.is_enabled();
                        this.state.enabled.store(now, Ordering::Relaxed);
                        log::info!("listening {}", if now { "enabled" } else { "disabled" });
                    }),
                    ..Default::default()
                }
                .into(),
                MenuItem::Separator,
                StandardItem {
                    label: format!("{} words · {} utterances", s.words, s.utterances),
                    enabled: false,
                    ..Default::default()
                }
                .into(),
                MenuItem::Separator,
                StandardItem {
                    label: "History & Settings…".into(),
                    icon_name: "preferences-system".into(),
                    activate: Box::new(|this: &mut Self| {
                        if let Err(e) =
                            std::process::Command::new(&this.exe).arg("settings").spawn()
                        {
                            log::error!("failed to open settings: {e}");
                        }
                    }),
                    ..Default::default()
                }
                .into(),
                StandardItem {
                    label: "Quit".into(),
                    icon_name: "application-exit".into(),
                    activate: Box::new(|_| std::process::exit(0)),
                    ..Default::default()
                }
                .into(),
            ]
        }
    }

    /// Spawns the tray. Returns a handle used to refresh icon/menu after
    /// state changes. Fails gracefully when no StatusNotifierWatcher exists.
    pub fn spawn(state: Arc<AppState>) -> Result<TrayHandle> {
        let exe = std::env::current_exe().unwrap_or_else(|_| "whisper-catch".into());
        let handle = WcTray { state, exe }
            .spawn()
            .map_err(|e| anyhow::anyhow!("tray unavailable: {e}"))?;
        Ok(TrayHandle(handle))
    }

    pub struct TrayHandle(Handle<WcTray>);

    impl TrayHandle {
        pub fn refresh(&self) {
            let _ = self.0.update(|_| {});
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::{spawn, TrayHandle};

#[cfg(not(target_os = "linux"))]
pub struct TrayHandle;

#[cfg(not(target_os = "linux"))]
impl TrayHandle {
    pub fn refresh(&self) {}
}

#[cfg(not(target_os = "linux"))]
pub fn spawn(_state: Arc<AppState>) -> Result<TrayHandle> {
    anyhow::bail!("tray not implemented for this platform yet")
}
