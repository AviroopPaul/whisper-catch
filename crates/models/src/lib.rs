//! First-run model download: resumable (HTTP Range + .part file),
//! SHA-256 verified against hashes pinned at build time.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

pub struct FileSpec {
    pub name: &'static str,
    pub size: u64,
    pub sha256: &'static str,
}

pub struct ModelSpec {
    pub dir_name: &'static str,
    pub base_url: &'static str,
    pub files: &'static [FileSpec],
}

pub const PARAKEET_V2_INT8: ModelSpec = ModelSpec {
    dir_name: "parakeet-tdt-0.6b-v2-int8",
    base_url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v2-onnx/resolve/main",
    files: &[
        FileSpec {
            name: "encoder-model.int8.onnx",
            size: 652_184_014,
            sha256: "3e0581fda6ab843888b51e56d7ee78b6d5bc3237ec113af1f732d1d5286aa155",
        },
        FileSpec {
            name: "decoder_joint-model.int8.onnx",
            size: 8_998_286,
            sha256: "a449f49acd68979d418651dd2dcb737cc0f1bf0225e009e29ee326354edbf7d3",
        },
        FileSpec {
            name: "nemo128.onnx",
            size: 139_764,
            sha256: "a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f",
        },
        FileSpec {
            name: "vocab.txt",
            size: 9_384,
            sha256: "ec182b70dd42113aff6c5372c75cac58c952443eb22322f57bbd7f53977d497d",
        },
        FileSpec {
            name: "config.json",
            size: 97,
            sha256: "666903c76b9798caf2c210afd4f6cd60b08a8dbf9800ec8d7a3bc0d2148ac466",
        },
    ],
};

impl ModelSpec {
    /// Ensures all model files exist under `models_root/<dir_name>`,
    /// downloading whatever is missing. Returns the model directory.
    pub fn ensure(&self, models_root: &Path) -> Result<PathBuf> {
        let dir = models_root.join(self.dir_name);
        std::fs::create_dir_all(&dir)?;
        for f in self.files {
            let dest = dir.join(f.name);
            // size check only for existing files — full hash verify happens on
            // download; a hash pass over 650MB at every startup isn't worth it
            if dest.metadata().map(|m| m.len()).ok() == Some(f.size) {
                continue;
            }
            download(&format!("{}/{}", self.base_url, f.name), &dest, f)?;
        }
        Ok(dir)
    }
}

fn download(url: &str, dest: &Path, spec: &FileSpec) -> Result<()> {
    let part = dest.with_extension(
        dest.extension()
            .map(|e| format!("{}.part", e.to_string_lossy()))
            .unwrap_or_else(|| "part".into()),
    );

    // resume: hash the existing partial file so the final digest stays valid
    let mut hasher = Sha256::new();
    let mut offset = 0u64;
    if let Ok(existing) = std::fs::read(&part) {
        if (existing.len() as u64) < spec.size {
            hasher.update(&existing);
            offset = existing.len() as u64;
        } else {
            std::fs::remove_file(&part).ok();
        }
    }

    log::info!(
        "downloading {} ({:.0} MB){}",
        spec.name,
        spec.size as f64 / 1e6,
        if offset > 0 {
            format!(" — resuming at {:.0} MB", offset as f64 / 1e6)
        } else {
            String::new()
        }
    );

    let mut req = ureq::get(url);
    if offset > 0 {
        req = req.header("Range", &format!("bytes={offset}-"));
    }
    let mut resp = req.call().with_context(|| format!("GET {url}"))?;
    if offset > 0 && resp.status() != 206 {
        // server ignored the Range header; start over
        log::warn!("server ignored resume request, restarting {}", spec.name);
        hasher = Sha256::new();
        offset = 0;
        std::fs::remove_file(&part).ok();
    }

    let mut out = std::fs::OpenOptions::new()
        .create(true)
        .append(offset > 0)
        .write(true)
        .truncate(offset == 0)
        .open(&part)
        .with_context(|| format!("opening {}", part.display()))?;

    let mut reader = resp.body_mut().as_reader();
    let mut buf = vec![0u8; 1 << 20];
    let mut done = offset;
    let mut last_pct = 0;
    loop {
        let n = reader.read(&mut buf).context("reading response body")?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        out.write_all(&buf[..n])?;
        done += n as u64;
        let pct = (done * 100 / spec.size) as u32;
        if pct >= last_pct + 10 {
            log::info!("  {} — {pct}%", spec.name);
            last_pct = pct;
        }
    }
    out.flush()?;
    drop(out);

    if done != spec.size {
        bail!(
            "{}: incomplete download ({done} of {} bytes) — rerun to resume",
            spec.name,
            spec.size
        );
    }
    let digest = hex::encode(hasher.finalize());
    if digest != spec.sha256 {
        std::fs::remove_file(&part).ok();
        bail!("{}: checksum mismatch (got {digest}) — corrupted download removed, rerun", spec.name);
    }
    std::fs::rename(&part, dest)?;
    log::info!("  {} — verified", spec.name);
    Ok(())
}
