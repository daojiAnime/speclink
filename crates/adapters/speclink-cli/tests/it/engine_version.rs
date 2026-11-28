//! `speclink --version` 的引擎版號查詢面（spec「引擎版號查詢面」）。
//!
//! 「這顆 binary 的引擎是哪版」以前無從問起——過期的 app 就這樣裝了進來。
//! 版號輸出把它變成一條指令可斷言，本機安裝腳本的兩道斷言也建立在這上面；
//! 在 Git 源碼樹內建置時一併附帶編譯期 8 位小寫短提交雜湊與已追蹤工作區 `-dirty` 標記。

#[path = "../../src/git_rev.rs"]
mod git_rev;

use speclink_core::init::ASSET_VERSION;
use std::process::Command;

/// Scenario「--version 含引擎版號」＋「脏工作区构建输出带 dirty 标记」＋ Example「版號輸出格式」。
#[test]
fn version_prints_the_package_arch_engine_and_git_rev_on_one_line() {
    let out = Command::new(env!("CARGO_BIN_EXE_speclink"))
        .arg("--version")
        .output()
        .expect("run speclink binary");

    assert!(out.status.success(), "--version must exit 0");
    assert!(
        out.stderr.is_empty(),
        "--version must not write to stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        stdout.lines().count(),
        1,
        "stdout must be a single line: {stdout}"
    );

    let no_color = Command::new(env!("CARGO_BIN_EXE_speclink"))
        .args(["--version", "--no-color"])
        .output()
        .expect("run speclink --version --no-color");
    assert!(no_color.status.success(), "--version --no-color must exit 0");
    assert!(no_color.stderr.is_empty(), "stderr must be empty");
    assert_eq!(
        no_color.stdout, out.stdout,
        "--version --no-color must match --version byte-for-byte"
    );

    let expected_rev = git_rev::resolve_for_manifest(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
        &mut |_| {},
    );
    if let Some(ref rev) = expected_rev {
        let sha = rev.strip_suffix("-dirty").unwrap_or(rev);
        assert!(
            sha.len() == 8 && sha.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
            "short sha must be 8 lowercase hex chars: {rev}"
        );
    }

    let pkg = env!("CARGO_PKG_VERSION");
    let arch: Option<&str> = if cfg!(target_arch = "x86_64") {
        Some("x64")
    } else if cfg!(target_arch = "aarch64") {
        Some("arm64")
    } else {
        None
    };

    let expected = match (arch, expected_rev.as_deref()) {
        (Some(arch), Some(rev)) => format!("speclink {pkg} ({arch}, engine {ASSET_VERSION}, {rev})"),
        (None, Some(rev)) => format!("speclink {pkg} (engine {ASSET_VERSION}, {rev})"),
        (Some(arch), None) => format!("speclink {pkg} ({arch}, engine {ASSET_VERSION})"),
        (None, None) => format!("speclink {pkg} (engine {ASSET_VERSION})"),
    };
    assert_eq!(stdout.trim_end(), expected, "版號格式是對外契約");
}
