//! Create Companion background engine.
//!
//! ```text
//! naya-companion                     # tray + engine, config at %APPDATA%\NayaCompanion\config.toml
//! naya-companion --config path.toml  # use another config file (still hot-reloaded)
//! naya-companion --no-tray           # console mode, Ctrl+C to quit
//! naya-companion --print-config      # dump the bundled default config as TOML and exit
//! naya-companion --allow-injected    # also treat synthetic F-keys as transport (testing)
//! ```
//!
//! Logs go to stderr (debug builds / console) and to
//! `%LOCALAPPDATA%\NayaCompanion\logs\`. `RUST_LOG` overrides the config's level.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config_store;
mod ipc;
mod paths;
mod pipeline;
#[cfg(windows)]
mod tray;

use anyhow::{Context, Result};
use companion_core::presets::DEFAULT_CONFIG_TOML;
use std::path::PathBuf;

struct Args {
    config: Option<PathBuf>,
    print_config: bool,
    allow_injected: bool,
    no_tray: bool,
}

fn parse_args() -> Result<Args> {
    let mut args = Args {
        config: None,
        print_config: false,
        allow_injected: false,
        no_tray: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--config" => args.config = Some(it.next().context("--config needs a path")?.into()),
            "--print-config" => args.print_config = true,
            "--allow-injected" => args.allow_injected = true,
            "--no-tray" => args.no_tray = true,
            "-h" | "--help" => {
                println!(
                    "naya-companion [--config <file.toml>] [--no-tray] [--print-config] [--allow-injected]"
                );
                std::process::exit(0);
            }
            other => anyhow::bail!("unknown argument `{other}`"),
        }
    }
    Ok(args)
}

fn init_logging(level: &str) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    let log_dir = paths::log_dir();
    std::fs::create_dir_all(&log_dir).with_context(|| format!("creating {}", log_dir.display()))?;
    let file = tracing_appender::rolling::daily(&log_dir, "companion.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(false).with_writer(std::io::stderr))
        .with(
            fmt::layer()
                .with_target(false)
                .with_ansi(false)
                .with_writer(file_writer),
        )
        .init();
    Ok(guard)
}

#[cfg(not(windows))]
fn main() -> Result<()> {
    anyhow::bail!("Create Companion currently supports Windows only (macOS is Phase 5)")
}

