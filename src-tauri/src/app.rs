//! The desktop app: tray icon, global shortcut, clipboard guard, action ring,
//! settings, history, and the commands the UI calls.

use crate::ring::Modifier;
use crate::{detect, guard, meta, pipeline, redact};
use base64::Engine;
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent, Wry};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

// ------------------------------------------------------------------ settings

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Lang {
    Auto,
    Zh,
    En,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Theme {
    Auto,
    Light,
    Dark,
}

pub const RING_ACTIONS: [&str; 4] = ["inspect", "strip", "copy", "cover"];

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Settings {
    // general
    lang: Lang,
    theme: Theme,
    launch_at_login: bool,
    start_hidden: bool,
    keep_history: bool,
    // clipboard guard
    guard: bool,
    auto_cover: bool,
    guard_images: bool,
    guard_text: bool,
    card_seconds: u32,
    // action ring
    ring: bool,
    ring_modifier: Modifier,
    /// Enabled ring actions, clockwise from the top (2 to 4 of RING_ACTIONS).
    ring_actions: Vec<String>,
    // shortcut
    shortcut: String,
    shortcut_enabled: bool,
    // detection
    disabled_kinds: Vec<String>,
    style: redact::Style,
    // output
    output_dir: Option<String>,
    clean_tag: String,
    redact_tag: String,
    reveal_after: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            lang: Lang::Auto,
            theme: Theme::Auto,
            launch_at_login: false,
            start_hidden: false,
            keep_history: true,
            guard: true,
            auto_cover: false,
            guard_images: true,
            guard_text: true,
            card_seconds: 15,
            ring: true,
            ring_modifier: Modifier::Option,
            ring_actions: RING_ACTIONS.iter().map(|s| s.to_string()).collect(),
            shortcut: "CmdOrCtrl+Alt+KeyR".into(),
            shortcut_enabled: true,
            disabled_kinds: vec![],
            style: redact::Style::Solid,
            output_dir: None,
            clean_tag: "clean".into(),
            redact_tag: "redacted".into(),
            reveal_after: false,
        }
    }
}

impl Settings {
    /// Repair values a hand-edited or older settings file might carry.
    fn sanitized(mut self) -> Self {
        self.card_seconds = self.card_seconds.clamp(5, 120);
        let mut seen = Vec::new();
        self.ring_actions.retain(|a| RING_ACTIONS.contains(&a.as_str()) && !seen.contains(a) && {
            seen.push(a.clone());
            true
        });
        if self.ring_actions.len() < 2 {
            self.ring_actions = Settings::default().ring_actions;
        }
        let tag = |t: &str, d: &str| {
            let t: String = t.chars().filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_').take(24).collect();
            if t.is_empty() { d.to_string() } else { t }
        };
        self.clean_tag = tag(&self.clean_tag, "clean");
        self.redact_tag = tag(&self.redact_tag, "redacted");
        if self.output_dir.as_deref().is_some_and(|d| d.trim().is_empty()) {
            self.output_dir = None;
        }
        self
    }

    fn keeps(&self, kind: &str) -> bool {
        !self.disabled_kinds.iter().any(|k| k == kind)
    }

    fn out_dir(&self) -> Option<PathBuf> {
        self.output_dir.as_ref().map(PathBuf::from).filter(|p| p.is_dir())
    }
}

// ------------------------------------------------------------------ state

/// What the guard found and is offering to clean.
enum Pending {
    Image { id: u32, findings: Vec<redact::ImageFinding> },
    Text { text: String, findings: Vec<detect::Finding> },
}

