//! Hidden metadata: find it, explain it, remove it.
//!
//! JPEG and PNG are cleaned losslessly: we drop the metadata segments/chunks
//! and copy the compressed image data byte-for-byte, so quality never changes.
//! The one piece of EXIF we keep is orientation, written into a minimal new
//! EXIF block, otherwise phone photos would suddenly appear rotated.

use serde::Serialize;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Clone)]
pub struct MetaItem {
    /// Machine key the UI translates: gps, camera, lens, taken, software,
    /// author, copyright, serial, comment, xmp, iptc, png_text, c2pa.
    pub key: &'static str,
    pub value: String,
    /// 3 = reveals where you are / who you are, 2 = identifies the device, 1 = minor.
    pub risk: u8,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub path: String,
    pub name: String,
    pub format: String,
    pub size: u64,
    pub items: Vec<MetaItem>,
    pub gps: Option<[f64; 2]>,
    pub can_clean: bool,
    pub can_redact: bool,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CleanResult {
    pub path: String,
    pub out: Option<String>,
    pub removed_bytes: i64,
    pub error: Option<String>,
}

pub fn format_of(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

fn sniff(data: &[u8]) -> &'static str {
    if data.starts_with(&[0xFF, 0xD8]) {
        "jpeg"
    } else if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        "png"
    } else {
        "other"
    }
}

pub fn inspect(path: &Path) -> Report {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let ext = format_of(path);
    let mut r = Report { path: path.display().to_string(), name, format: ext.clone(), size, items: vec![], gps: None, can_clean: false, can_redact: false, note: None };
    let Ok(data) = fs::read(path) else {
        r.note = Some("unreadable".into());
        return r;
    };
    let kind = sniff(&data);
    r.can_redact = kind != "other" || matches!(ext.as_str(), "heic" | "heif" | "webp" | "tif" | "tiff" | "gif" | "bmp");
    r.can_clean = kind != "other" || (cfg!(target_os = "macos") && matches!(ext.as_str(), "heic" | "heif" | "webp" | "tif" | "tiff"));
    if !r.can_clean {
        r.note = Some("unsupported".into());
    }
    read_exif(path, &mut r);
    match kind {
        "jpeg" => jpeg_extras(&data, &mut r.items),
        "png" => png_extras(&data, &mut r.items),
        _ => {}
    }
    r.items.sort_by(|a, b| b.risk.cmp(&a.risk));
    r
}

fn read_exif(path: &Path, r: &mut Report) {
    let Ok(file) = fs::File::open(path) else { return };
    let Ok(ex) = exif::Reader::new().read_from_container(&mut BufReader::new(file)) else { return };
    use exif::{In, Tag};
    let get = |t: Tag| ex.get_field(t, In::PRIMARY).map(|f| f.display_value().with_unit(&ex).to_string().trim_matches('"').trim().to_string()).filter(|s| !s.is_empty());

    if let (Some(lat), Some(lon)) = (gps_coord(&ex, Tag::GPSLatitude, Tag::GPSLatitudeRef), gps_coord(&ex, Tag::GPSLongitude, Tag::GPSLongitudeRef)) {
        r.gps = Some([lat, lon]);
        r.items.push(MetaItem { key: "gps", value: format!("{lat:.5}, {lon:.5}"), risk: 3 });
    }
    let camera = [get(Tag::Make), get(Tag::Model)].into_iter().flatten().collect::<Vec<_>>().join(" ");
    if !camera.is_empty() {
        r.items.push(MetaItem { key: "camera", value: camera, risk: 2 });
    }
    let push = |r: &mut Report, key, tag, risk| {
        if let Some(v) = get(tag) {
            r.items.push(MetaItem { key, value: v, risk });
        }
    };
    push(r, "lens", Tag::LensModel, 1);
    push(r, "taken", Tag::DateTimeOriginal, 2);
    push(r, "software", Tag::Software, 1);
    push(r, "author", Tag::Artist, 3);
    push(r, "author", Tag::CameraOwnerName, 3);
    push(r, "copyright", Tag::Copyright, 2);
    push(r, "serial", Tag::BodySerialNumber, 2);
    push(r, "serial", Tag::ImageUniqueID, 2);
    push(r, "comment", Tag::UserComment, 2);
    push(r, "comment", Tag::ImageDescription, 1);
}

