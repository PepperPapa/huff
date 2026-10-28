use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn huff_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_huff"))
}

fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("huff-test-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn cli_roundtrip_binary() {
    let dir = tempdir("roundtrip");
    let input = dir.join("input.bin");
    let compressed = dir.join("input.bin.huff");
    let output = dir.join("output.bin");

    let data: Vec<u8> = (0..4096u32).map(|i| ((i * 7 + 3) % 256) as u8).collect();
    fs::write(&input, &data).unwrap();

    let bin = huff_bin();

    let s1 = Command::new(&bin)
        .arg("compress")
        .arg(&input)
        .arg("-o")
        .arg(&compressed)
        .status()
        .unwrap();
    assert!(s1.success(), "compress failed");

    let s2 = Command::new(&bin)
        .arg("decompress")
        .arg(&compressed)
        .arg("-o")
        .arg(&output)
        .status()
        .unwrap();
    assert!(s2.success(), "decompress failed");

    assert_eq!(fs::read(&output).unwrap(), data);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cli_decompress_rejects_garbage() {
    let dir = tempdir("garbage");
    let input = dir.join("garbage.huff");
    let output = dir.join("out.bin");
    fs::write(&input, b"NOT-A-HUFF-FILE").unwrap();

    let bin = huff_bin();
    let s = Command::new(&bin)
        .arg("decompress")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .status()
        .unwrap();
    assert!(!s.success(), "should reject garbage input");
    assert!(!output.exists(), "should not create output on error");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cli_refuses_to_overwrite_without_force() {
    let dir = tempdir("overwrite");
    let input = dir.join("input.bin");
    let compressed = dir.join("input.bin.huff");
    fs::write(&input, b"hello huffman").unwrap();
    fs::write(&compressed, b"existing").unwrap();

    let bin = huff_bin();
    let s = Command::new(&bin)
        .arg("compress")
        .arg(&input)
        .arg("-o")
        .arg(&compressed)
        .status()
        .unwrap();
    assert!(!s.success(), "should refuse to overwrite without --force");
    assert_eq!(
        fs::read(&compressed).unwrap(),
        b"existing",
        "file must be untouched"
    );

    let s = Command::new(&bin)
        .arg("compress")
        .arg(&input)
        .arg("-o")
        .arg(&compressed)
        .arg("--force")
        .status()
        .unwrap();
    assert!(s.success(), "should overwrite with --force");

    let _ = fs::remove_dir_all(&dir);
}
