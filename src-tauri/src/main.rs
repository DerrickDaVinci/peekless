// Release builds on Windows run without a console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cli;
mod detect;
mod guard;
mod meta;
mod ocr;
mod pipeline;
mod redact;
mod ring;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // Release builds on Windows have no console of their own; borrow the
    // terminal that launched us so CLI output is visible.
    #[cfg(windows)]
    if cli::is_command(&args) {
        unsafe {
            let _ = windows::Win32::System::Console::AttachConsole(windows::Win32::System::Console::ATTACH_PARENT_PROCESS);
        }
    }
    if let Some(code) = cli::run(&args) {
        std::process::exit(code);
    }
    app::run();
}
