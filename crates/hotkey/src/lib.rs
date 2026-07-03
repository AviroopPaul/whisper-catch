//! Global push-to-talk hotkey via raw evdev (Linux).
//!
//! Listen-only: never grabs the device, so a key also bound in the
//! compositor will fire both — pick a low-conflict key (default Right-Ctrl).
//! Requires read access to /dev/input/event* (`input` group membership).

use std::sync::mpsc::{self, Receiver};
use std::thread;

use anyhow::{bail, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PttEvent {
    Pressed,
    Released,
}

/// Named keys we support as PTT triggers. Maps to evdev key codes on Linux.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PttKey {
    RightCtrl,
    LeftCtrl,
    RightAlt,
    LeftAlt,
    Super,
    F13,
    ScrollLock,
}

impl PttKey {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "rightctrl" | "rctrl" => Self::RightCtrl,
            "leftctrl" | "lctrl" => Self::LeftCtrl,
            "rightalt" | "ralt" => Self::RightAlt,
            "leftalt" | "lalt" => Self::LeftAlt,
            "super" | "meta" | "win" => Self::Super,
            "f13" => Self::F13,
            "scrolllock" => Self::ScrollLock,
            other => bail!("unknown PTT key '{other}' (try: rctrl, lctrl, ralt, lalt, super, f13, scrolllock)"),
        })
    }
}

/// True when at least one keyboard-capable input device is readable —
/// i.e. hotkey listening will work without further permission setup.
#[cfg(target_os = "linux")]
pub fn keyboard_accessible() -> bool {
    evdev::enumerate().any(|(_, d)| {
        d.supported_keys()
            .map(|k| k.contains(evdev::KeyCode::KEY_A))
            .unwrap_or(false)
    })
}

#[cfg(not(target_os = "linux"))]
pub fn keyboard_accessible() -> bool {
    false
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use evdev::{Device, EventSummary, KeyCode};

    impl PttKey {
        fn code(self) -> KeyCode {
            match self {
                PttKey::RightCtrl => KeyCode::KEY_RIGHTCTRL,
                PttKey::LeftCtrl => KeyCode::KEY_LEFTCTRL,
                PttKey::RightAlt => KeyCode::KEY_RIGHTALT,
                PttKey::LeftAlt => KeyCode::KEY_LEFTALT,
                PttKey::Super => KeyCode::KEY_LEFTMETA,
                PttKey::F13 => KeyCode::KEY_F13,
                PttKey::ScrollLock => KeyCode::KEY_SCROLLLOCK,
            }
        }
    }

    /// Spawns one reader thread per keyboard-capable device; events funnel
    /// into a single channel. No hotplug rescan yet (MVP).
    pub fn listen(key: PttKey) -> Result<Receiver<PttEvent>> {
        let code = key.code();
        let devices: Vec<(std::path::PathBuf, Device)> = evdev::enumerate()
            .filter(|(_, d)| {
                d.supported_keys()
                    .map(|keys| keys.contains(code))
                    .unwrap_or(false)
            })
            .collect();

        if devices.is_empty() {
            bail!(
                "no readable input device supports {:?} — are you in the `input` group? \
                 (sudo usermod -aG input $USER, then re-login)",
                key
            );
        }

        let (tx, rx) = mpsc::channel();
        for (path, mut dev) in devices {
            log::info!("listening on {} ({})", path.display(), dev.name().unwrap_or("?"));
            let tx = tx.clone();
            thread::spawn(move || loop {
                let events = match dev.fetch_events() {
                    Ok(ev) => ev,
                    Err(e) => {
                        log::warn!("device read failed (unplugged?): {e}");
                        return;
                    }
                };
                for ev in events {
                    if let EventSummary::Key(_, c, value) = ev.destructure() {
                        if c == code {
                            // value: 1 = press, 0 = release, 2 = autorepeat (ignore)
                            let msg = match value {
                                1 => Some(PttEvent::Pressed),
                                0 => Some(PttEvent::Released),
                                _ => None,
                            };
                            if let Some(m) = msg {
                                if tx.send(m).is_err() {
                                    return;
                                }
                            }
                        }
                    }
                }
            });
        }
        Ok(rx)
    }
}

#[cfg(target_os = "linux")]
pub use linux::listen;

#[cfg(not(target_os = "linux"))]
pub fn listen(_key: PttKey) -> Result<Receiver<PttEvent>> {
    bail!("hotkey listener not implemented for this platform yet")
}
