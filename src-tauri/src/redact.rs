//! Burning redactions into pixels, and turning OCR matches into boxes.

use crate::detect::{self, Finding};
use crate::ocr::OcrLine;
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    /// Opaque bar. The only style that is safe against reconstruction.
    Solid,
    /// Mosaic. Looks softer but low-res text can sometimes be recovered.
    Pixelate,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageFinding {
    pub kind: &'static str,
    pub preview: String,
    pub severity: u8,
    /// [x, y, w, h] in image pixels.
    pub boxes: Vec<[f32; 4]>,
}

/// Run detection on every OCR line and convert char spans into padded boxes.
pub fn findings_from_ocr(lines: &[OcrLine], width: u32, height: u32) -> Vec<ImageFinding> {
    let mut out = Vec::new();
    for line in lines {
        for f in detect::scan(&line.text) {
            if let Some(b) = span_box(line, &f, width, height) {
                out.push(ImageFinding { kind: f.kind, preview: f.preview, severity: f.severity, boxes: vec![b] });
            }
        }
    }
    out
}

fn span_box(line: &OcrLine, f: &Finding, width: u32, height: u32) -> Option<[f32; 4]> {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for b in line.boxes.get(f.start..f.end.min(line.boxes.len()))? {
        if b.len() == 4 && b[2] > 0.0 && b[3] > 0.0 {
            x0 = x0.min(b[0]);
            y0 = y0.min(b[1]);
            x1 = x1.max(b[0] + b[2]);
            y1 = y1.max(b[1] + b[3]);
        }
    }
    if x0 > x1 {
        return None;
    }
    // Generous padding: a sliver of an ascender left visible can leak a digit.
    let pad = ((y1 - y0) * 0.18).max(2.0);
    let x0 = (x0 - pad).max(0.0);
    let y0 = (y0 - pad).max(0.0);
    let x1 = (x1 + pad).min(width as f64);
    let y1 = (y1 + pad).min(height as f64);
    Some([x0 as f32, y0 as f32, (x1 - x0) as f32, (y1 - y0) as f32])
}

fn clamp_box(b: &[f32; 4], w: u32, h: u32) -> Option<(u32, u32, u32, u32)> {
    let x0 = b[0].max(0.0).floor() as u32;
    let y0 = b[1].max(0.0).floor() as u32;
    let x1 = ((b[0] + b[2]).ceil() as u32).min(w);
    let y1 = ((b[1] + b[3]).ceil() as u32).min(h);
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1, y1))
}

/// Cover every box of every finding.
pub fn cover(img: &RgbaImage, findings: &[ImageFinding], style: Style) -> RgbaImage {
    let boxes: Vec<[f32; 4]> = findings.iter().flat_map(|f| f.boxes.iter().copied()).collect();
    apply(img, &boxes, style)
}

pub fn apply(img: &RgbaImage, boxes: &[[f32; 4]], style: Style) -> RgbaImage {
    let mut out = img.clone();
    let (w, h) = out.dimensions();
    for b in boxes {
        let Some((x0, y0, x1, y1)) = clamp_box(b, w, h) else { continue };
        match style {
            Style::Solid => {
                for y in y0..y1 {
                    for x in x0..x1 {
                        out.put_pixel(x, y, Rgba([17, 18, 20, 255]));
                    }
                }
            }
            Style::Pixelate => {
                let cell = ((y1 - y0) / 2).max(8);
                let mut by = y0;
                while by < y1 {
                    let mut bx = x0;
                    let ey = (by + cell).min(y1);
                    while bx < x1 {
                        let ex = (bx + cell).min(x1);
                        let (mut r, mut g, mut bl, mut n) = (0u64, 0u64, 0u64, 0u64);
                        for y in by..ey {
                            for x in bx..ex {
                                let p = img.get_pixel(x, y);
                                r += p[0] as u64;
                                g += p[1] as u64;
                                bl += p[2] as u64;
                                n += 1;
                            }
                        }
                        let avg = Rgba([(r / n) as u8, (g / n) as u8, (bl / n) as u8, 255]);
                        for y in by..ey {
                            for x in bx..ex {
                                out.put_pixel(x, y, avg);
                            }
                        }
                        bx = ex;
                    }
                    by = ey;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boxes_follow_the_matched_span_only() {
        let text = "mail a@b.co ok";
        let boxes = text.chars().enumerate().map(|(i, _)| vec![10.0 * i as f64, 5.0, 10.0, 20.0]).collect();
        let line = OcrLine { text: text.into(), boxes };
        let f = findings_from_ocr(&[line], 1000, 100);
        assert_eq!(f.len(), 1);
        let b = f[0].boxes[0];
        // "a@b.co" is chars 5..11 -> x 50..110, padded by 3.6px
        assert!((b[0] - 46.4).abs() < 0.01 && (b[2] - 67.2).abs() < 0.01, "{b:?}");
    }

    #[test]
    fn solid_bars_cover_every_pixel() {
        let img = RgbaImage::from_pixel(20, 20, Rgba([255, 255, 255, 255]));
        let out = apply(&img, &[[2.5, 2.5, 10.0, 5.0]], Style::Solid);
        assert_eq!(out.get_pixel(2, 2)[0], 17);
        assert_eq!(out.get_pixel(12, 7)[0], 17);
        assert_eq!(out.get_pixel(13, 8)[0], 255);
    }
}
