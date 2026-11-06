//! `peekless <command>`: the same engine, for terminals and scripts.

use crate::{detect, meta, pipeline, redact};
use std::io::Read;
use std::path::Path;

const HELP: &str = "Peekless: clean screenshots, photos and text before you send them. Offline.

USAGE
  peekless                              open the app
  peekless text [FILE]                  print text with secrets replaced (reads stdin without FILE)
  peekless scan IMAGE... [--text]       list what would be hidden in each image (JSON)
  peekless redact IMAGE [-o OUT] [--pixelate]
                                        write a redacted copy (default: NAME.redacted.png)
  peekless inspect FILE...              show hidden metadata (GPS, camera, author...)
  peekless clean FILE...                write NAME.clean.EXT without metadata (originals untouched)
  peekless clipboard [--pixelate]       clean the clipboard in place, no window (for Raycast, Alfred,
                                        AutoHotkey, Shortcuts...)

EXAMPLES
  pbpaste | peekless text | pbcopy      clean the clipboard text before pasting it into a chat
  peekless clean ~/Pictures/*.jpg
";

const COMMANDS: [&str; 10] = ["text", "scan", "redact", "inspect", "clean", "clipboard", "-h", "--help", "help", "-V"];

pub fn is_command(args: &[String]) -> bool {
    args.get(1).is_some_and(|a| COMMANDS.contains(&a.as_str()) || a == "--version")
}

pub fn run(args: &[String]) -> Option<i32> {
    if !is_command(args) {
        return None;
    }
    let cmd = args.get(1)?.as_str();
    let rest: Vec<&str> = args.iter().skip(2).map(String::as_str).collect();
    let code = match cmd {
        "text" => text(&rest),
        "scan" => scan(&rest),
        "redact" => redact_cmd(&rest),
        "inspect" => inspect(&rest),
        "clean" => clean(&rest),
        "clipboard" => clipboard(&rest),
        "-h" | "--help" | "help" => {
            print!("{HELP}");
            0
        }
        "-V" | "--version" => {
            println!("peekless {}", env!("CARGO_PKG_VERSION"));
            0
        }
        _ => return None,
    };
    Some(code)
}

fn fail(msg: impl std::fmt::Display) -> i32 {
    eprintln!("peekless: {msg}");
    1
}

fn text(rest: &[&str]) -> i32 {
    let mut input = String::new();
    let read = match rest.first() {
        Some(p) => std::fs::read_to_string(p).map(|s| input = s),
        None => std::io::stdin().read_to_string(&mut input).map(|_| ()),
    };
    if let Err(e) = read {
        return fail(e);
    }
    let found = detect::scan(&input);
    let ranges: Vec<_> = found.iter().map(|f| (f.start, f.end, f.kind.to_string())).collect();
    print!("{}", detect::mask_text(&input, &ranges));
    if !found.is_empty() {
        eprintln!("peekless: hid {} item(s): {}", found.len(), summarize(found.iter().map(|f| f.kind)));
    }
    0
}

fn summarize<'a>(kinds: impl Iterator<Item = &'a str>) -> String {
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for k in kinds {
        match counts.iter_mut().find(|(n, _)| *n == k) {
            Some(c) => c.1 += 1,
            None => counts.push((k, 1)),
        }
    }
    counts.iter().map(|(k, n)| format!("{k}×{n}")).collect::<Vec<_>>().join(", ")
}

fn scan(rest: &[&str]) -> i32 {
    if rest.is_empty() {
        return fail("scan needs at least one image");
    }
    let show_text = rest.contains(&"--text");
    let mut all = Vec::new();
    for p in rest.iter().filter(|a| !a.starts_with("--")) {
        let entry = match pipeline::load_image(Path::new(p)).and_then(|img| pipeline::scan_image(&img).map(|s| (img, s))) {
            Ok((img, s)) => {
                let mut v = serde_json::json!({"file": p, "width": img.width(), "height": img.height(), "lines": s.lines, "findings": s.findings});
                if show_text {
                    v["text"] = serde_json::json!(s.text);
                }
                v
            }
            Err(e) => serde_json::json!({"file": p, "error": e}),
        };
        all.push(entry);
    }
    println!("{}", serde_json::to_string_pretty(&all).unwrap());
    0
}

fn redact_cmd(rest: &[&str]) -> i32 {
    let Some(input) = rest.iter().find(|a| !a.starts_with('-')).copied() else { return fail("redact needs an image") };
    let style = if rest.contains(&"--pixelate") { redact::Style::Pixelate } else { redact::Style::Solid };
    let out = rest.iter().position(|a| *a == "-o").and_then(|i| rest.get(i + 1)).map(|s| s.to_string()).unwrap_or_else(|| {
        let p = Path::new(input);
        let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "image".into());
        p.with_file_name(format!("{stem}.redacted.png")).display().to_string()
    });
    let img = match pipeline::load_image(Path::new(input)) {
        Ok(i) => i,
        Err(e) => return fail(e),
    };
    let scanned = match pipeline::scan_image(&img) {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let result = redact::cover(&img, &scanned.findings, style);
    match pipeline::encode_png(&result, false).and_then(|b| std::fs::write(&out, b).map_err(|e| e.to_string())) {
        Ok(()) => {
            eprintln!("peekless: hid {} item(s) ({}) -> {out}", scanned.findings.len(), summarize(scanned.findings.iter().map(|f| f.kind)));
            0
        }
        Err(e) => fail(e),
    }
}

fn inspect(rest: &[&str]) -> i32 {
    for p in rest {
        let r = meta::inspect(Path::new(p));
        println!("{}", r.name);
        if r.items.is_empty() {
            println!("  no hidden metadata found");
        }
        for i in &r.items {
            let mark = match i.risk { 3 => "!!", 2 => "! ", _ => "  " };
            println!("  {mark} {:<10} {}", i.key, i.value);
        }
    }
    0
}

fn clean(rest: &[&str]) -> i32 {
    let mut code = 0;
    for p in rest {
        let r = meta::clean(Path::new(p));
        match (&r.out, &r.error) {
            (Some(out), _) => println!("{p} -> {out} ({} bytes of metadata removed)", r.removed_bytes.max(0)),
            (_, Some(e)) => {
                code = 1;
                eprintln!("{p}: {e}");
            }
            _ => {}
        }
    }
    code
}

fn clipboard(rest: &[&str]) -> i32 {
    let style = if rest.contains(&"--pixelate") { redact::Style::Pixelate } else { redact::Style::Solid };
    match pipeline::read_clipboard() {
        Err(e) => fail(e),
        Ok(pipeline::Clip::Empty) => fail("the clipboard has no image or text"),
        Ok(pipeline::Clip::Text(t)) => {
            let found = detect::scan(&t);
            let ranges: Vec<_> = found.iter().map(|f| (f.start, f.end, f.kind.to_string())).collect();
            if let Err(e) = pipeline::write_text(&detect::mask_text(&t, &ranges)) {
                return fail(e);
            }
            eprintln!("peekless: text cleaned, hid {} item(s) {}", found.len(), summarize(found.iter().map(|f| f.kind)));
            0
        }
        Ok(pipeline::Clip::Image(img)) => {
            let scanned = match pipeline::scan_image(&img) {
                Ok(s) => s,
                Err(e) => return fail(e),
            };
            if let Err(e) = pipeline::write_image(&redact::cover(&img, &scanned.findings, style)) {
                return fail(e);
            }
            eprintln!("peekless: image cleaned, hid {} item(s) {}", scanned.findings.len(), summarize(scanned.findings.iter().map(|f| f.kind)));
            0
        }
    }
}
