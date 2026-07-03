use std::path::Path;

use anyhow::{Context, Result};
use transcribe_rs::onnx::moonshine::{MoonshineModel, MoonshineParams, MoonshineVariant};
use transcribe_rs::onnx::parakeet::{ParakeetModel, ParakeetParams};
use transcribe_rs::onnx::Quantization;

pub use wc_models::ModelId;

/// A loaded STT model. Load once, transcribe many times. The variant is chosen
/// by the user (settings → Model); both run int8 ONNX via the same ORT stack.
pub enum Engine {
    Parakeet(ParakeetModel),
    Moonshine(MoonshineModel),
}

impl Engine {
    /// Loads `model` from `model_dir` (the directory containing its ONNX files).
    pub fn load(model: ModelId, model_dir: &Path) -> Result<Self> {
        let dir = model_dir.to_path_buf();
        match model {
            ModelId::Parakeet => {
                let m = ParakeetModel::load(&dir, &Quantization::Int8)
                    .map_err(|e| anyhow::anyhow!("{e}"))
                    .with_context(|| format!("loading Parakeet model from {}", dir.display()))?;
                Ok(Engine::Parakeet(m))
            }
            ModelId::Moonshine => {
                let m = MoonshineModel::load(&dir, MoonshineVariant::Base, &Quantization::Int8)
                    .map_err(|e| anyhow::anyhow!("{e}"))
                    .with_context(|| format!("loading Moonshine model from {}", dir.display()))?;
                Ok(Engine::Moonshine(m))
            }
        }
    }

    /// samples: 16 kHz mono f32 in [-1, 1].
    pub fn transcribe(&mut self, samples: &[f32]) -> Result<String> {
        let text = match self {
            Engine::Parakeet(m) => m
                .transcribe_with(samples, &ParakeetParams::default())
                .map_err(|e| anyhow::anyhow!("{e}"))?
                .text,
            Engine::Moonshine(m) => m
                .transcribe_with(samples, &MoonshineParams::default())
                .map_err(|e| anyhow::anyhow!("{e}"))?
                .text,
        };
        Ok(text.trim().to_string())
    }
}