fn gps_coord(ex: &exif::Exif, tag: exif::Tag, reftag: exif::Tag) -> Option<f64> {
    let f = ex.get_field(tag, exif::In::PRIMARY)?;
    let exif::Value::Rational(ref v) = f.value else { return None };
    if v.len() < 3 {
        return None;
    }
    let mut deg = v[0].to_f64() + v[1].to_f64() / 60.0 + v[2].to_f64() / 3600.0;
    if !deg.is_finite() || deg == 0.0 {
        return None;
    }
    if let Some(r) = ex.get_field(reftag, exif::In::PRIMARY) {
        let s = r.display_value().to_string();
        if s.contains('S') || s.contains('W') {
            deg = -deg;
        }
    }
    Some(deg)
}

// ------------------------------------------------------------------ JPEG

struct Segment<'a> {
    marker: u8,
    bytes: &'a [u8], // including the 0xFF marker and length
}

/// Splits a JPEG into header segments and the entropy-coded tail (from SOS on).
fn jpeg_segments(data: &[u8]) -> Result<(Vec<Segment<'_>>, &[u8]), String> {
    if !data.starts_with(&[0xFF, 0xD8]) {
        return Err("not a JPEG".into());
    }
    let mut segs = Vec::new();
    let mut i = 2;
    loop {
        if i + 4 > data.len() {
            return Err("truncated JPEG".into());
        }
        if data[i] != 0xFF {
            return Err("corrupt JPEG marker".into());
        }
        let mut j = i + 1;
        while j < data.len() && data[j] == 0xFF {
            j += 1; // fill bytes
        }
        let marker = *data.get(j).ok_or("truncated JPEG")?;
        if marker == 0xDA || marker == 0xD9 {
            return Ok((segs, &data[i..]));
        }
        if (0xD0..=0xD7).contains(&marker) || marker == 0x01 {
            segs.push(Segment { marker, bytes: &data[i..j + 1] });
            i = j + 1;
            continue;
        }
        let len = u16::from_be_bytes([*data.get(j + 1).ok_or("truncated")?, *data.get(j + 2).ok_or("truncated")?]) as usize;
        let end = j + 1 + len;
        if end > data.len() || len < 2 {
            return Err("corrupt JPEG segment".into());
        }
        segs.push(Segment { marker, bytes: &data[i..end] });
        i = end;
    }
}

fn payload<'a>(s: &Segment<'a>) -> &'a [u8] {
    let start = s.bytes.iter().position(|&b| b != 0xFF).unwrap_or(0) + 3;
    s.bytes.get(start..).unwrap_or(&[])
}

fn jpeg_extras(data: &[u8], items: &mut Vec<MetaItem>) {
    let Ok((segs, _)) = jpeg_segments(data) else { return };
    for s in &segs {
        let p = payload(s);
        match s.marker {
            0xE1 if p.starts_with(b"http://ns.adobe.com/xap/1.0/") => items.push(MetaItem { key: "xmp", value: format!("{} B", p.len()), risk: 2 }),
            0xED => items.push(MetaItem { key: "iptc", value: format!("{} B", p.len()), risk: 2 }),
            0xFE => items.push(MetaItem { key: "comment", value: String::from_utf8_lossy(&p[..p.len().min(80)]).trim().to_string(), risk: 2 }),
            0xEB => items.push(MetaItem { key: "c2pa", value: format!("{} B", p.len()), risk: 1 }),
            _ => {}
        }
    }
}

fn jpeg_orientation(data: &[u8]) -> u16 {
    let mut cur = std::io::Cursor::new(data);
    exif::Reader::new()
        .read_from_container(&mut cur)
        .ok()
        .and_then(|e| e.get_field(exif::Tag::Orientation, exif::In::PRIMARY).and_then(|f| f.value.get_uint(0)))
        .map(|v| v as u16)
        .unwrap_or(1)
}