/// What was on the clipboard before Peekless replaced it, for Undo.
enum Original {
    Image(RgbaImage),
    Text(String),
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HudItem {
    kind: &'static str,
    preview: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Hud {
    /// "ask": waiting for the user. "done": the clipboard is already clean.
    mode: &'static str,
    kind: &'static str,
    items: Vec<HudItem>,
    src: Option<String>,
    width: u32,
    height: u32,
    boxes: Vec<[f32; 4]>,
    excerpt: Option<String>,
    seconds: u32,
    seq: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoryItem {
    time: u64,
    /// clipboard | ring | review | files
    source: String,
    /// cover | strip | copy | text
    action: String,
    /// What it acted on, for display: a file name, or empty for the clipboard.
    label: String,
    hidden: usize,
    outputs: Vec<String>,
}

struct TrayItems {
    clean: MenuItem<Wry>,
    guard: CheckMenuItem<Wry>,
    open: MenuItem<Wry>,
    quit: MenuItem<Wry>,
}

#[derive(Default)]
struct Store {
    images: Mutex<HashMap<u32, (RgbaImage, Option<String>)>>,
    next: AtomicU32,
    shortcut: Mutex<Option<Shortcut>>,
    shortcut_ok: AtomicBool,
    settings: Mutex<Settings>,
    pending: Mutex<Option<Pending>>,
    original: Mutex<Option<Original>>,
    hud: Mutex<Option<Hud>>,
    tray: Mutex<Option<TrayItems>>,
    history: Mutex<Vec<HistoryItem>>,
}

impl Store {
    fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Review {
    Image { id: u32, width: u32, height: u32, src: String, findings: Vec<redact::ImageFinding>, lines: usize, origin: &'static str, file: Option<String> },
    Text { text: String, findings: Vec<detect::Finding> },
    Empty,
}

fn data_url(img: &RgbaImage) -> Result<String, String> {
    let png = pipeline::encode_png(img, true)?;
    Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png)))
}

fn remember_image(store: &Store, img: RgbaImage, file: Option<String>) -> u32 {
    let id = store.next.fetch_add(1, Ordering::Relaxed) + 1;
    let mut images = store.images.lock().unwrap();
    images.retain(|k, _| *k + 8 > id); // keep only recent ones
    images.insert(id, (img, file));
    id
}

fn scan_image(store: &Store, img: &RgbaImage) -> Result<pipeline::Scanned, String> {
    let s = store.settings();
    let mut scanned = pipeline::scan_image(img)?;
    scanned.findings.retain(|f| s.keeps(f.kind));
    Ok(scanned)
}

fn scan_text(store: &Store, text: &str) -> Vec<detect::Finding> {
    let s = store.settings();
    detect::scan(text).into_iter().filter(|f| s.keeps(f.kind)).collect()
}

fn image_review(store: &Store, img: RgbaImage, origin: &'static str, file: Option<String>) -> Result<Review, String> {
    let scanned = scan_image(store, &img)?;
    let src = data_url(&img)?;
    let (width, height) = img.dimensions();
    let id = remember_image(store, img, file.clone());
    Ok(Review::Image { id, width, height, src, findings: scanned.findings, lines: scanned.lines, origin, file })
}

fn file_name(p: &str) -> String {
    Path::new(p).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// ------------------------------------------------------------------ persistence

fn config_file(app: &AppHandle, name: &str) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join(name))
}

fn read_json<T: for<'de> Deserialize<'de>>(app: &AppHandle, name: &str) -> Option<T> {
    config_file(app, name).and_then(|p| std::fs::read(p).ok()).and_then(|b| serde_json::from_slice(&b).ok())
}

fn write_json<T: Serialize>(app: &AppHandle, name: &str, value: &T) {
    if let Some(p) = config_file(app, name) {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, serde_json::to_vec_pretty(value).unwrap_or_default());
    }
}

/// Recent activity: names and output paths only, never the content.
fn record(app: &AppHandle, source: &str, action: &str, label: String, hidden: usize, outputs: Vec<String>) {
    let store = app.state::<Store>();
    if !store.settings().keep_history {
        return;
    }
    let mut h = store.history.lock().unwrap();
    h.insert(0, HistoryItem { time: now(), source: source.into(), action: action.into(), label, hidden, outputs });
    h.truncate(40);
    write_json(app, "history.json", &*h);
    let _ = app.emit_to("main", "history-changed", ());
}

// ------------------------------------------------------------------ shortcut

fn parse_shortcut(s: &str) -> Option<Shortcut> {
    s.parse::<Shortcut>().ok()
}

/// Swap the registered global shortcut for the one in settings.
fn apply_shortcut(app: &AppHandle, s: &Settings) {
    let store = app.state::<Store>();
    let mut cur = store.shortcut.lock().unwrap();
    if let Some(old) = cur.take() {
        let _ = app.global_shortcut().unregister(old);
    }
    let mut ok = true;
    if s.shortcut_enabled {
        match parse_shortcut(&s.shortcut) {
            Some(sc) => {
                ok = app.global_shortcut().register(sc).is_ok();
                if ok {
                    *cur = Some(sc);
                }
            }
            None => ok = false,
        }
    }
    store.shortcut_ok.store(ok, Ordering::Relaxed);
}

// ------------------------------------------------------------------ the guard

