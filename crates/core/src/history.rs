//! Transcription history: append-only JSONL at
//! <data-dir>/whisper-catch/history.jsonl. Local only, user-clearable.

use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// Unix timestamp (seconds)
    pub ts: u64,
    /// Utterance length in seconds
    pub dur_s: f32,
    /// Inference time in seconds
    pub infer_s: f32,
    pub text: String,
}

pub fn history_path() -> PathBuf {
    dirs::data_dir()
        .expect("no data dir on this platform")
        .join("whisper-catch")
        .join("history.jsonl")
}

pub fn append(entry: &Entry) -> Result<()> {
    let path = history_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("opening {}", path.display()))?;
    writeln!(f, "{}", serde_json::to_string(entry)?)?;
    Ok(())
}

/// Returns up to `limit` most-recent entries, newest first.
/// Malformed lines are skipped rather than failing the whole load.
pub fn load(limit: usize) -> Result<Vec<Entry>> {
    let path = history_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path)?;
    let mut entries: Vec<Entry> = raw
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    entries.reverse();
    entries.truncate(limit);
    Ok(entries)
}

pub fn clear() -> Result<()> {
    match std::fs::remove_file(history_path()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// All-time totals: (utterances, words, audio seconds).
pub fn totals() -> (u64, u64, f32) {
    load(usize::MAX)
        .unwrap_or_default()
        .iter()
        .fold((0, 0, 0.0), |(n, w, s), e| {
            (n + 1, w + e.text.split_whitespace().count() as u64, s + e.dur_s)
        })
}
