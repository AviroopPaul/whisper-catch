use std::path::Path;

use anyhow::{Context, Result};
use transcribe_rs::onnx::parakeet::{ParakeetModel, ParakeetParams};
use transcribe_rs::onnx::Quantization;

/// Wraps a loaded STT model. Load once, transcribe many times.
pub struct Engine {
    model: ParakeetModel,
}

impl Engine {
    pub fn load(model_dir: &Path) -> Result<Self> {
        let model = ParakeetModel::load(&model_dir.to_path_buf(), &Quantization::Int8)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("loading parakeet model from {}", model_dir.display()))?;
        Ok(Self { model })
    }

    /// samples: 16 kHz mono f32 in [-1, 1].
    pub fn transcribe(&mut self, samples: &[f32]) -> Result<String> {
        let result = self
            .model
            .transcribe_with(samples, &ParakeetParams::default())
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(result.text.trim().to_string())
    }
}