/// Minimal EXIF APP1 that carries only the Orientation tag.
fn orientation_app1(orientation: u16) -> Vec<u8> {
    let mut tiff = Vec::new();
    tiff.extend_from_slice(b"MM\x00\x2A\x00\x00\x00\x08"); // big-endian TIFF, IFD at 8
    tiff.extend_from_slice(&1u16.to_be_bytes()); // one entry
    tiff.extend_from_slice(&0x0112u16.to_be_bytes()); // Orientation
    tiff.extend_from_slice(&3u16.to_be_bytes()); // SHORT
    tiff.extend_from_slice(&1u32.to_be_bytes()); // count
    tiff.extend_from_slice(&orientation.to_be_bytes());
    tiff.extend_from_slice(&[0, 0]);
    tiff.extend_from_slice(&0u32.to_be_bytes()); // no next IFD
    let mut seg = vec![0xFF, 0xE1];
    seg.extend_from_slice(&((2 + 6 + tiff.len()) as u16).to_be_bytes());
    seg.extend_from_slice(b"Exif\x00\x00");
    seg.extend_from_slice(&tiff);
    seg
}

pub fn strip_jpeg(data: &[u8]) -> Result<Vec<u8>, String> {
    let orientation = jpeg_orientation(data);
    let (segs, tail) = jpeg_segments(data)?;
    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(&[0xFF, 0xD8]);
    let mut wrote_orientation = false;
    for s in &segs {
        let p = payload(s);
        let keep = match s.marker {
            0xE0 => p.starts_with(b"JFIF\0") || p.starts_with(b"JFXX\0"),
            0xE2 => p.starts_with(b"ICC_PROFILE\0"), // colour profile: keeps colours correct
            0xEE => p.starts_with(b"Adobe"),          // colour transform flag, needed to decode
            0xE1..=0xEF | 0xFE => false,             // EXIF, XMP, IPTC, C2PA, comments, vendor blobs
            _ => true,
        };
        if !wrote_orientation && orientation > 1 && s.marker != 0xE0 {
            out.extend(orientation_app1(orientation));
            wrote_orientation = true;
        }
        if keep {
            out.extend_from_slice(s.bytes);
        }
    }
    if !wrote_orientation && orientation > 1 {
        out.extend(orientation_app1(orientation));
    }
    out.extend_from_slice(&tail[..main_image_end(tail)]);
    Ok(out)
}

/// Length of the primary image inside `tail` (which starts at the first SOS),
/// up to and including its EOI. Phones append extra images after it (HDR gain
/// maps, depth maps) that carry their own metadata, so we stop there.
fn main_image_end(tail: &[u8]) -> usize {
    let mut i = 0;
    while i + 1 < tail.len() {
        if tail[i] != 0xFF {
            i += 1;
            continue;
        }
        let m = tail[i + 1];
        match m {
            0xD9 => return i + 2,
            0x00 | 0xFF | 0xD0..=0xD7 => i += if m == 0xFF { 1 } else { 2 },
            _ => {
                // A header segment between scans (SOS, DHT, DQT... in
                // progressive files): skip it by its length.
                let Some(len) = tail.get(i + 2..i + 4).map(|b| u16::from_be_bytes([b[0], b[1]]) as usize) else { return tail.len() };
                i += 2 + len;
            }
        }
    }
    tail.len()
}

// ------------------------------------------------------------------ PNG

const PNG_DROP: [&[u8; 4]; 6] = [b"tEXt", b"zTXt", b"iTXt", b"eXIf", b"tIME", b"caBX"];

fn png_chunks(data: &[u8]) -> Result<Vec<(&[u8], &[u8])>, String> {
    if !data.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("not a PNG".into());
    }
    let mut out = Vec::new();
    let mut i = 8;
    while i + 12 <= data.len() {
        let len = u32::from_be_bytes(data[i..i + 4].try_into().unwrap()) as usize;
        let end = i + 12 + len;
        if end > data.len() {
            return Err("truncated PNG".into());
        }
        let ty = &data[i + 4..i + 8];
        out.push((ty, &data[i..end]));
        i = end;
        if ty == b"IEND" {
            break;
        }
    }
    Ok(out)
}

