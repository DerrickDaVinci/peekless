# Contributing

Thanks for helping. A few things keep Peekless small and trustworthy.

## Setup

You need [Rust](https://rustup.rs), Node.js 18+, and on macOS the Xcode Command Line Tools.

```bash
npm install
npm run dev                 # run the app
cd src-tauri && cargo test  # run the tests
```

The UI is plain HTML, CSS and JS in `ui/` with no build step. Open `ui/index.html` in any browser to work on it: `ui/preview-mock.js` fakes the backend with sample data when the page runs outside the app.

## Where things live

| File | What it does |
|---|---|
| `src-tauri/src/detect.rs` | Detection rules and validators |
| `src-tauri/src/ocr.rs` | Text recognition (Vision on macOS, Windows.Media.Ocr on Windows) |
| `src-tauri/src/redact.rs` | Turning matches into boxes and burning them in |
| `src-tauri/src/meta.rs` | Reading and removing metadata (lossless for JPEG and PNG) |
| `src-tauri/src/guard.rs` | Cheap clipboard change detection |
| `src-tauri/src/ring.rs` | Detecting "dragging files with the trigger key held" |
| `src-tauri/src/app.rs` | Settings, windows, tray and the commands the UI calls |
| `src-tauri/src/cli.rs` | The `peekless` command line |

## Adding a detection rule

1. Add a `rule(...)` in `detect.rs`. Use a validator (checksum, date check) whenever the format has one.
2. Add tests for what it **catches** and for ordinary text it must **leave alone** (`quiet_on_ordinary_text`). False positives erode trust as much as misses.
3. Add a label for the new kind in `ui/app.js` (`KIND` and `RULES`) and `ui/hud.js`.

## Ground rules

- **No network code.** Not for updates, analytics or crash reports.
- **Never modify the user's original files.** Write new files next to them.
- **Prefer less code.** Reuse what exists, use the standard library and the OS before adding a dependency. A new dependency needs a reason in the pull request.
- Keep pull requests focused, and run `cargo test` before opening one.

When you report a detection problem, please cover real personal data in your screenshot first (Peekless can do that for you) or recreate it with fake values.
