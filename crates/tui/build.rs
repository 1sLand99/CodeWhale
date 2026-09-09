use std::fmt::Write;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    codewhale_build_support::declare_rerun_conditions(&manifest_dir);
    generate_localization(&manifest_dir);
    codewhale_build_support::configure_windows_main_stack("codewhale-tui");
    build_computer_use_helper(&manifest_dir);
    codewhale_build_support::emit_build_version(&manifest_dir, env!("CARGO_PKG_VERSION"));
}

fn generate_localization(manifest_dir: &Path) {
    let locales = manifest_dir.join("locales");
    println!("cargo:rerun-if-changed={}", locales.display());
    // Use the same loader as rust-i18n's macro, including flattening and
    // locale merging. Store entries in static data rather than emitting one
    // map.insert statement per translation into a huge debug-stack frame.
    let translations = rust_i18n_support::try_load_locales(
        locales.to_str().expect("UTF-8 locale path"),
        |_| false,
        true,
    )
    .expect("valid translation catalog");
    assert!(!translations.is_empty(), "translation catalog is empty");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let mut data = String::from("static LOCALES: &[(&str, Messages)] = &[\n");
    for (locale, entries) in translations {
        writeln!(data, "({locale:?}, &[").unwrap();
        for (key, value) in entries {
            writeln!(data, "({key:?}, {value:?}),").unwrap();
        }
        data.push_str("]),\n");
    }
    data.push_str("];\n");
    std::fs::write(out.join("i18n_data.rs"), data).expect("write translation data");

    // Keep the library's translation/fallback macros and extension API, but
    // let our loop populate its SimpleBackend. Its built-in table is empty.
    let bootstrap = out.join("i18n_bootstrap");
    std::fs::create_dir_all(&bootstrap).expect("create i18n bootstrap directory");
    std::fs::write(
        out.join("i18n_init.rs"),
        format!(
            "i18n!({:?}, fallback = [\"en\"], backend = crate::localization_backend::new());\n",
            bootstrap.to_str().expect("UTF-8 build path")
        ),
    )
    .expect("write i18n bootstrap");
}

/// Ship native computer-use support with macOS binaries. Requiring clang on
/// the customer's machine would make the built-in plugin a source-only demo.
fn build_computer_use_helper(manifest_dir: &std::path::Path) {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let source = manifest_dir.join("plugins/computer-use/src/backends/darwin-accessibility.m");
    println!("cargo:rerun-if-changed={}", source.display());
    println!(
        "cargo:rerun-if-changed={}",
        source.with_file_name("darwin-recording.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        source.with_file_name("darwin-ocr.h").display()
    );
    println!("cargo:rerun-if-env-changed=CODEWHALE_CU_SIGN_IDENTITY");
    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        other => panic!("unsupported macOS computer-use architecture: {other:?}"),
    };
    let output = PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"))
        .join("computer-use-accessibility");
    let compiled = std::process::Command::new("xcrun")
        .args([
            "clang",
            "-fobjc-arc",
            "-Os",
            "-arch",
            arch,
            "-mmacosx-version-min=13.0",
            "-framework",
            "Cocoa",
            "-framework",
            "ApplicationServices",
            "-framework",
            "ScreenCaptureKit",
            "-framework",
            "AVFoundation",
            "-framework",
            "CoreMedia",
            "-framework",
            "Vision",
        ])
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .output()
        .expect("macOS builds require Xcode Command Line Tools to package Computer Use");
    assert!(
        compiled.status.success(),
        "Computer Use helper compilation failed: {}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    // Developer builds use ad-hoc signing; release builders can supply the
    // same Developer ID as the app. No keychain lookup or credential copying.
    let identity = std::env::var("CODEWHALE_CU_SIGN_IDENTITY").unwrap_or_else(|_| "-".into());
    let signed = std::process::Command::new("codesign")
        .args([
            "--force",
            if identity == "-" {
                "--timestamp=none"
            } else {
                "--timestamp"
            },
            "--options",
            "runtime",
            "--identifier",
            "net.codewhale.computer-use.helper",
            "--sign",
        ])
        .arg(&identity)
        .arg(&output)
        .output()
        .expect("macOS builds require codesign to package Computer Use");
    assert!(
        signed.status.success(),
        "Computer Use helper signing failed: {}",
        String::from_utf8_lossy(&signed.stderr)
    );
}
