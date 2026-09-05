//! Tauri backend for the configuration window.
//!
//! The window edits the same TOML the engine hot-reloads, through the shared
//! `companion-core` types, so both sides agree on the schema. Live status and
//! "detect input" come from the engine's named pipe (see the engine's `ipc.rs`),
//! re-emitted to the webview as the `engine` event.

use companion_core::presets::DEFAULT_CONFIG_TOML;
use companion_core::Config;
use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const ACTIONS_CATALOG: &str = include_str!("../../../presets/actions.json");
const APPS_CATALOG: &str = include_str!("../../../presets/apps.json");

/// Per-application catalog: match rules, the app's own shortcuts, defaults.
#[tauri::command]
fn app_catalog() -> Result<serde_json::Value, String> {
    serde_json::from_str(APPS_CATALOG).map_err(err)
}
const SOCKET_NAME: &str = "CreateCompanion.sock";

/// Last known engine state, so a webview that subscribes after the pipe
/// connected can catch up (`engine_state` command).
#[derive(Default, Clone, Serialize)]
struct EngineState {
    connected: bool,
    hello: Option<serde_json::Value>,
    status: Option<serde_json::Value>,
}

static ENGINE_STATE: Mutex<EngineState> = Mutex::new(EngineState {
    connected: false,
    hello: None,
    status: None,
});

fn remember(v: &serde_json::Value) {
    if let Ok(mut st) = ENGINE_STATE.lock() {
        match v.get("type").and_then(|t| t.as_str()) {
            Some("connected") => st.connected = true,
            Some("disconnected") => {
                st.connected = false;
                st.status = None;
            }
            Some("hello") => {
                st.connected = true;
                st.hello = Some(v.clone());
            }
            Some("status") => st.status = Some(v.clone()),
            _ => {}
        }
    }
}

#[tauri::command]
fn engine_state() -> EngineState {
    ENGINE_STATE.lock().map(|s| s.clone()).unwrap_or_default()
}

/// Write half of the engine pipe, while connected.
static ENGINE_TX: Mutex<Option<Box<dyn std::io::Write + Send>>> = Mutex::new(None);

/// Send one command object to the engine (`{"cmd":"learn","on":true}`).
#[tauri::command]
fn engine_send(command: serde_json::Value) -> Result<(), String> {
    let mut guard = ENGINE_TX.lock().map_err(|_| "engine link poisoned")?;
    let Some(tx) = guard.as_mut() else {
        return Err("engine not connected".into());
    };
    let line = serde_json::to_string(&command).map_err(err)? + "\n";
    tx.write_all(line.as_bytes()).map_err(err)?;
    tx.flush().map_err(err)
}

/// Write a text file chosen through the save dialog (exports).
#[tauri::command]
fn write_text_file(path: String, contents: String) -> Result<(), String> {
    std::fs::write(&path, contents).map_err(err)
}

fn config_file() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("CreateCompanion");
    // Same one-time rename the engine performs (see companion-engine paths.rs).
    let legacy = base.join("NayaCompanion");
    if legacy.is_dir() && !dir.exists() {
        let _ = std::fs::rename(&legacy, &dir);
    }
    dir.join("config.toml")
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Serialize)]
struct Loaded {
    path: String,
    config: serde_json::Value,
    created: bool,
}

/// Read the config (creating the bundled default on first run), as JSON.
#[tauri::command]
fn load_config() -> Result<Loaded, String> {
    let path = config_file();
    let mut created = false;
    if !path.exists() {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(err)?;
        }
        std::fs::write(&path, DEFAULT_CONFIG_TOML).map_err(err)?;
        created = true;
    }
    let text = std::fs::read_to_string(&path).map_err(err)?;
    let mut cfg = Config::from_toml(&text).map_err(|e| format!("{e:#}"))?;
    backfill_names(&mut cfg);
    Ok(Loaded {
        path: path.display().to_string(),
        config: serde_json::to_value(&cfg).map_err(err)?,
        created,
    })
}

/// Configs written before bindings had a `name` get the bundled default's
/// label wherever the action is still the default one.
fn backfill_names(cfg: &mut Config) {
    let Ok(defaults) = Config::from_toml(DEFAULT_CONFIG_TOML) else {
        return;
    };
    let fill = |p: &mut companion_core::profile::Profile, d: &companion_core::profile::Profile| {
        for (ev, b) in p.bindings.iter_mut() {
            if b.name.is_none() {
                if let Some(db) = d.bindings.get(ev) {
                    if db.action == b.action {
                        b.name = db.name.clone();
                    }
                }
            }
        }
    };
    fill(&mut cfg.default_profile, &defaults.default_profile);
    for p in cfg.profiles.iter_mut() {
        // Same name, or (older configs) a shared executable / bundle id with
        // the same kind of title rule -- "Photoshop" vs "Adobe Photoshop".
        let by_name = defaults
            .profiles
            .iter()
            .find(|d| d.name.eq_ignore_ascii_case(&p.name));
        let by_app = defaults.profiles.iter().find(|d| {
            d.app_match.has_title_rule() == p.app_match.has_title_rule()
                && (d.app_match.windows_exe.iter().any(|e| {
                    p.app_match
                        .windows_exe
                        .iter()
                        .any(|x| x.eq_ignore_ascii_case(e))
                }) || d
                    .app_match
                    .macos_bundle
                    .iter()
                    .any(|b| p.app_match.macos_bundle.contains(b)))
        });
        if let Some(d) = by_name.or(by_app) {
            fill(p, d);
        }
    }
}

