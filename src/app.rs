use crate::codec::{decode, encode};
use crate::format::{deserialize, serialize};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

pub fn compress_file(input: &Path, output: &Path, force: bool) -> Result<()> {
    if !input.is_file() {
        bail!("input is not a regular file: {}", input.display());
    }
    if output.exists() && !force {
        bail!("output already exists (pass --force to overwrite): {}", output.display());
    }
    let data = fs::read(input)
        .with_context(|| format!("reading {}", input.display()))?;
    let encoded = encode(&data);
    let bytes = serialize(&encoded);
    fs::write(output, &bytes)
        .with_context(|| format!("writing {}", output.display()))?;
    let ratio = if data.is_empty() {
        0.0
    } else {
        bytes.len() as f64 / data.len() as f64
    };
    println!(
        "compressed {} -> {} ({} -> {} bytes, ratio {:.3})",
        input.display(), output.display(), data.len(), bytes.len(), ratio
    );
    Ok(())
}

pub fn decompress_file(input: &Path, output: &Path, force: bool) -> Result<()> {
    if !input.is_file() {
        bail!("input is not a regular file: {}", input.display());
    }
    if output.exists() && !force {
        bail!("output already exists (pass --force to overwrite): {}", output.display());
    }
    let bytes = fs::read(input)
        .with_context(|| format!("reading {}", input.display()))?;
    let encoded = deserialize(&bytes)
        .with_context(|| format!("parsing {}", input.display()))?;
    let data = decode(&encoded)
        .with_context(|| format!("decoding {}", input.display()))?;
    fs::write(output, &data)
        .with_context(|| format!("writing {}", output.display()))?;
    println!(
        "decompressed {} -> {} ({} bytes)",
        input.display(), output.display(), data.len()
    );
    Ok(())
}