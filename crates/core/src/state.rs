use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Shared between the PTT loop and the tray UI.
pub struct AppState {
    pub enabled: AtomicBool,
    pub recording: AtomicBool,
    pub stats: Mutex<Stats>,
}

#[derive(Default, Clone, Copy)]
pub struct Stats {
    pub utterances: u64,
    pub words: u64,
    pub audio_secs: f32,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
            recording: AtomicBool::new(false),
            stats: Mutex::new(Stats::default()),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn record_utterance(&self, words: usize, audio_secs: f32) {
        let mut s = self.stats.lock().unwrap();
        s.utterances += 1;
        s.words += words as u64;
        s.audio_secs += audio_secs;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
