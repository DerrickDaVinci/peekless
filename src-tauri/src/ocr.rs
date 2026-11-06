//! On-device text recognition using the engine the OS already ships:
//! Apple Vision on macOS, Windows.Media.Ocr on Windows. No model downloads,
//! no network, and on macOS the image never touches disk.

#[derive(Debug, Clone)]
pub struct OcrLine {
    pub text: String,
    /// One box per char of `text`: [x, y, w, h] in pixels, top-left origin.
    /// Empty when the engine could not place that char.
    pub boxes: Vec<Vec<f64>>,
}

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use objc2::rc::Retained;
    use objc2::AnyThread;
    use objc2_foundation::{NSArray, NSData, NSDictionary, NSRange, NSString};
    use objc2_vision::{VNImageRequestHandler, VNRecognizeTextRequest, VNRequest, VNRequestTextRecognitionLevel};

    const LANGS: [&str; 3] = ["zh-Hans", "zh-Hant", "en-US"];

    pub fn recognize_png(png: &[u8]) -> Result<Vec<OcrLine>, String> {
        if png.len() < 24 || &png[12..16] != b"IHDR" {
            return Err("expected a PNG image".into());
        }
        let w = u32::from_be_bytes(png[16..20].try_into().unwrap()) as f64;
        let h = u32::from_be_bytes(png[20..24].try_into().unwrap()) as f64;
        objc2::rc::autoreleasepool(|_| unsafe { run(png, w, h) })
    }

    unsafe fn run(png: &[u8], w: f64, h: f64) -> Result<Vec<OcrLine>, String> {
        let request = VNRecognizeTextRequest::new();
        request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
        request.setUsesLanguageCorrection(false);
        if let Ok(supported) = request.supportedRecognitionLanguagesAndReturnError() {
            let have: Vec<String> = supported.iter().map(|s| s.to_string()).collect();
            let langs: Vec<Retained<NSString>> = LANGS.iter().filter(|l| have.iter().any(|h| h == *l)).map(|l| NSString::from_str(l)).collect();
            request.setRecognitionLanguages(&NSArray::from_retained_slice(&langs));
        }

        let data = NSData::with_bytes(png);
        let handler = VNImageRequestHandler::initWithData_options(VNImageRequestHandler::alloc(), &data, &NSDictionary::new());
        let as_request: &VNRequest = &request;
        handler
            .performRequests_error(&NSArray::from_slice(&[as_request]))
            .map_err(|e| format!("text recognition failed: {}", e.localizedDescription()))?;

        let mut lines = Vec::new();
        let Some(results) = request.results() else { return Ok(lines) };
        for observation in results.iter() {
            let Some(candidate) = observation.topCandidates(1).iter().next() else { continue };
            let text = candidate.string().to_string();
            // Vision ranges are UTF-16 based; walk chars and ask for each box.
            let mut boxes = Vec::with_capacity(text.len());
            let mut at = 0usize;
            for ch in text.chars() {
                let len = ch.len_utf16();
                let b = match candidate.boundingBoxForRange_error(NSRange::new(at, len)) {
                    Ok(rect) => {
                        let r = rect.boundingBox();
                        // normalized, bottom-left origin -> pixels, top-left origin
                        vec![r.origin.x * w, (1.0 - r.origin.y - r.size.height) * h, r.size.width * w, r.size.height * h]
                    }
                    Err(_) => vec![],
                };
                boxes.push(b);
                at += len;
            }
            lines.push(OcrLine { text, boxes });
        }
        Ok(lines)
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use windows::core::HSTRING;
    use windows::Graphics::Imaging::BitmapDecoder;
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::{FileAccessMode, StorageFile};

    pub fn recognize_png(png: &[u8]) -> Result<Vec<OcrLine>, String> {
        // WinRT wants a StorageFile; the temp copy is deleted right after.
        let path = std::env::temp_dir().join(format!("peekless-ocr-{}.png", std::process::id()));
        std::fs::write(&path, png).map_err(|e| e.to_string())?;
        let result = run(&path);
        let _ = std::fs::remove_file(&path);
        result.map_err(|e| format!("text recognition failed: {e}"))
    }

    fn run(path: &std::path::Path) -> windows::core::Result<Vec<OcrLine>> {
        let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(path.as_os_str()))?.get()?;
        let stream = file.OpenAsync(FileAccessMode::Read)?.get()?;
        let decoder = BitmapDecoder::CreateAsync(&stream)?.get()?;
        let bitmap = decoder.GetSoftwareBitmapAsync()?.get()?;
        let engine = OcrEngine::TryCreateFromUserProfileLanguages()?;
        let result = engine.RecognizeAsync(&bitmap)?.get()?;
        let mut lines = Vec::new();
        for line in result.Lines()? {
            let mut text = String::new();
            let mut boxes: Vec<Vec<f64>> = Vec::new();
            for word in line.Words()? {
                let w = word.Text()?.to_string();
                let r = word.BoundingRect()?;
                let cjk = |c: Option<char>| c.is_some_and(|c| c as u32 >= 0x2E80);
                if !text.is_empty() && !(cjk(text.chars().last()) && cjk(w.chars().next())) {
                    text.push(' ');
                    boxes.push(vec![]);
                }
                for _ in w.chars() {
                    boxes.push(vec![r.X as f64, r.Y as f64, r.Width as f64, r.Height as f64]);
                }
                text.push_str(&w);
            }
            lines.push(OcrLine { text, boxes });
        }
        Ok(lines)
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod imp {
    use super::*;
    pub fn recognize_png(_png: &[u8]) -> Result<Vec<OcrLine>, String> {
        Err("text recognition is only available on macOS and Windows for now".into())
    }
}

pub use imp::recognize_png;
