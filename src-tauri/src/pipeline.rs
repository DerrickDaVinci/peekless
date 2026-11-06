//! Glue shared by the app and the CLI: load images, run OCR + detection,
//! talk to the clipboard.

use crate::{ocr, redact};
use image::{DynamicImage, ImageDecoder, ImageReader, RgbaImage};
use std::path::Path;

pub struct Scanned {
    pub findings: Vec<redact::ImageFinding>,
    pub lines: usize,
    pub text: Vec<String>,
}

pub fn encode_png(img: &RgbaImage, fast: bool) -> Result<Vec<u8>, String> {
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    use image::ImageEncoder;
    let mut buf = Vec::new();
    let comp = if fast { CompressionType::Fast } else { CompressionType::Default };
    PngEncoder::new_with_quality(&mut buf, comp, FilterType::Adaptive)
        .write_image(img.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())?;
    Ok(buf)
}

pub fn scan_image(img: &RgbaImage) -> Result<Scanned, String> {
    let png = encode_png(img, true)?;
    let lines = ocr::recognize_png(&png)?;
    let findings = redact::findings_from_ocr(&lines, img.width(), img.height());
    Ok(Scanned { findings, lines: lines.len(), text: lines.into_iter().map(|l| l.text).collect() })
}

/// Loads any image we can read, with EXIF orientation applied so boxes land
/// where the user sees the text.
pub fn load_image(path: &Path) -> Result<RgbaImage, String> {
    match load_with_image_crate(path) {
        Ok(img) => Ok(img),
        Err(e) => load_via_os(path).ok_or(e),
    }
}

fn load_with_image_crate(path: &Path) -> Result<RgbaImage, String> {
    let mut decoder = ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .into_decoder()
        .map_err(|_| "unsupported image format".to_string())?;
    let orientation = decoder.orientation().map_err(|e| e.to_string())?;
    let mut img = DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    img.apply_orientation(orientation);
    Ok(img.to_rgba8())
}

#[cfg(target_os = "macos")]
fn load_via_os(path: &Path) -> Option<RgbaImage> {
    // HEIC, WebP, TIFF, GIF...: let macOS decode it to PNG for us.
    let tmp = std::env::temp_dir().join(format!("peekless-load-{}.png", std::process::id()));
    let ok = std::process::Command::new("/usr/bin/sips")
        .args(["-s", "format", "png"])
        .arg(path)
        .arg("--out")
        .arg(&tmp)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()?
        .success();
    let img = if ok { image::open(&tmp).ok().map(|i| i.to_rgba8()) } else { None };
    let _ = std::fs::remove_file(&tmp);
    img
}

#[cfg(not(target_os = "macos"))]
fn load_via_os(_path: &Path) -> Option<RgbaImage> {
    None
}

pub enum Clip {
    Image(RgbaImage),
    Text(String),
    Empty,
}

pub fn read_clipboard() -> Result<Clip, String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    if let Ok(img) = cb.get_image() {
        if let Some(rgba) = RgbaImage::from_raw(img.width as u32, img.height as u32, img.bytes.into_owned()) {
            return Ok(Clip::Image(rgba));
        }
    }
    match cb.get_text() {
        Ok(t) if !t.trim().is_empty() => Ok(Clip::Text(t)),
        _ => Ok(Clip::Empty),
    }
}

pub fn write_image(img: &RgbaImage) -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_image(arboard::ImageData { width: img.width() as usize, height: img.height() as usize, bytes: img.as_raw().into() })
        .map_err(|e| e.to_string())?;
    crate::guard::mark_own_write();
    Ok(())
}

pub fn write_text(text: &str) -> Result<(), String> {
    arboard::Clipboard::new().and_then(|mut c| c.set_text(text)).map_err(|e| e.to_string())?;
    crate::guard::mark_own_write();
    Ok(())
}