fn spawn_guard(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last = guard::change_count();
        loop {
            std::thread::sleep(Duration::from_millis(450));
            let Some(now) = guard::change_count() else { return };
            if Some(now) == last {
                continue;
            }
            last = Some(now);
            let store = app.state::<Store>();
            let s = store.settings();
            if !s.guard || guard::is_own_write(now) || guard::is_concealed() {
                continue;
            }
            // Let the app that copied finish writing all its formats.
            std::thread::sleep(Duration::from_millis(120));
            if guard::change_count() != Some(now) {
                continue; // changed again; the next tick will handle it
            }
            match pipeline::read_clipboard() {
                Ok(pipeline::Clip::Image(img)) if s.guard_images => {
                    if img.width() as u64 * img.height() as u64 > 60_000_000 {
                        continue;
                    }
                    if let Ok(scanned) = scan_image(&store, &img) {
                        if scanned.findings.iter().any(|f| f.severity >= 2) && guard::change_count() == Some(now) {
                            offer_image(&app, img, scanned.findings);
                        }
                    }
                }
                Ok(pipeline::Clip::Text(text)) if s.guard_text => {
                    if text.len() > 400_000 {
                        continue;
                    }
                    let findings = scan_text(&store, &text);
                    if findings.iter().any(|f| f.severity >= 2) {
                        offer_text(&app, text, findings);
                    }
                }
                _ => {}
            }
        }
    });
}

fn offer_image(app: &AppHandle, img: RgbaImage, findings: Vec<redact::ImageFinding>) {
    let store = app.state::<Store>();
    // Preview at most 720px wide: crisp on retina in a 340pt card, cheap to send.
    let s = (720.0 / img.width() as f32).min(1.0);
    let thumb = if s < 1.0 {
        image::imageops::resize(&img, (img.width() as f32 * s) as u32, (img.height() as f32 * s) as u32, image::imageops::FilterType::Triangle)
    } else {
        img.clone()
    };
    let boxes: Vec<[f32; 4]> = findings.iter().flat_map(|f| f.boxes.iter().map(|b| [b[0] * s, b[1] * s, b[2] * s, b[3] * s])).collect();
    let Ok(src) = data_url(&thumb) else { return };
    let items = findings.iter().map(|f| HudItem { kind: f.kind, preview: f.preview.clone() }).collect();
    let id = remember_image(&store, img, None);
    *store.pending.lock().unwrap() = Some(Pending::Image { id, findings });
    let seq = store.next.fetch_add(1, Ordering::Relaxed);
    let seconds = store.settings().card_seconds;
    let hud = Hud { mode: "ask", kind: "image", items, src: Some(src), width: thumb.width(), height: thumb.height(), boxes, excerpt: None, seconds, seq };
    present(app, hud);
}

fn excerpt(masked: &str) -> String {
    let mut lines: Vec<String> = masked
        .lines()
        .filter(|l| l.contains("[REDACTED:"))
        .take(3)
        .map(|l| {
            let l = l.trim();
            if l.chars().count() > 90 { l.chars().take(90).collect::<String>() + "…" } else { l.to_string() }
        })
        .collect();
    if lines.is_empty() {
        lines.push(masked.chars().take(90).collect());
    }
    lines.join("\n")
}

fn offer_text(app: &AppHandle, text: String, findings: Vec<detect::Finding>) {
    let store = app.state::<Store>();
    let ranges: Vec<_> = findings.iter().map(|f| (f.start, f.end, f.kind.to_string())).collect();
    let items = findings.iter().map(|f| HudItem { kind: f.kind, preview: f.preview.clone() }).collect();
    let ex = excerpt(&detect::mask_text(&text, &ranges));
    *store.pending.lock().unwrap() = Some(Pending::Text { text, findings });
    let seq = store.next.fetch_add(1, Ordering::Relaxed);
    let seconds = store.settings().card_seconds;
    let hud = Hud { mode: "ask", kind: "text", items, src: None, width: 0, height: 0, boxes: vec![], excerpt: Some(ex), seconds, seq };
    present(app, hud);
}

fn present(app: &AppHandle, mut hud: Hud) {
    let store = app.state::<Store>();
    if store.settings().auto_cover && apply_pending(app).is_ok() {
        hud.mode = "done";
    }
    let kind = hud.kind;
    *store.hud.lock().unwrap() = Some(hud);
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || show_hud(&app2, kind));
}

/// Replace the clipboard with the cleaned version of what is pending.
fn apply_pending(app: &AppHandle) -> Result<(), String> {
    let store = app.state::<Store>();
    let style = store.settings().style;
    let pending = store.pending.lock().unwrap();
    let (action, hidden) = match pending.as_ref().ok_or("nothing to clean")? {
        Pending::Image { id, findings } => {
            let (img, _) = store.images.lock().unwrap().get(id).cloned().ok_or("image expired")?;
            pipeline::write_image(&redact::cover(&img, findings, style))?;
            *store.original.lock().unwrap() = Some(Original::Image(img));
            ("cover", findings.len())
        }
        Pending::Text { text, findings } => {
            let ranges: Vec<_> = findings.iter().map(|f| (f.start, f.end, f.kind.to_string())).collect();
            pipeline::write_text(&detect::mask_text(text, &ranges))?;
            *store.original.lock().unwrap() = Some(Original::Text(text.clone()));
            ("text", findings.len())
        }
    };
    drop(pending);
    record(app, "clipboard", action, String::new(), hidden, vec![]);
    Ok(())
}