/// Validate and write the config atomically. The engine's watcher applies it.
#[tauri::command]
fn save_config(config: serde_json::Value) -> Result<(), String> {
    let cfg: Config = serde_json::from_value(config).map_err(|e| format!("invalid config: {e}"))?;
    cfg.transport_table().map_err(|e| e.to_string())?;
    for p in std::iter::once(&cfg.default_profile).chain(cfg.profiles.iter()) {
        for (ev, b) in &p.bindings {
            if let companion_core::action::Action::Keys { chord } = &b.action {
                chord
                    .0
                    .parse::<companion_core::keys::ParsedChord>()
                    .map_err(|e| format!("{} / {ev}: {e}", p.name))?;
            }
        }
    }
    let text = cfg.to_toml().map_err(|e| e.to_string())?;
    let path = config_file();
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, text).map_err(err)?;
    std::fs::rename(&tmp, &path).map_err(err)?;
    Ok(())
}

/// The bundled default config as JSON (for "reset profile to defaults").
#[tauri::command]
fn default_config() -> Result<serde_json::Value, String> {
    let cfg = Config::from_toml(DEFAULT_CONFIG_TOML).map_err(|e| format!("{e:#}"))?;
    serde_json::to_value(&cfg).map_err(err)
}

/// The action catalog generated from the NayaOS reference data.
#[tauri::command]
fn action_catalog() -> Result<serde_json::Value, String> {
    serde_json::from_str(ACTIONS_CATALOG).map_err(err)
}

#[derive(Serialize)]
struct WindowInfo {
    exe: String,
    title: String,
}

/// Visible top-level windows, for the "add application" picker.
#[tauri::command]
fn running_windows() -> Vec<WindowInfo> {
    #[cfg(windows)]
    {
        let mut v: Vec<WindowInfo> = companion_platform::windows::visible_windows()
            .into_iter()
            .filter(|w| !w.exe.eq_ignore_ascii_case("create-companion-ui.exe"))
            .map(|w| WindowInfo {
                exe: w.exe,
                title: w.title,
            })
            .collect();
        v.sort_by(|a, b| a.exe.to_lowercase().cmp(&b.exe.to_lowercase()));
        v
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Validate a chord string as typed / recorded in the UI.
#[tauri::command]
fn validate_chord(chord: String) -> Result<String, String> {
    chord
        .parse::<companion_core::keys::ParsedChord>()
        .map(|c| c.to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_config_folder() -> Result<(), String> {
    let dir = config_file()
        .parent()
        .map(Path::to_path_buf)
        .ok_or("no config dir")?;
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(dir)
            .spawn()
            .map(|_| ())
            .map_err(err)
    }
    #[cfg(not(windows))]
    {
        let _ = dir;
        Err("unsupported".into())
    }
}

/// Connect to the engine's pipe and forward every JSON line to the webview
/// as the `engine` event. Reconnects while the window is open.
fn spawn_engine_listener(app: AppHandle) {
    let emit = move |v: serde_json::Value| {
        remember(&v);
        let _ = app.emit("engine", v);
    };
    std::thread::Builder::new()
        .name("ui-engine-listener".into())
        .spawn(move || {
            use interprocess::local_socket::{prelude::*, GenericNamespaced, Stream, ToNsName};
            loop {
                let Ok(name) = SOCKET_NAME.to_ns_name::<GenericNamespaced>() else {
                    return;
                };
                match Stream::connect(name) {
                    Ok(stream) => {
                        let (recv, send) = stream.split();
                        if let Ok(mut g) = ENGINE_TX.lock() {
                            *g = Some(Box::new(send));
                        }
                        emit(serde_json::json!({"type": "connected"}));
                        let reader = BufReader::new(recv);
                        for line in reader.lines() {
                            let Ok(line) = line else { break };
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                                emit(v);
                            }
                        }
                        if let Ok(mut g) = ENGINE_TX.lock() {
                            *g = None;
                        }
                        emit(serde_json::json!({"type": "disconnected"}));
                    }
                    Err(_) => emit(serde_json::json!({"type": "disconnected"})),
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        })
        .ok();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            default_config,
            action_catalog,
            app_catalog,
            running_windows,
            validate_chord,
            open_config_folder,
            engine_state,
            engine_send,
            write_text_file,
        ])
        .setup(|app| {
            spawn_engine_listener(app.handle().clone());
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_focus();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Create Companion UI");
}
