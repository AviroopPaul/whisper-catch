mod autostart;
mod config;
mod settings_app;

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use wc_core::audio::Capture;
use wc_core::engine::Engine;
use wc_core::state::AppState;
use wc_hotkey::{PttEvent, PttKey};
use wc_inject::Injector;

#[derive(Parser)]
#[command(name = "whisper-catch", about = "Local push-to-talk dictation")]
struct Cli {
    /// Model directory (overrides config; defaults to <data-dir>/whisper-catch/models/parakeet-tdt-0.6b-v2-int8)
    #[arg(long, global = true)]
    model: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Transcribe a WAV file (engine smoke test)
    Transcribe { wav: PathBuf },
    /// Record N seconds from the mic, then transcribe
    Record {
        #[arg(long, default_value_t = 5)]
        seconds: u64,
    },
    /// Push-to-talk daemon: hold key, speak, release; text is typed at cursor
    Ptt {
        /// PTT key: rctrl, lctrl, ralt, lalt, super, f13, scrolllock (overrides config)
        #[arg(long)]
        key: Option<String>,
        /// Print transcripts to stdout instead of typing them
        #[arg(long)]
        print_only: bool,
        /// Run without the system tray icon
        #[arg(long)]
        no_tray: bool,
    },
    /// Open the settings & history window
    Settings,
    /// Download the default model without starting the daemon
    DownloadModel,
    /// Start whisper-catch automatically on login
    Autostart {
        #[arg(long, conflicts_with = "disable")]
        enable: bool,
        #[arg(long)]
        disable: bool,
    },
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();
    let cfg = config::load()?;

    // subcommands that don't need the engine
    match &cli.cmd {
        Cmd::Settings => return settings_app::run(),
        Cmd::DownloadModel => {
            let dir = wc_models::PARAKEET_V2_INT8.ensure(&wc_core::models_dir())?;
            log::info!("model ready at {}", dir.display());
            return Ok(());
        }
        Cmd::Autostart { enable, disable } => {
            if *disable {
                autostart::disable()?;
            } else {
                let _ = enable; // --enable is the default action
                autostart::enable()?;
            }
            return Ok(());
        }
        _ => {}
    }

    let model_dir = match cli.model.or_else(|| cfg.model_dir.clone()) {
        Some(dir) => dir, // explicit dir: user manages it, don't auto-download
        None => wc_models::PARAKEET_V2_INT8
            .ensure(&wc_core::models_dir())
            .context("fetching default model")?,
    };

    log::info!("loading model from {}", model_dir.display());
    let t0 = std::time::Instant::now();
    let mut engine = Engine::load(&model_dir)?;
    log::info!("model loaded in {:.1}s", t0.elapsed().as_secs_f32());

    match cli.cmd {
        Cmd::Transcribe { wav } => {
            let samples = transcribe_rs::audio::read_wav_samples(&wav)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| format!("reading {}", wav.display()))?;
            let t0 = std::time::Instant::now();
            let text = engine.transcribe(&samples)?;
            log::info!("inference took {:.2}s", t0.elapsed().as_secs_f32());
            println!("{text}");
        }
        Cmd::Record { seconds } => {
            let cap = Capture::open()?;
            cap.begin();
            eprintln!("recording {seconds}s — speak now...");
            std::thread::sleep(Duration::from_secs(seconds));
            let samples = cap.end()?;
            let t0 = std::time::Instant::now();
            let text = engine.transcribe(&samples)?;
            log::info!("inference took {:.2}s", t0.elapsed().as_secs_f32());
            println!("{text}");
        }
        Cmd::Ptt {
            key,
            print_only,
            no_tray,
        } => {
            let key = PttKey::parse(key.as_deref().unwrap_or(&cfg.key))?;
            run_ptt(engine, key, print_only, no_tray, cfg.history)?;
        }
        Cmd::Settings | Cmd::DownloadModel | Cmd::Autostart { .. } => unreachable!(),
    }
    Ok(())
}

fn run_ptt(
    mut engine: Engine,
    key: PttKey,
    print_only: bool,
    no_tray: bool,
    keep_history: bool,
) -> Result<()> {
    let state = Arc::new(AppState::new());
    let tray = if no_tray {
        None
    } else {
        match wc_tray::spawn(state.clone()) {
            Ok(t) => Some(t),
            Err(e) => {
                log::warn!("{e:#} — continuing without tray");
                None
            }
        }
    };
    let refresh = |t: &Option<wc_tray::TrayHandle>| {
        if let Some(t) = t {
            t.refresh();
        }
    };

    let capture = Capture::open()?;
    let events = wc_hotkey::listen(key)?;
    let mut injector = if print_only {
        None
    } else {
        Some(Injector::new()?)
    };
    eprintln!("ready — hold {key:?} and speak, release to type. Ctrl-C to quit.");

    let mut armed = false;
    for ev in events {
        match ev {
            PttEvent::Pressed => {
                if !state.is_enabled() || armed {
                    continue;
                }
                capture.begin();
                armed = true;
                log::info!("recording...");
                state.recording.store(true, Ordering::Relaxed);
                refresh(&tray);
            }
            PttEvent::Released => {
                if !armed {
                    continue;
                }
                armed = false;
                state.recording.store(false, Ordering::Relaxed);
                refresh(&tray);
                let dur = capture.armed_secs();
                if dur < 0.3 {
                    capture.cancel();
                    log::info!("too short ({dur:.2}s), ignored");
                    continue;
                }
                let samples = match capture.end() {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("audio processing failed: {e:#}");
                        continue;
                    }
                };
                let t0 = std::time::Instant::now();
                match engine.transcribe(&samples) {
                    Ok(text) if text.is_empty() => log::info!("(empty transcript)"),
                    Ok(text) => {
                        let infer_s = t0.elapsed().as_secs_f32();
                        log::info!("{dur:.1}s audio → {infer_s:.2}s inference");
                        state.record_utterance(text.split_whitespace().count(), dur);
                        if keep_history {
                            let entry = wc_core::history::Entry {
                                ts: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs())
                                    .unwrap_or(0),
                                dur_s: dur,
                                infer_s,
                                text: text.clone(),
                            };
                            if let Err(e) = wc_core::history::append(&entry) {
                                log::warn!("history write failed: {e:#}");
                            }
                        }
                        refresh(&tray);
                        if let Some(inj) = injector.as_mut() {
                            // let the user finish releasing the modifier so
                            // injected keys don't combine with it
                            std::thread::sleep(Duration::from_millis(150));
                            if let Err(e) = inj.type_text(&text) {
                                log::error!("injection failed: {e:#}");
                                println!("{text}");
                            }
                        } else {
                            println!("{text}");
                        }
                    }
                    Err(e) => log::error!("transcription failed: {e:#}"),
                }
            }
        }
    }
    Ok(())
}
