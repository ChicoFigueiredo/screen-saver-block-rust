#[cfg(windows)]
use std::error::Error;
#[cfg(windows)]
use std::path::Path;

#[cfg(windows)]
fn main() -> Result<(), Box<dyn Error>> {
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env != "msvc" {
        println!(
            "cargo:warning=Skipping icon embedding for target_env={} (requires msvc tools)",
            target_env
        );
        println!("cargo:rerun-if-changed=assets/preferences-desktop-screensaver.ico");
        println!("cargo:rerun-if-changed=build.rs");
        return Ok(());
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let icon_path = Path::new(&manifest_dir).join("assets/preferences-desktop-screensaver.ico");
    if !icon_path.exists() {
        return Err(format!("Icon not found at {}", icon_path.display()).into());
    }

    let mut res = winres::WindowsResource::new();
    res.set_icon(icon_path.to_string_lossy().as_ref());
    res.set("FileDescription", "Screen Saver Blocker");
    res.set("ProductName", "Screen Saver Blocker");
    res.set("OriginalFilename", "screen-saver-blocker-rust.exe");
    res.compile()?;

    println!("cargo:rerun-if-changed=assets/preferences-desktop-screensaver.ico");
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}

#[cfg(not(windows))]
fn main() {}
