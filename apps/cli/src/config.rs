use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// PTT key: rctrl, lctrl, ralt, lalt, super, f13, scrolllock
    pub key: String,
    /// Model directory override; defaults to <data-dir>/whisper-catch/models/<default model>
    pub model_dir: Option<PathBuf>,
    /// Keep a local log of transcriptions (history.jsonl)
    pub history: bool,
    /// UI theme: system, light, dark
    pub theme: String,
    /// Type words live while speaking instead of all at once on release
    pub streaming: bool,
    /// Show the floating recording indicator while dictating
    pub overlay: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            key: "ralt".into(),
            model_dir: None,
            history: true,
            theme: "system".into(),
            streaming: true,
            overlay: true,
        }
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .expect("no config dir on this platform")
        .join("whisper-catch")
        .join("config.toml")
}

/// Loads config, writing the default file on first run so users can find it.
pub fn load() -> Result<Config> {
    let path = config_path();
    if !path.exists() {
        let cfg = Config::default();
        save(&cfg)?;
        log::info!("wrote default config to {}", path.display());
        return Ok(cfg);
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

pub fn save(cfg: &Config) -> Result<()> {
    let path = config_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, toml::to_string_pretty(cfg)?)
        .with_context(|| format!("writing {}", path.display()))
}
