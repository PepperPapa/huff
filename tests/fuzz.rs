use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn huff_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_huff"))
}

fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("huff-fuzz-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn lcg(seed: &mut u64) -> u8 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 33) as u8
}

/// 跑一次 `huff decompress`，断言子进程没有 panic。
/// 用 `.output()` 捕获 stderr，避免错误输出泄漏到测试终端；
/// 检查 `panicked at` 而不是 catch_unwind，因为子进程 panic 不会 unwind 到父进程。
fn assert_no_panic(bin: &PathBuf, input: &PathBuf, output: &PathBuf, ctx: &str) {
    let result = Command::new(bin)
        .arg("decompress")
        .arg(input)
        .arg("-o")
        .arg(output)
        .arg("--force")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !stderr.contains("panicked at"),
        "subprocess panicked on {}: {}",
        ctx,
        stderr
    );
}

#[test]
fn fuzz_random_bytes_never_panics() {
    let dir = tempdir("random");
    let bin = huff_bin();
    let mut seed = 0x1234_5678_9abc_def0u64;

    for i in 0..50 {
        let len = (lcg(&mut seed) as usize) % 2048;
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            bytes.push(lcg(&mut seed));
        }
        let input = dir.join(format!("rand-{}.huff", i));
        let output = dir.join(format!("rand-{}.out", i));
        fs::write(&input, &bytes).unwrap();

        assert_no_panic(&bin, &input, &output, &format!("random input len={}", len));
    }

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fuzz_truncated_valid_file_never_panics() {
    let dir = tempdir("truncated");
    let bin = huff_bin();

    let input = dir.join("src.bin");
    let compressed = dir.join("src.huff");
    let data = b"hello huffman, hello world, this is a fuzz seed";
    fs::write(&input, data).unwrap();

    let s = Command::new(&bin)
        .arg("compress")
        .arg(&input)
        .arg("-o")
        .arg(&compressed)
        .status()
        .unwrap();
    assert!(s.success());

    let full = fs::read(&compressed).unwrap();

    for cut in 0..full.len() {
        let truncated = dir.join(format!("trunc-{}.huff", cut));
        let out = dir.join(format!("trunc-{}.out", cut));
        fs::write(&truncated, &full[..cut]).unwrap();

        assert_no_panic(&bin, &truncated, &out, &format!("truncation at {}", cut));
    }

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fuzz_bit_flip_never_panics() {
    let dir = tempdir("bitflip");
    let bin = huff_bin();

    let input = dir.join("src.bin");
    let compressed = dir.join("src.huff");
    let data = b"the quick brown fox jumps over the lazy dog";
    fs::write(&input, data).unwrap();

    let s = Command::new(&bin)
        .arg("compress")
        .arg(&input)
        .arg("-o")
        .arg(&compressed)
        .status()
        .unwrap();
    assert!(s.success());

    let full = fs::read(&compressed).unwrap();

    for pos in 0..full.len().min(64) {
        let mut corrupted = full.clone();
        corrupted[pos] ^= 0xFF;
        let bad = dir.join(format!("bad-{}.huff", pos));
        let out = dir.join(format!("bad-{}.out", pos));
        fs::write(&bad, &corrupted).unwrap();

        assert_no_panic(&bin, &bad, &out, &format!("bit flip at {}", pos));
    }

    let _ = fs::remove_dir_all(&dir);
}
