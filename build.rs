#[cfg(windows)]
use std::error::Error;
#[cfg(windows)]
use std::fs;
#[cfg(windows)]
use std::path::Path;

#[cfg(windows)]
fn write_generated_icon(path: &Path) -> Result<(), Box<dyn Error>> {
    let width: usize = 16;
    let height: usize = 16;

    let dib_header_size: u32 = 40;
    let pixel_bytes: u32 = (width * height * 4) as u32;
    let mask_row_bytes: usize = ((width + 31) / 32) * 4;
    let mask_bytes: u32 = (mask_row_bytes * height) as u32;
    let image_size: u32 = dib_header_size + pixel_bytes + mask_bytes;

    let mut bytes: Vec<u8> = Vec::with_capacity((6 + 16 + image_size) as usize);

    // ICONDIR
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());

    // ICONDIRENTRY
    bytes.push(width as u8);
    bytes.push(height as u8);
    bytes.push(0);
    bytes.push(0);
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(&image_size.to_le_bytes());
    bytes.extend_from_slice(&22u32.to_le_bytes());

    // BITMAPINFOHEADER
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&(width as i32).to_le_bytes());
    bytes.extend_from_slice(&((height as i32) * 2).to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&(pixel_bytes + mask_bytes).to_le_bytes());
    bytes.extend_from_slice(&0i32.to_le_bytes());
    bytes.extend_from_slice(&0i32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());

    // Pixel data (BGRA, bottom-up): yellow/cyan checker.
    for y in (0..height).rev() {
        for x in 0..width {
            let checker = ((x / 4) + (y / 4)) % 2 == 0;
            let (r, g, b) = if checker { (240u8, 235u8, 45u8) } else { (0u8, 190u8, 210u8) };
            bytes.push(b);
            bytes.push(g);
            bytes.push(r);
            bytes.push(255u8);
        }
    }

    // AND mask: all zero = fully opaque.
    bytes.extend(std::iter::repeat_n(0u8, mask_row_bytes * height));

    fs::write(path, bytes)?;
    Ok(())
}

#[cfg(windows)]
fn main() -> Result<(), Box<dyn Error>> {
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env != "msvc" {
        println!(
            "cargo:warning=Skipping icon embedding for target_env={} (requires msvc tools)",
            target_env
        );
        println!("cargo:rerun-if-changed=build.rs");
        return Ok(());
    }

    let out_dir = std::env::var("OUT_DIR")?;
    let icon_path = Path::new(&out_dir).join("app.ico");

    write_generated_icon(&icon_path)?;

    let mut res = winres::WindowsResource::new();
    res.set_icon(icon_path.to_string_lossy().as_ref());
    res.set("FileDescription", "Screen Saver Blocker");
    res.set("ProductName", "Screen Saver Blocker");
    res.set("OriginalFilename", "screen-saver-blocker-rust.exe");
    res.compile()?;

    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}

#[cfg(not(windows))]
fn main() {}
