//! Build script for ai-memory-web.
//!
//! Downloads the standalone Tailwind CSS CLI (pinned version) and
//! compiles `static/input.css` → `OUT_DIR/tailwind.css`.
//!
//! # Escape hatch
//! Set `TAILWIND_SKIP=1` to skip the download entirely and use the
//! vendored `static/tailwind.css` instead.
//!
//! # Incremental builds
//! Also skips the download when `static/tailwind.css` is newer than
//! every template file and `static/input.css`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

const TAILWIND_VERSION: &str = "3.4.17";

fn main() {
    // Re-run triggers.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=static/input.css");
    println!("cargo:rerun-if-changed=templates/");
    println!("cargo:rerun-if-env-changed=TAILWIND_SKIP");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let out_css = out_dir.join("tailwind.css");
    let crate_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    // Escape hatch: TAILWIND_SKIP=1 → use the vendored file.
    if std::env::var("TAILWIND_SKIP").as_deref() == Ok("1") {
        let src = crate_dir.join("static/tailwind.css");
        std::fs::copy(&src, &out_css)
            .unwrap_or_else(|e| panic!("TAILWIND_SKIP=1 but static/tailwind.css missing: {e}"));
        emit_env(&out_css);
        return;
    }

    // Incremental: skip if static/tailwind.css is newer than all sources.
    let vendored = crate_dir.join("static/tailwind.css");
    if is_vendored_fresh(&vendored, &crate_dir) {
        std::fs::copy(&vendored, &out_css)
            .expect("failed to copy vendored tailwind.css to OUT_DIR");
        emit_env(&out_css);
        return;
    }

    // Download the tailwind binary (cached by version in OUT_DIR's parent).
    let binary = download_tailwind(&out_dir);

    // Run tailwind.
    compile_tailwind(&binary, &crate_dir, &out_css);
    // Also update the vendored copy so the next build is incremental.
    let _ = std::fs::copy(&out_css, &vendored);

    emit_env(&out_css);
}

fn emit_env(css_path: &Path) {
    println!(
        "cargo:rustc-env=AI_MEMORY_WEB_TAILWIND_CSS={}",
        css_path.display()
    );
}

/// True when `static/tailwind.css` is newer than all template files
/// and `static/input.css`.
fn is_vendored_fresh(vendored: &Path, crate_dir: &Path) -> bool {
    let Ok(vmt) = mtime(vendored) else {
        return false;
    };
    // Check input.css.
    let input = crate_dir.join("static/input.css");
    if mtime(&input).map(|t| t > vmt).unwrap_or(false) {
        return false;
    }
    // Walk templates/.
    let tmpl_dir = crate_dir.join("templates");
    is_dir_older_than(&tmpl_dir, vmt)
}

fn is_dir_older_than(dir: &Path, threshold: SystemTime) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return true; // no templates → consider fresh
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if !is_dir_older_than(&path, threshold) {
                return false;
            }
        } else if mtime(&path).map(|t| t > threshold).unwrap_or(false) {
            return false;
        }
    }
    true
}

fn mtime(p: &Path) -> std::io::Result<SystemTime> {
    std::fs::metadata(p)?.modified()
}

/// Platform-specific download URL for the Tailwind CLI binary.
fn tailwind_url() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let slug = match (os, arch) {
        ("linux", "x86_64") => "tailwindcss-linux-x64",
        ("linux", "aarch64") => "tailwindcss-linux-arm64",
        ("macos", "x86_64") => "tailwindcss-macos-x64",
        ("macos", "aarch64") => "tailwindcss-macos-arm64",
        ("windows", "x86_64") => "tailwindcss-windows-x64.exe",
        _ => panic!(
            "Unsupported platform {os}/{arch} — set TAILWIND_SKIP=1 and provide static/tailwind.css manually"
        ),
    };
    format!(
        "https://github.com/tailwindlabs/tailwindcss/releases/download/v{TAILWIND_VERSION}/{slug}"
    )
}

/// Download the Tailwind CLI and return the path to the executable.
/// The file is cached in `OUT_DIR/tailwindcss-{version}` across rebuilds.
fn download_tailwind(out_dir: &Path) -> PathBuf {
    let bin_name = if cfg!(windows) {
        format!("tailwindcss-{TAILWIND_VERSION}.exe")
    } else {
        format!("tailwindcss-{TAILWIND_VERSION}")
    };
    // Cache in the grandparent of OUT_DIR so it survives incremental rebuilds
    // across profile changes. Fall back to OUT_DIR itself if that fails.
    let cache_dir = out_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .unwrap_or(out_dir);
    let dest = cache_dir.join(&bin_name);

    if dest.exists() {
        // Already cached.
        return dest;
    }

    let url = tailwind_url();
    eprintln!("cargo:warning=Downloading Tailwind CSS CLI v{TAILWIND_VERSION} from {url}");

    // Try curl first, then wget.
    let success = try_download_curl(&url, &dest) || try_download_wget(&url, &dest);

    if !success {
        panic!(
            "Could not download Tailwind CSS CLI — curl and wget both failed.\n\
             Either install curl or wget, OR set TAILWIND_SKIP=1 and place a compiled \
             tailwind.css in crates/ai-memory-web/static/tailwind.css."
        );
    }

    // Make executable on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms).unwrap();
    }

    dest
}

fn try_download_curl(url: &str, dest: &Path) -> bool {
    Command::new("curl")
        .args(["--fail", "--silent", "--location", "--output"])
        .arg(dest)
        .arg(url)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn try_download_wget(url: &str, dest: &Path) -> bool {
    Command::new("wget")
        .args(["--quiet", "--output-document"])
        .arg(dest)
        .arg(url)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Invoke the Tailwind CLI to produce `out_css`.
fn compile_tailwind(binary: &Path, crate_dir: &Path, out_css: &Path) {
    let input_css = crate_dir.join("static/input.css");
    let config_js = crate_dir.join("tailwind.config.js");

    let status = Command::new(binary)
        .current_dir(crate_dir)
        .args([
            "--input",
            input_css.to_str().unwrap(),
            "--output",
            out_css.to_str().unwrap(),
            "--config",
            config_js.to_str().unwrap(),
            "--minify",
        ])
        .status()
        .unwrap_or_else(|e| panic!("Failed to run tailwind CLI: {e}"));

    if !status.success() {
        panic!("tailwind CSS compilation failed (exit status {status:?})");
    }
}
