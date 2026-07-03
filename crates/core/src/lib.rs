pub mod engine;
pub mod history;
pub mod state;

#[cfg(feature = "capture")]
pub mod audio;

use std::path::PathBuf;

/// 16 kHz mono f32 — the sample format every STT engine here consumes.
pub const SAMPLE_RATE: u32 = 16_000;

/// Default model storage: ~/.local/share/whisper-catch/models (XDG) or platform equivalent.
pub fn models_dir() -> PathBuf {
    dirs::data_dir()
        .expect("no data dir on this platform")
        .join("whisper-catch")
        .join("models")
}
