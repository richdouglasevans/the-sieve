//! Tauri backend for The Sieve desktop app.
//!
//! Lets the user pick a markdown file and watches it for changes; on each
//! change, rebuilds the PDF (writing it next to the source) and emits a log
//! event to the frontend.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Default)]
pub struct WatcherState {
    debouncer: Mutex<Option<Debouncer<notify::RecommendedWatcher>>>,
    current_path: Mutex<Option<PathBuf>>,
}

#[derive(Serialize, Clone)]
struct LogEntry {
    timestamp: String,
    level: &'static str,
    message: String,
}

fn emit_log(app: &AppHandle, level: &'static str, message: impl Into<String>) {
    let entry = LogEntry {
        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        level,
        message: message.into(),
    };
    let _ = app.emit("log", entry);
}

fn rebuild(path: &Path, app: &AppHandle) {
    let start = Instant::now();
    let markdown = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            emit_log(app, "error", format!("Read failed: {e}"));
            return;
        }
    };
    let base = path.parent().map(Path::to_path_buf).unwrap_or_default();
    match the_sieve::convert_markdown_to_pdf(&markdown, &base) {
        Ok(pdf) => {
            let mut out = path.to_path_buf();
            out.set_extension("pdf");
            match std::fs::write(&out, &pdf) {
                Ok(()) => {
                    let dur = start.elapsed();
                    emit_log(
                        app,
                        "ok",
                        format!("Wrote {} ({} ms)", out.display(), dur.as_millis()),
                    );
                }
                Err(e) => emit_log(app, "error", format!("Write failed: {e}")),
            }
        }
        Err(e) => emit_log(app, "error", format!("Render failed: {e}")),
    }
}

#[tauri::command]
async fn pick_file(app: AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .add_filter("Markdown", &["md", "markdown"])
        .blocking_pick_file()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
async fn start_watching(
    path: String,
    app: AppHandle,
    state: State<'_, WatcherState>,
) -> Result<(), String> {
    let path = PathBuf::from(&path);
    if !path.exists() {
        return Err(format!("File does not exist: {}", path.display()));
    }

    let app_for_cb = app.clone();
    let path_for_cb = path.clone();
    let mut debouncer = new_debouncer(
        Duration::from_millis(200),
        move |result: DebounceEventResult| match result {
            Ok(events) if !events.is_empty() => rebuild(&path_for_cb, &app_for_cb),
            Ok(_) => {}
            Err(e) => emit_log(&app_for_cb, "error", format!("Watch error: {e}")),
        },
    )
    .map_err(|e| e.to_string())?;

    debouncer
        .watcher()
        .watch(&path, RecursiveMode::NonRecursive)
        .map_err(|e| e.to_string())?;

    // Replace any previous watcher; the old one drops here.
    *state.debouncer.lock().unwrap() = Some(debouncer);
    *state.current_path.lock().unwrap() = Some(path.clone());

    emit_log(&app, "info", format!("Watching {}", path.display()));
    rebuild(&path, &app);
    Ok(())
}

#[tauri::command]
async fn stop_watching(state: State<'_, WatcherState>, app: AppHandle) -> Result<(), String> {
    *state.debouncer.lock().unwrap() = None;
    let path = state.current_path.lock().unwrap().take();
    if let Some(p) = path {
        emit_log(&app, "info", format!("Stopped watching {}", p.display()));
    }
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(WatcherState::default())
        .invoke_handler(tauri::generate_handler![
            pick_file,
            start_watching,
            stop_watching
        ])
        .run(tauri::generate_context!())
        .expect("error while running The Sieve desktop app");
}
