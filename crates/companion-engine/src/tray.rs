//! System tray icon and menu (scope §15).
//!
//! ```text
//! Create Companion
//! Active: Photoshop          (disabled, informational)
//! Last: TUNE_CW -> ]         (disabled, informational)
//! ---
//! Open configuration...        (launches create-companion-ui)
//! Edit configuration file
//! Open configuration folder
//! Reload configuration
//! [ ] Pause companion
//! [ ] Start at login
//! ---
//! Quit
//! ```

use crate::pipeline::Status;
use anyhow::{Context, Result};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    OpenUi,
    OpenConfigFile,
    OpenConfigFolder,
    Reload,
    SetPaused(bool),
    SetAutostart(bool),
    Quit,
}

pub struct Tray {
    icon: TrayIcon,
    active: MenuItem,
    last: MenuItem,
    pause: CheckMenuItem,
    autostart: CheckMenuItem,
    ids: Ids,
}

struct Ids {
    open_ui: MenuId,
    open_file: MenuId,
    open_folder: MenuId,
    reload: MenuId,
    pause: MenuId,
    autostart: MenuId,
    quit: MenuId,
}

/// 32x32 RGBA: a filled ring, the closest thing to a dial we can draw in code.
fn make_icon() -> Result<Icon> {
    const N: usize = 32;
    let mut rgba = vec![0u8; N * N * 4];
    let c = (N as f32 - 1.0) / 2.0;
    for y in 0..N {
        for x in 0..N {
            let d = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt();
            let (r, g, b, a) = if (5.5..=14.5).contains(&d) {
                (235, 235, 235, 255)
            } else if (14.5..15.5).contains(&d) || (4.5..5.5).contains(&d) {
                (235, 235, 235, 120) // soft edge
            } else {
                (0, 0, 0, 0)
            };
            // A notch at 12 o'clock so it reads as a dial.
            let notch = (x as f32 - c).abs() < 1.6 && y < 9 && y > 2;
            let (r, g, b, a) = if notch {
                (30, 30, 30, 255)
            } else {
                (r, g, b, a)
            };
            let i = (y * N + x) * 4;
            rgba[i..i + 4].copy_from_slice(&[r, g, b, a]);
        }
    }
    Icon::from_rgba(rgba, N as u32, N as u32).context("building tray icon")
}

impl Tray {
    pub fn new(autostart_enabled: bool) -> Result<Self> {
        let active = MenuItem::new("Active: —", false, None);
        let last = MenuItem::new("Last: —", false, None);
        let open_ui = MenuItem::new("Open configuration...", true, None);
        let open_file = MenuItem::new("Edit configuration file", true, None);
        let open_folder = MenuItem::new("Open configuration folder", true, None);
        let reload = MenuItem::new("Reload configuration", true, None);
        let pause = CheckMenuItem::new("Pause companion", true, false, None);
        let autostart = CheckMenuItem::new("Start at login", true, autostart_enabled, None);
        let quit = MenuItem::new("Quit", true, None);

        let menu = Menu::new();
        menu.append_items(&[
            &active,
            &last,
            &PredefinedMenuItem::separator(),
            &open_ui,
            &open_file,
            &open_folder,
            &reload,
            &pause,
            &autostart,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .context("building tray menu")?;

        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Create Companion")
            .with_icon(make_icon()?)
            .build()
            .context("creating tray icon")?;

        Ok(Self {
            icon,
            ids: Ids {
                open_ui: open_ui.id().clone(),
                open_file: open_file.id().clone(),
                open_folder: open_folder.id().clone(),
                reload: reload.id().clone(),
                pause: pause.id().clone(),
                autostart: autostart.id().clone(),
                quit: quit.id().clone(),
            },
            active,
            last,
            pause,
            autostart,
        })
    }

    pub fn set_status(&self, s: &Status) {
        let state = if s.paused { " (paused)" } else { "" };
        self.active
            .set_text(format!("Active: {}{state}", s.profile));
        self.last.set_text(match (&s.last_event, &s.last_action) {
            (Some(ev), Some(act)) => format!("Last: {ev} \u{2192} {act}"),
            _ => "Last: —".to_string(),
        });
        self.pause.set_checked(s.paused);
        let _ = self.icon.set_tooltip(Some(format!(
            "Create Companion\nProfile: {}{state}",
            s.profile
        )));
    }

    pub fn set_autostart(&self, enabled: bool) {
        self.autostart.set_checked(enabled);
    }

    /// Drain pending menu clicks. Call from the message loop.
    pub fn poll(&self) -> Vec<TrayAction> {
        MenuEvent::receiver()
            .try_iter()
            .filter_map(|ev| {
                let id = ev.id();
                Some(if *id == self.ids.open_ui {
                    TrayAction::OpenUi
                } else if *id == self.ids.open_file {
                    TrayAction::OpenConfigFile
                } else if *id == self.ids.open_folder {
                    TrayAction::OpenConfigFolder
                } else if *id == self.ids.reload {
                    TrayAction::Reload
                } else if *id == self.ids.pause {
                    TrayAction::SetPaused(self.pause.is_checked())
                } else if *id == self.ids.autostart {
                    TrayAction::SetAutostart(self.autostart.is_checked())
                } else if *id == self.ids.quit {
                    TrayAction::Quit
                } else {
                    return None;
                })
            })
            .collect()
    }
}
