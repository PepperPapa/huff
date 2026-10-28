use crate::codec::{decode, encode};
use crate::format::{deserialize, serialize};
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;

/// Compress `input` to `output`.
///
/// # Errors
///
/// Returns an error if `input` is not a regular file, if `output`
/// exists and `force` is false, or if reading/writing fails.
pub fn compress_file(input: &Path, output: &Path, force: bool) -> Result<()> {
    if !input.is_file() {
        bail!("input is not a regular file: {}", input.display());
    }
    if output.exists() && !force {
        bail!(
            "output already exists (pass --force to overwrite): {}",
            output.display()
        );
    }
    let data = fs::read(input).with_context(|| format!("reading {}", input.display()))?;
    let encoded = encode(&data);
    let bytes = serialize(&encoded);
    fs::write(output, &bytes).with_context(|| format!("writing {}", output.display()))?;
    let ratio = if data.is_empty() {
        0.0
    } else {
        size_ratio(bytes.len(), data.len())
    };
    println!(
        "compressed {} -> {} ({} -> {} bytes, ratio {:.3})",
        input.display(),
        output.display(),
        data.len(),
        bytes.len(),
        ratio
    );
    Ok(())
}

/// Decompress `input` to `output`.
///
/// # Errors
///
/// Returns an error if `input` is not a regular file, if `output`
/// exists and `force` is false, if reading fails, if the header or
/// payload is corrupt, or if writing fails.
pub fn decompress_file(input: &Path, output: &Path, force: bool) -> Result<()> {
    if !input.is_file() {
        bail!("input is not a regular file: {}", input.display());
    }
    if output.exists() && !force {
        bail!(
            "output already exists (pass --force to overwrite): {}",
            output.display()
        );
    }
    let bytes = fs::read(input).with_context(|| format!("reading {}", input.display()))?;
    let encoded = deserialize(&bytes).with_context(|| format!("parsing {}", input.display()))?;
    let data = decode(&encoded).with_context(|| format!("decoding {}", input.display()))?;
    fs::write(output, &data).with_context(|| format!("writing {}", output.display()))?;
    println!(
        "decompressed {} -> {} ({} bytes)",
        input.display(),
        output.display(),
        data.len()
    );
    Ok(())
}

// 文件大小远小于 2^52，usize -> f64 无损
#[allow(clippy::cast_precision_loss)]
fn size_ratio(numerator: usize, denominator: usize) -> f64 {
    numerator as f64 / denominator as f64
}
