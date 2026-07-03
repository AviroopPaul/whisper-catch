//! Types transcribed text into the focused window.
//!
//! X11: enigo → XTEST, works everywhere. Wayland: enigo tries libei/portal,
//! flaky — the uinput virtual-keyboard cascade lands post-MVP (SCOPE.md §3).

use anyhow::{Context, Result};
use enigo::{Enigo, Keyboard, Settings};

pub struct Injector {
    enigo: Enigo,
}

impl Injector {
    pub fn new() -> Result<Self> {
        let enigo = Enigo::new(&Settings::default())
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("initializing input injection")?;
        Ok(Self { enigo })
    }

    pub fn type_text(&mut self, text: &str) -> Result<()> {
        self.enigo
            .text(text)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("typing text")
    }
}
