//! Load, create, save and watch the on-disk config.

use crate::pipeline::Control;
use anyhow::{Context, Result};
use companion_core::presets::DEFAULT_CONFIG_TOML;
use companion_core::Config;
use crossbeam_channel::Sender;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// Read the config, writing the bundled default first if the file is missing.
/// Returns `(config, created)`; the caller logs `created` once logging is up.
pub fn load_or_create(path: &Path) -> Result<(Config, bool)> {
    let mut created = false;
    if !path.exists() {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        std::fs::write(path, DEFAULT_CONFIG_TOML)
            .with_context(|| format!("writing default config to {}", path.display()))?;
        created = true;
    }
    Ok((load(path)?, created))
}

pub fn load(path: &Path) -> Result<Config> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Config::from_toml(&text).with_context(|| format!("parsing {}", path.display()))
}

/// Serialize and write atomically (temp file + rename) so the watcher never
/// sees a half-written file.
pub fn save(path: &Path, cfg: &Config) -> Result<()> {
    let text = cfg.to_toml()?;
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
    Ok(())
}

/// Watch the config file; on change (debounced) re-parse and send
/// [`Control::Reload`]. A file that fails to parse is logged and ignored, so
/// the engine keeps running on the last good config.
///
/// Returns the watcher, which must be kept alive.
pub fn watch(path: PathBuf, ctrl: Sender<Control>) -> Result<RecommendedWatcher> {
    let dir = path
        .parent()
        .context("config path has no parent directory")?
        .to_path_buf();
    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher =
        RecommendedWatcher::new(tx, notify::Config::default()).context("creating file watcher")?;
    watcher
        .watch(&dir, RecursiveMode::NonRecursive)
        .with_context(|| format!("watching {}", dir.display()))?;

    let file_name = path.file_name().map(|s| s.to_os_string());
    std::thread::Builder::new()
        .name("cc-config-watch".into())
        .spawn(move || {
            let relevant = |ev: &notify::Event| {
                ev.paths
                    .iter()
                    .any(|p| p.file_name().map(|s| s.to_os_string()) == file_name)
            };
            while let Ok(first) = rx.recv() {
                let mut hit = matches!(&first, Ok(ev) if relevant(ev));
                // Debounce: editors write several events per save.
                while let Ok(next) = rx.recv_timeout(Duration::from_millis(250)) {
                    hit |= matches!(&next, Ok(ev) if relevant(ev));
                }
                if !hit {
                    continue;
                }
                match load(&path) {
                    Ok(cfg) => {
                        tracing::info!("configuration changed, reloading");
                        if ctrl.send(Control::Reload(Box::new(cfg))).is_err() {
                            break;
                        }
                    }
                    Err(e) => tracing::warn!("ignoring config change: {e:#}"),
                }
            }
        })
        .context("spawning config watch thread")?;
    Ok(watcher)
}
