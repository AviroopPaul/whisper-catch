//! Engine smoke test without audio-capture deps:
//! cargo run -p wc-core --no-default-features --example transcribe_wav -- <wav>

use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let wav = PathBuf::from(std::env::args().nth(1).expect("usage: transcribe_wav <wav>"));
    let model_dir = wc_core::models_dir().join("parakeet-tdt-0.6b-v2-int8");

    let t0 = std::time::Instant::now();
    let mut engine = wc_core::engine::Engine::load(&model_dir)?;
    eprintln!("model loaded in {:.1}s", t0.elapsed().as_secs_f32());

    let samples = transcribe_rs::audio::read_wav_samples(&wav).map_err(|e| anyhow::anyhow!("{e}"))?;
    let audio_secs = samples.len() as f32 / 16_000.0;

    let t0 = std::time::Instant::now();
    let text = engine.transcribe(&samples)?;
    let dt = t0.elapsed().as_secs_f32();
    eprintln!("{audio_secs:.1}s audio → {dt:.2}s inference ({:.1}x realtime)", audio_secs / dt);
    println!("{text}");
    Ok(())
}