fn png_extras(data: &[u8], items: &mut Vec<MetaItem>) {
    let Ok(chunks) = png_chunks(data) else { return };
    for (ty, bytes) in chunks {
        let body = &bytes[8..bytes.len() - 4];
        match ty {
            b"tEXt" | b"iTXt" | b"zTXt" => {
                let key_end = body.iter().position(|&b| b == 0).unwrap_or(body.len());
                let key = String::from_utf8_lossy(&body[..key_end]).to_string();
                let mut value = if ty == b"tEXt" { String::from_utf8_lossy(&body[(key_end + 1).min(body.len())..]).to_string() } else { format!("{} B", body.len()) };
                if value.chars().count() > 90 {
                    value = value.chars().take(90).collect::<String>() + "…";
                }
                // AI image tools store the full prompt here ("parameters", "prompt", "workflow").
                let risk = if matches!(key.to_lowercase().as_str(), "parameters" | "prompt" | "workflow" | "author" | "comment") { 3 } else { 1 };
                items.push(MetaItem { key: "png_text", value: format!("{key}: {value}"), risk });
            }
            b"eXIf" => {}
            b"caBX" => items.push(MetaItem { key: "c2pa", value: format!("{} B", body.len()), risk: 1 }),
            b"tIME" => items.push(MetaItem { key: "taken", value: "PNG tIME".into(), risk: 1 }),
            _ => {}
        }
    }
}

pub fn strip_png(data: &[u8]) -> Result<Vec<u8>, String> {
    let chunks = png_chunks(data)?;
    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(&data[..8]);
    for (ty, bytes) in chunks {
        if !PNG_DROP.iter().any(|d| d.as_slice() == ty) {
            out.extend_from_slice(bytes);
        }
    }
    Ok(out)
}

// ------------------------------------------------------------------ files

/// "photo.jpg" -> "photo.<tag>.<ext>" next to the original, or in `dir` when
/// given, adding "-2", "-3"... rather than overwriting.
pub fn sibling(path: &Path, dir: Option<&Path>, tag: &str, ext: &str) -> PathBuf {
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
    let dir = dir.or(path.parent()).unwrap_or(Path::new("."));
    let tag = if tag.trim().is_empty() { "clean" } else { tag.trim() };
    let mut candidate = dir.join(format!("{stem}.{tag}.{ext}"));
    let mut n = 2;
    while candidate.exists() {
        candidate = dir.join(format!("{stem}.{tag}-{n}.{ext}"));
        n += 1;
    }
    candidate
}

pub fn clean(path: &Path) -> CleanResult {
    clean_with(path, None, "clean")
}

/// Like `clean`, but writes into `dir` (if given) with a custom name tag.
pub fn clean_with(path: &Path, dir: Option<&Path>, tag: &str) -> CleanResult {
    let mut res = CleanResult { path: path.display().to_string(), out: None, removed_bytes: 0, error: None };
    match clean_inner(path, dir, tag) {
        Ok((out, removed)) => {
            res.out = Some(out.display().to_string());
            res.removed_bytes = removed;
        }
        Err(e) => res.error = Some(e),
    }
    res
}

fn clean_inner(path: &Path, dir: Option<&Path>, tag: &str) -> Result<(PathBuf, i64), String> {
    let data = fs::read(path).map_err(|e| e.to_string())?;
    let (bytes, ext) = match sniff(&data) {
        "jpeg" => (strip_jpeg(&data)?, if format_of(path) == "jpeg" { "jpeg" } else { "jpg" }),
        "png" => (strip_png(&data)?, "png"),
        _ => (convert_then_strip(path)?, "jpg"),
    };
    let out = sibling(path, dir, tag, ext);
    fs::write(&out, &bytes).map_err(|e| e.to_string())?;
    Ok((out, data.len() as i64 - bytes.len() as i64))
}

#[cfg(target_os = "macos")]
fn convert_then_strip(path: &Path) -> Result<Vec<u8>, String> {
    // HEIC/WebP/TIFF: let macOS's own `sips` decode it, then strip the JPEG.
    let tmp = std::env::temp_dir().join(format!("peekless-{}.jpg", std::process::id()));
    let status = std::process::Command::new("/usr/bin/sips")
        .args(["-s", "format", "jpeg", "-s", "formatOptions", "best"])
        .arg(path)
        .arg("--out")
        .arg(&tmp)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    let data = fs::read(&tmp);
    let _ = fs::remove_file(&tmp);
    if !status.success() {
        return Err("unsupported".into());
    }
    strip_jpeg(&data.map_err(|e| e.to_string())?)
}

