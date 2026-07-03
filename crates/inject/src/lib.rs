//! Types transcribed text into the focused window.
//!
//! X11: enigo → XTEST, works everywhere. Wayland: enigo tries libei/portal,
//! flaky — the uinput virtual-keyboard cascade lands post-MVP (SCOPE.md §3).

use anyhow::{Context, Result};
use enigo::{Enigo, Keyboard, Settings};

pub struct Injector {
    enigo: Enigo,
    #[cfg(target_os = "linux")]
    x11: Option<x11rb::rust_connection::RustConnection>,
}

impl Injector {
    pub fn new() -> Result<Self> {
        let enigo = Enigo::new(&Settings::default())
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("initializing input injection")?;
        #[cfg(target_os = "linux")]
        let x11 = x11rb::connect(None).ok().map(|(c, _)| c);
        Ok(Self {
            enigo,
            #[cfg(target_os = "linux")]
            x11,
        })
    }

    pub fn type_text(&mut self, text: &str) -> Result<()> {
        self.enigo
            .text(text)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("typing text")
    }

    /// Fakes a release of a physically-held key at the display-server level,
    /// so text injected *while the PTT modifier is held* doesn't turn into
    /// modifier+letter shortcuts. The kernel-level evdev listener still sees
    /// the real release later. Best-effort; no-op off X11.
    #[cfg(target_os = "linux")]
    pub fn lift_key(&mut self, evdev_code: u16) {
        use x11rb::protocol::xtest::ConnectionExt as _;
        if let Some(conn) = &self.x11 {
            let keycode = (evdev_code + 8) as u8;
            // 3 = KeyRelease
            let _ = conn.xtest_fake_input(3, keycode, x11rb::CURRENT_TIME, 0u32, 0, 0, 0);
            let _ = x11rb::connection::Connection::flush(conn);
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn lift_key(&mut self, _evdev_code: u16) {}
}