const HUD_W: f64 = 372.0;

/// The monitor containing a physical point. (Tauri's own monitor_from_point
/// returns None on macOS for points that are plainly on screen, so we match
/// the monitor rectangles ourselves.)
fn monitor_at(app: &AppHandle, x: f64, y: f64) -> Option<tauri::Monitor> {
    let monitors = app.available_monitors().ok().unwrap_or_default();
    let inside = |m: &tauri::Monitor| {
        let (p, s) = (m.position(), m.size());
        x >= p.x as f64 && x < p.x as f64 + s.width as f64 && y >= p.y as f64 && y < p.y as f64 + s.height as f64
    };
    match monitors.iter().position(inside) {
        Some(i) => monitors.into_iter().nth(i),
        None => app.primary_monitor().ok().flatten(),
    }
}

fn show_hud(app: &AppHandle, kind: &str) {
    let h = if kind == "image" { 330.0 } else { 262.0 };
    // Bottom-right of the screen the pointer is on, like the macOS screenshot thumbnail.
    let monitor = app.cursor_position().ok().and_then(|p| monitor_at(app, p.x, p.y));
    let (x, y) = match &monitor {
        Some(m) => {
            let sf = m.scale_factor();
            let wa = m.work_area();
            let right = (wa.position.x as f64 + wa.size.width as f64) / sf;
            let bottom = (wa.position.y as f64 + wa.size.height as f64) / sf;
            (right - HUD_W - 12.0, bottom - h - 12.0)
        }
        None => (40.0, 40.0),
    };
    if let Some(w) = app.get_webview_window("hud") {
        let _ = w.set_size(LogicalSize::new(HUD_W, h));
        let _ = w.set_position(LogicalPosition::new(x, y));
        let _ = w.emit("hud-update", ());
        let _ = w.show();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, "hud", WebviewUrl::App("hud.html".into()))
        .title("Peekless")
        .inner_size(HUD_W, h)
        .position(x, y)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .skip_taskbar(true)
        .focused(false)
        .focusable(false)
        .accept_first_mouse(true)
        .build();
}

fn hide_hud(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("hud") {
        let _ = w.hide();
    }
}

// ------------------------------------------------------------------ the ring

const RING: f64 = 300.0;

fn build_ring(app: &AppHandle) {
    let _ = WebviewWindowBuilder::new(app, "ring", WebviewUrl::App("ring.html".into()))
        .title("Peekless")
        .inner_size(RING, RING)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .skip_taskbar(true)
        .focused(false)
        .focusable(false)
        .visible(false)
        .build();
}

fn show_ring(app: &AppHandle) {
    let Some(w) = app.get_webview_window("ring") else { return };
    let Ok(p) = app.cursor_position() else { return };
    let monitor = monitor_at(app, p.x, p.y);
    let sf = monitor.as_ref().map(|m| m.scale_factor()).unwrap_or(1.0);
    let area = monitor.as_ref().map(|m| {
        let wa = m.work_area();
        [wa.position.x as f64, wa.position.y as f64, wa.size.width as f64, wa.size.height as f64]
    });
    let (x, y) = ring_origin(p.x, p.y, RING * sf, area);
    let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
    let _ = w.emit("ring-show", ());
    let _ = w.show();
}