#[cfg(not(target_os = "macos"))]
fn convert_then_strip(_path: &Path) -> Result<Vec<u8>, String> {
    Err("unsupported".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg_with_exif() -> Vec<u8> {
        let img = image::RgbImage::from_pixel(8, 8, image::Rgb([200, 30, 30]));
        let mut plain = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut plain).encode_image(&img).unwrap();
        // Splice in an APP1 EXIF with orientation 6 plus a fake GPS-ish blob,
        // an XMP packet and a comment right after SOI.
        let mut out = plain[..2].to_vec();
        out.extend(orientation_app1(6));
        let xmp = b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta>secret creator</x:xmpmeta>";
        out.extend([0xFF, 0xE1]);
        out.extend(((xmp.len() + 2) as u16).to_be_bytes());
        out.extend(xmp);
        out.extend([0xFF, 0xFE, 0x00, 0x0A]);
        out.extend(b"hello!!!");
        out.extend(&plain[2..]);
        out
    }

    #[test]
    fn jpeg_strip_is_lossless_and_keeps_orientation() {
        let src = jpeg_with_exif();
        let out = strip_jpeg(&src).unwrap();
        assert!(!out.windows(6).any(|w| w == b"secret"), "XMP survived");
        assert!(!out.windows(8).any(|w| w == b"hello!!!"), "comment survived");
        assert_eq!(jpeg_orientation(&out), 6);
        let (_, tail_a) = jpeg_segments(&src).unwrap();
        let (_, tail_b) = jpeg_segments(&out).unwrap();
        assert_eq!(tail_a, tail_b, "compressed image data must be untouched");
        image::load_from_memory(&out).expect("output must still decode");
    }

    #[test]
    fn jpeg_strip_drops_appended_images() {
        let mut src = jpeg_with_exif();
        let main_len = src.len();
        src.extend(jpeg_with_exif()); // e.g. an HDR gain map with its own metadata
        let out = strip_jpeg(&src).unwrap();
        assert!(out.len() < main_len, "appended image must be cut off");
        assert!(out.ends_with(&[0xFF, 0xD9]));
        image::load_from_memory(&out).expect("output must still decode");
    }

    #[test]
    fn png_strip_drops_text_chunks() {
        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([1, 2, 3, 255]));
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        // insert a tEXt chunk after IHDR (8 + 25 bytes)
        let body = b"parameters\0a cat, by my secret prompt";
        let mut chunk = (body.len() as u32).to_be_bytes().to_vec();
        chunk.extend(b"tEXt");
        chunk.extend(body);
        chunk.extend([0, 0, 0, 0]);
        let mut with = png[..33].to_vec();
        with.extend(&chunk);
        with.extend(&png[33..]);
        let mut items = vec![];
        png_extras(&with, &mut items);
        assert_eq!(items[0].key, "png_text");
        assert_eq!(items[0].risk, 3);
        let out = strip_png(&with).unwrap();
        assert_eq!(out, png);
    }

    #[test]
    fn clean_with_honours_folder_and_tag() {
        let dir = std::env::temp_dir().join(format!("peekless-out-{}", std::process::id()));
        let out_dir = dir.join("out");
        fs::create_dir_all(&out_dir).unwrap();
        let src = dir.join("photo.jpg");
        fs::write(&src, jpeg_with_exif()).unwrap();
        let r = clean_with(&src, Some(&out_dir), "safe");
        let out = PathBuf::from(r.out.expect("cleaned"));
        assert_eq!(out, out_dir.join("photo.safe.jpg"));
        assert!(r.removed_bytes > 0);
        assert_eq!(fs::read(&src).unwrap(), jpeg_with_exif(), "original untouched");
        let again = clean_with(&src, Some(&out_dir), "safe");
        assert_eq!(PathBuf::from(again.out.unwrap()), out_dir.join("photo.safe-2.jpg"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn clean_names_never_overwrite() {
        let dir = std::env::temp_dir().join(format!("peekless-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.jpg");
        assert_eq!(sibling(&p, None, "clean", "jpg").file_name().unwrap(), "a.clean.jpg");
        fs::write(dir.join("a.clean.jpg"), b"x").unwrap();
        assert_eq!(sibling(&p, None, "clean", "jpg").file_name().unwrap(), "a.clean-2.jpg");
        fs::remove_dir_all(dir).unwrap();
    }
}