#[cfg(windows)]
fn main() -> Result<()> {
    use companion_platform::windows::{
        autostart, foreground_app, foreground_title, hook, instance, message_loop, KeyboardHook,
        SendInputSink,
    };
    use companion_platform::InputHook;
    use pipeline::{Control, SharedStatus, Status};
    use std::sync::{Arc, Mutex};
    use tray::{Tray, TrayAction};

    let args = parse_args()?;
    if args.print_config {
        print!("{DEFAULT_CONFIG_TOML}");
        return Ok(());
    }

    let _instance = match instance::acquire("NayaCompanion.Engine")? {
        Some(guard) => guard,
        None => {
            eprintln!("naya-companion is already running (see the tray icon).");
            return Ok(());
        }
    };

    let config_path = args.config.clone().unwrap_or_else(paths::config_file);
    let (cfg, created) = config_store::load_or_create(&config_path)?;
    let _log_guard = init_logging(&cfg.engine.log_level)?;
    tracing::info!(
        config = %config_path.display(),
        version = env!("CARGO_PKG_VERSION"),
        "naya-companion starting"
    );
    if created {
        tracing::info!("wrote default configuration (first run)");
    }

    if let Err(e) = autostart::set_enabled(cfg.engine.start_at_login) {
        tracing::warn!("could not apply start_at_login: {e}");
    }

    // Channels: hook -> worker (bounded, hook never blocks), tray/watcher -> worker,
    // worker -> IPC clients (the configuration UI).
    let (ev_tx, ev_rx) = crossbeam_channel::bounded(64);
    let (ctrl_tx, ctrl_rx) = crossbeam_channel::unbounded::<Control>();
    let (ipc_tx, ipc_rx) = crossbeam_channel::unbounded::<ipc::IpcMessage>();
    if let Err(e) = ipc::start(
        ipc_rx,
        ipc::IpcMessage::Hello {
            version: env!("CARGO_PKG_VERSION").into(),
            config: config_path.display().to_string(),
        },
        ctrl_tx.clone(),
    ) {
        tracing::warn!(
            "IPC server unavailable, the configuration UI will not see live status: {e:#}"
        );
    }

    hook::set_allow_injected(args.allow_injected);
    let mut kb_hook = KeyboardHook::new(ev_tx);
    kb_hook.start().context("installing keyboard hook")?;

    let status: SharedStatus = Arc::new(Mutex::new(Status {
        profile: cfg.default_profile.name.clone(),
        ..Default::default()
    }));

    let tray = if args.no_tray {
        None
    } else {
        Some(Tray::new(autostart::is_enabled().unwrap_or(false)).context("creating tray")?)
    };
    let waker = message_loop::Waker::for_current_thread();

    let worker = {
        let status = Arc::clone(&status);
        let on_status = move || waker.wake();
        std::thread::Builder::new()
            .name("naya-pipeline".into())
            .spawn(move || {
                pipeline::run(
                    cfg,
                    ev_rx,
                    ctrl_rx,
                    status,
                    on_status,
                    foreground_app,
                    foreground_title,
                    hook::set_reserved,
                    hook::set_learn,
                    ipc_tx,
                    SendInputSink,
                )
            })
            .context("spawning pipeline thread")?
    };

    let _watcher = config_store::watch(config_path.clone(), ctrl_tx.clone())?;

    {
        let ctrl_tx = ctrl_tx.clone();
        ctrlc::set_handler(move || {
            tracing::info!("shutting down");
            let _ = ctrl_tx.send(Control::Quit);
            waker.quit();
            // In --no-tray mode there is no message loop to observe WM_QUIT.
            std::thread::sleep(std::time::Duration::from_millis(200));
            std::process::exit(0);
        })
        .context("installing Ctrl+C handler")?;
    }

    tracing::info!(tray = tray.is_some(), "naya-companion running");

    match tray {
        None => {
            let _ = worker.join();
        }
        Some(tray) => {
            if let Ok(s) = status.lock() {
                tray.set_status(&s);
            }
            message_loop::run(|| {
                for action in tray.poll() {
                    match action {
                        TrayAction::OpenUi => open_ui(&config_path),
                        TrayAction::OpenConfigFile => open_path(&config_path),
                        TrayAction::OpenConfigFolder => {
                            if let Some(dir) = config_path.parent() {
                                open_path(dir);
                            }
                        }
                        TrayAction::Reload => match config_store::load(&config_path) {
                            Ok(c) => {
                                let _ = ctrl_tx.send(Control::Reload(Box::new(c)));
                            }
                            Err(e) => tracing::warn!("reload failed: {e:#}"),
                        },
                        TrayAction::SetPaused(p) => {
                            let _ = ctrl_tx.send(Control::SetPaused(p));
                        }
                        TrayAction::SetAutostart(enabled) => {
                            match autostart::set_enabled(enabled) {
                                Ok(()) => {
                                    tray.set_autostart(enabled);
                                    // Persist so the setting survives; the watcher
                                    // will reload, which is harmless.
                                    match config_store::load(&config_path) {
                                        Ok(mut c) => {
                                            c.engine.start_at_login = enabled;
                                            if let Err(e) = config_store::save(&config_path, &c) {
                                                tracing::warn!("could not save config: {e:#}");
                                            }
                                        }
                                        Err(e) => tracing::warn!("could not read config: {e:#}"),
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("autostart change failed: {e}");
                                    tray.set_autostart(!enabled);
                                }
                            }
                        }
                        TrayAction::Quit => {
                            let _ = ctrl_tx.send(Control::Quit);
                            waker.quit();
                        }
                    }
                }
                if let Ok(s) = status.lock() {
                    tray.set_status(&s);
                }
            });
            let _ = worker.join();
        }
    }

    kb_hook.stop();
    tracing::info!("naya-companion stopped");
    Ok(())
}

/// Launch the configuration UI (`naya-companion-ui.exe` next to this binary).
/// Falls back to opening the config file when the UI is not installed.
#[cfg(windows)]
fn open_ui(config_path: &std::path::Path) {
    let ui = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("naya-companion-ui.exe")));
    match ui {
        Some(exe) if exe.exists() => {
            if let Err(e) = std::process::Command::new(&exe).spawn() {
                tracing::warn!("could not start {}: {e}", exe.display());
            }
        }
        _ => {
            tracing::info!(
                "configuration UI not found next to the engine; opening the config file"
            );
            open_path(config_path);
        }
    }
}

#[cfg(windows)]
fn open_path(path: &std::path::Path) {
    // `explorer` opens folders directly and files with their default handler.
    if let Err(e) = std::process::Command::new("explorer").arg(path).spawn() {
        tracing::warn!("could not open {}: {e}", path.display());
    }
}