/// Top-left of the ring window: centred on the cursor, but pushed back inside
/// the screen's usable area (menu bar and Dock excluded) near an edge.
fn ring_origin(cx: f64, cy: f64, size: f64, area: Option<[f64; 4]>) -> (f64, f64) {
    let (mut x, mut y) = (cx - size / 2.0, cy - size / 2.0);
    if let Some([l, t, w, h]) = area {
        x = x.max(l).min((l + w - size).max(l));
        y = y.max(t).min((t + h - size).max(t));
    }
    (x, y)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RingResult {
    done: usize,
    failed: usize,
    hidden: usize,
    outputs: Vec<String>,
    error: Option<String>,
}

fn redact_file(store: &Store, path: &Path) -> Result<(RgbaImage, usize), String> {
    let img = pipeline::load_image(path)?;
    let scanned = scan_image(store, &img)?;
    Ok((redact::cover(&img, &scanned.findings, store.settings().style), scanned.findings.len()))
}

fn reveal_paths(paths: &[String]) {
    #[cfg(target_os = "macos")]
    if !paths.is_empty() {
        let _ = std::process::Command::new("open").arg("-R").args(paths).spawn();
    }
    #[cfg(target_os = "windows")]
    if let Some(p) = paths.first() {
        let _ = std::process::Command::new("explorer").arg(format!("/select,{p}")).spawn();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = paths;
}

#[tauri::command]
async fn ring_action(app: AppHandle, store: State<'_, Store>, action: String, paths: Vec<String>) -> Result<RingResult, String> {
    let s = store.settings();
    let mut r = RingResult { done: 0, failed: 0, hidden: 0, outputs: vec![], error: None };
    let label = match paths.len() {
        1 => file_name(&paths[0]),
        n => format!("{n}"),
    };
    match action.as_str() {
        "inspect" => {
            show_main(&app);
            let _ = app.emit_to("main", "open-files", paths.clone());
            r.done = paths.len();
            return Ok(r);
        }
        "strip" => {
            for p in &paths {
                let c = meta::clean_with(Path::new(p), s.out_dir().as_deref(), &s.clean_tag);
                match c.out {
                    Some(out) => {
                        r.done += 1;
                        r.outputs.push(out);
                    }
                    None => {
                        r.failed += 1;
                        r.error = c.error;
                    }
                }
            }
        }
        "cover" => {
            // Redacted copies are re-encoded PNGs, so they carry no metadata either.
            for p in &paths {
                let path = Path::new(p);
                match redact_file(&store, path).and_then(|(img, n)| {
                    let out = meta::sibling(path, s.out_dir().as_deref(), &s.redact_tag, "png");
                    std::fs::write(&out, pipeline::encode_png(&img, false)?).map_err(|e| e.to_string())?;
                    Ok((out, n))
                }) {
                    Ok((out, n)) => {
                        r.done += 1;
                        r.hidden += n;
                        r.outputs.push(out.display().to_string());
                    }
                    Err(e) => {
                        r.failed += 1;
                        r.error = Some(e);
                    }
                }
            }
        }
        "copy" => match paths.first().map(|p| redact_file(&store, Path::new(p))) {
            Some(Ok((img, n))) => match pipeline::write_image(&img) {
                Ok(()) => {
                    r.done = 1;
                    r.hidden = n;
                }
                Err(e) => r.error = Some(e),
            },
            Some(Err(e)) => {
                r.failed = 1;
                r.error = Some(e);
            }
            None => {}
        },
        _ => return Err("unknown action".into()),
    }
    if r.done > 0 {
        record(&app, "ring", &action, label, r.hidden, r.outputs.clone());
        if s.reveal_after && !r.outputs.is_empty() {
            reveal_paths(&r.outputs);
        }
    }
    Ok(r)
}

#[tauri::command]
fn ring_hide(app: AppHandle) {
    if let Some(w) = app.get_webview_window("ring") {
        let _ = w.hide();
    }
}

// ------------------------------------------------------------------ commands

#[tauri::command]
async fn clean_clipboard(store: State<'_, Store>) -> Result<Review, String> {
    match pipeline::read_clipboard()? {
        pipeline::Clip::Image(img) => image_review(&store, img, "clipboard", None),
        pipeline::Clip::Text(text) => Ok(Review::Text { findings: scan_text(&store, &text), text }),
        pipeline::Clip::Empty => Ok(Review::Empty),
    }
}

#[tauri::command]
async fn open_image(store: State<'_, Store>, path: String) -> Result<Review, String> {
    let img = pipeline::load_image(Path::new(&path))?;
    image_review(&store, img, "file", Some(path))
}

#[tauri::command]
async fn apply_image(app: AppHandle, store: State<'_, Store>, id: u32, boxes: Vec<[f32; 4]>, style: redact::Style, action: String) -> Result<String, String> {
    let (img, file) = store.images.lock().unwrap().get(&id).cloned().ok_or("This image is no longer in memory. Scan it again.")?;
    let out = redact::apply(&img, &boxes, style);
    let label = file.as_deref().map(file_name).unwrap_or_default();
    if action == "copy" {
        pipeline::write_image(&out)?;
        record(&app, "review", "copy", label, boxes.len(), vec![]);
        return Ok("copied".into());
    }
    let s = store.settings();
    let name = file
        .as_deref()
        .and_then(|f| Path::new(f).file_stem())
        .map(|st| format!("{}.{}.png", st.to_string_lossy(), s.redact_tag))
        .unwrap_or_else(|| format!("screenshot.{}.png", s.redact_tag));
    let mut dialog = app.dialog().file().set_file_name(&name).add_filter("PNG", &["png"]);
    if let Some(dir) = s.out_dir().or_else(|| file.as_deref().and_then(|f| Path::new(f).parent().map(Path::to_path_buf))) {
        dialog = dialog.set_directory(dir);
    }
    let Some(target) = dialog.blocking_save_file() else { return Ok("cancelled".into()) };
    let path = target.into_path().map_err(|e| e.to_string())?;
    std::fs::write(&path, pipeline::encode_png(&out, false)?).map_err(|e| e.to_string())?;
    let saved = path.display().to_string();
    record(&app, "review", "cover", label, boxes.len(), vec![saved.clone()]);
    Ok(saved)
}

#[tauri::command]
fn discard(store: State<'_, Store>, id: u32) {
    store.images.lock().unwrap().remove(&id);
}

#[tauri::command]
async fn apply_text(app: AppHandle, text: String, ranges: Vec<(usize, usize, String)>) -> Result<String, String> {
    let masked = detect::mask_text(&text, &ranges);
    pipeline::write_text(&masked)?;
    record(&app, "review", "text", String::new(), ranges.len(), vec![]);
    Ok(masked)
}

#[tauri::command]
async fn inspect_files(paths: Vec<String>) -> Vec<meta::Report> {
    paths.iter().map(|p| meta::inspect(Path::new(p))).collect()
}

#[tauri::command]
async fn clean_files(app: AppHandle, store: State<'_, Store>, paths: Vec<String>) -> Result<Vec<meta::CleanResult>, String> {
    let s = store.settings();
    let out: Vec<meta::CleanResult> = paths.iter().map(|p| meta::clean_with(Path::new(p), s.out_dir().as_deref(), &s.clean_tag)).collect();
    let outputs: Vec<String> = out.iter().filter_map(|r| r.out.clone()).collect();
    if !outputs.is_empty() {
        let label = if paths.len() == 1 { file_name(&paths[0]) } else { format!("{}", paths.len()) };
        record(&app, "files", "strip", label, 0, outputs.clone());
        if s.reveal_after {
            reveal_paths(&outputs);
        }
    }
    Ok(out)
}

#[tauri::command]
async fn pick_files(app: AppHandle) -> Vec<String> {
    app.dialog()
        .file()
        .add_filter("Images", &["jpg", "jpeg", "png", "heic", "heif", "webp", "tif", "tiff", "gif", "bmp"])
        .blocking_pick_files()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| p.into_path().ok())
        .map(|p| p.display().to_string())
        .collect()
}

#[tauri::command]
async fn pick_folder(app: AppHandle) -> Option<String> {
    app.dialog().file().blocking_pick_folder().and_then(|p| p.into_path().ok()).map(|p| p.display().to_string())
}

#[tauri::command]
fn hide_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

#[tauri::command]
fn reveal_many(paths: Vec<String>) {
    reveal_paths(&paths);
}

#[tauri::command]
fn info(store: State<'_, Store>) -> serde_json::Value {
    serde_json::json!({
        "shortcutOk": store.shortcut_ok.load(Ordering::Relaxed),
        "platform": std::env::consts::OS,
        "version": env!("CARGO_PKG_VERSION"),
        "settings": store.settings(),
        "defaults": Settings::default(),
    })
}

#[tauri::command]
fn set_settings(app: AppHandle, store: State<'_, Store>, settings: Settings) -> serde_json::Value {
    let settings = settings.sanitized();
    let old = store.settings();
    *store.settings.lock().unwrap() = settings.clone();
    write_json(&app, "settings.json", &settings);
    if old.shortcut != settings.shortcut || old.shortcut_enabled != settings.shortcut_enabled {
        apply_shortcut(&app, &settings);
    }
    if old.launch_at_login != settings.launch_at_login {
        let al = app.autolaunch();
        let _ = if settings.launch_at_login { al.enable() } else { al.disable() };
    }
    if let Some(t) = store.tray.lock().unwrap().as_ref() {
        let _ = t.guard.set_checked(settings.guard);
    }
    let _ = app.emit("settings-changed", &settings);
    serde_json::json!({ "settings": settings, "shortcutOk": store.shortcut_ok.load(Ordering::Relaxed) })
}

/// Does this string parse as a global shortcut? Used by the recorder.
#[tauri::command]
fn check_shortcut(shortcut: String) -> bool {
    parse_shortcut(&shortcut).is_some()
}

/// The UI knows the user's language; the tray menu follows it.
#[tauri::command]
fn set_lang(store: State<'_, Store>, zh: bool, shortcut: String) {
    if let Some(t) = store.tray.lock().unwrap().as_ref() {
        let (clean, guard, open, quit) = tray_labels(zh, &shortcut);
        let _ = t.clean.set_text(clean);
        let _ = t.guard.set_text(guard);
        let _ = t.open.set_text(open);
        let _ = t.quit.set_text(quit);
    }
}

fn tray_labels(zh: bool, shortcut: &str) -> (String, &'static str, &'static str, &'static str) {
    let sc = if shortcut.is_empty() { String::new() } else { format!("   {shortcut}") };
    if zh {
        (format!("清理剪贴板{sc}"), "自动守护剪贴板", "打开 Peekless", "退出 Peekless")
    } else {
        (format!("Clean Clipboard{sc}"), "Guard Clipboard Automatically", "Open Peekless", "Quit Peekless")
    }
}

#[tauri::command]
fn history(store: State<'_, Store>) -> Vec<HistoryItem> {
    store.history.lock().unwrap().clone()
}

#[tauri::command]
fn clear_history(app: AppHandle, store: State<'_, Store>) {
    store.history.lock().unwrap().clear();
    write_json(&app, "history.json", &Vec::<HistoryItem>::new());
}

/// Put a sample screenshot (fictional data) on the clipboard, as if the user
/// had just taken it, so the guard can be tried safely.
#[tauri::command]
fn try_sample() -> Result<(), String> {
    let img = image::load_from_memory(include_bytes!("../../docs/before.png")).map_err(|e| e.to_string())?.to_rgba8();
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_image(arboard::ImageData { width: img.width() as usize, height: img.height() as usize, bytes: img.as_raw().into() }).map_err(|e| e.to_string())
}

#[tauri::command]
fn hud_payload(store: State<'_, Store>) -> Option<Hud> {
    store.hud.lock().unwrap().clone()
}

#[tauri::command]
fn hud_accept(app: AppHandle, store: State<'_, Store>) -> Result<(), String> {
    apply_pending(&app)?;
    if let Some(h) = store.hud.lock().unwrap().as_mut() {
        h.mode = "done";
    }
    Ok(())
}

#[tauri::command]
fn hud_undo(app: AppHandle, store: State<'_, Store>) -> Result<(), String> {
    match store.original.lock().unwrap().take().ok_or("nothing to undo")? {
        Original::Image(img) => pipeline::write_image(&img)?,
        Original::Text(t) => pipeline::write_text(&t)?,
    }
    hide_hud(&app);
    Ok(())
}

#[tauri::command]
fn hud_review(app: AppHandle, store: State<'_, Store>) -> Result<(), String> {
    let review = match store.pending.lock().unwrap().take().ok_or("nothing to review")? {
        Pending::Image { id, findings } => {
            let (img, _) = store.images.lock().unwrap().get(&id).cloned().ok_or("image expired")?;
            // If we already covered it, undo first so the user edits the original.
            if let Some(Original::Image(orig)) = store.original.lock().unwrap().take() {
                pipeline::write_image(&orig)?;
            }
            Review::Image { id, width: img.width(), height: img.height(), src: data_url(&img)?, findings, lines: 0, origin: "clipboard", file: None }
        }
        Pending::Text { text, findings } => {
            if let Some(Original::Text(orig)) = store.original.lock().unwrap().take() {
                pipeline::write_text(&orig)?;
            }
            Review::Text { text, findings }
        }
    };
    hide_hud(&app);
    show_main(&app);
    let _ = app.emit_to("main", "open-review", review);
    Ok(())
}

#[tauri::command]
fn hud_hide(app: AppHandle) {
    hide_hud(&app);
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn trigger_clean(app: &AppHandle) {
    hide_hud(app);
    show_main(app);
    let _ = app.emit("trigger-clean", ());
}

fn toggle_guard(app: &AppHandle) {
    let store = app.state::<Store>();
    let mut s = store.settings();
    s.guard = !s.guard;
    *store.settings.lock().unwrap() = s.clone();
    write_json(app, "settings.json", &s);
    if let Some(t) = store.tray.lock().unwrap().as_ref() {
        let _ = t.guard.set_checked(s.guard);
    }
    let _ = app.emit("settings-changed", &s);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, sc, event| {
                    let current = *app.state::<Store>().shortcut.lock().unwrap();
                    if event.state() == ShortcutState::Pressed && current.as_ref() == Some(sc) {
                        trigger_clean(app);
                    }
                })
                .build(),
        )
        .manage(Store::default())
        .invoke_handler(tauri::generate_handler![
            clean_clipboard, open_image, apply_image, discard, apply_text, inspect_files, clean_files, pick_files, pick_folder, hide_window,
            reveal_many, info, set_settings, check_shortcut, set_lang, history, clear_history, try_sample, hud_payload, hud_accept,
            hud_undo, hud_review, hud_hide, ring_action, ring_hide
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            let settings = read_json::<Settings>(&handle, "settings.json").unwrap_or_default().sanitized();
            let store = app.state::<Store>();
            *store.settings.lock().unwrap() = settings.clone();
            *store.history.lock().unwrap() = read_json(&handle, "history.json").unwrap_or_default();
            apply_shortcut(&handle, &settings);

            let launched_hidden = std::env::args().any(|a| a == "--hidden");
            if launched_hidden || settings.start_hidden {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }

            let (l_clean, l_guard, l_open, l_quit) = tray_labels(false, "");
            let clean = MenuItem::with_id(app, "clean", l_clean, true, None::<&str>)?;
            let guard_item = CheckMenuItem::with_id(app, "guard", l_guard, true, settings.guard, None::<&str>)?;
            let open = MenuItem::with_id(app, "open", l_open, true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", l_quit, true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&clean, &guard_item, &open, &PredefinedMenuItem::separator(app)?, &quit])?;
            TrayIconBuilder::with_id("main")
                .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?)
                .icon_as_template(true)
                .tooltip("Peekless")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, e| match e.id.as_ref() {
                    "clean" => trigger_clean(app),
                    "open" => show_main(app),
                    "guard" => toggle_guard(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            *store.tray.lock().unwrap() = Some(TrayItems { clean, guard: guard_item, open, quit });

            spawn_guard(handle.clone());

            build_ring(&handle);
            let on_ring = handle.clone();
            let ring_cfg = handle.clone();
            crate::ring::spawn(
                move |signal| match signal {
                    crate::ring::Signal::Show => {
                        let app = on_ring.clone();
                        let _ = on_ring.run_on_main_thread(move || show_ring(&app));
                    }
                    crate::ring::Signal::Release => {
                        let _ = on_ring.emit_to("ring", "ring-release", ());
                    }
                },
                move || {
                    let s = ring_cfg.state::<Store>().settings();
                    s.ring.then_some(s.ring_modifier)
                },
            );
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("failed to start Peekless")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                show_main(app);
            }
            let _ = (app, event);
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    const AREA: Option<[f64; 4]> = Some([0.0, 50.0, 3000.0, 1800.0]); // 50px menu bar

    #[test]
    fn ring_is_centred_away_from_edges() {
        assert_eq!(ring_origin(1500.0, 900.0, 600.0, AREA), (1200.0, 600.0));
    }

    #[test]
    fn ring_stays_on_screen_at_edges_and_corners() {
        assert_eq!(ring_origin(2995.0, 900.0, 600.0, AREA), (2400.0, 600.0)); // right edge
        assert_eq!(ring_origin(3.0, 900.0, 600.0, AREA), (0.0, 600.0)); // left edge
        assert_eq!(ring_origin(1500.0, 55.0, 600.0, AREA), (1200.0, 50.0)); // under the menu bar
        assert_eq!(ring_origin(2990.0, 1845.0, 600.0, AREA), (2400.0, 1250.0)); // bottom-right corner
    }

    #[test]
    fn ring_on_a_second_monitor_uses_that_monitors_area() {
        let right_screen = Some([3000.0, 0.0, 1920.0, 1080.0]);
        assert_eq!(ring_origin(4915.0, 10.0, 600.0, right_screen), (4320.0, 0.0));
    }

    #[test]
    fn shortcut_strings_from_the_recorder_parse() {
        for s in ["CmdOrCtrl+Alt+KeyR", "Cmd+Shift+KeyS", "Ctrl+Alt+Digit1", "Alt+Space", "Cmd+Alt+F5"] {
            assert!(parse_shortcut(s).is_some(), "{s}");
        }
        assert!(parse_shortcut("Cmd+").is_none());
    }

    #[test]
    fn settings_repair_bad_values() {
        let mut s = Settings::default();
        s.card_seconds = 1;
        s.ring_actions = vec!["copy".into(), "nonsense".into()];
        s.clean_tag = "../../etc".into();
        s.output_dir = Some("  ".into());
        let s = s.sanitized();
        assert_eq!(s.card_seconds, 5);
        assert_eq!(s.ring_actions.len(), 4, "fewer than two valid actions falls back to defaults");
        assert_eq!(s.clean_tag, "etc");
        assert!(s.output_dir.is_none());
    }

    #[test]
    fn ring_actions_drop_duplicates_anywhere() {
        let mut s = Settings::default();
        s.ring_actions = vec!["copy".into(), "strip".into(), "copy".into()];
        assert_eq!(s.sanitized().ring_actions, vec!["copy", "strip"]);
    }

    #[test]
    fn old_settings_files_still_load() {
        let s: Settings = serde_json::from_str(r#"{"guard": true, "autoCover": false}"#).unwrap();
        assert!(s.ring && s.guard_images && s.ring_actions.len() == 4);
    }
}
