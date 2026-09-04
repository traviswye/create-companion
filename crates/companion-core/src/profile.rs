//! Application profiles and the resolver that picks one for the foreground app.

use crate::accel::AccelPreset;
use crate::action::Action;
use crate::event::SemanticEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// How the platform layer identifies the foreground application.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppIdentity {
    /// Windows: executable file name only, e.g. `chrome.exe`. Matched case-insensitively.
    WindowsExe(String),
    /// macOS: bundle identifier, e.g. `com.google.Chrome`.
    MacBundle(String),
}

/// Everything the resolver may look at. `title` is only populated when some
/// profile actually has a `window_title` rule (scope §20: titles are read at
/// event time, never stored).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppContext {
    pub id: Option<AppIdentity>,
    pub title: Option<String>,
}

impl AppContext {
    pub fn app(id: AppIdentity) -> Self {
        Self {
            id: Some(id),
            title: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppMatch {
    #[serde(default)]
    pub windows_exe: Vec<String>,
    #[serde(default)]
    pub macos_bundle: Vec<String>,
    /// Case-insensitive substrings of the foreground window title, e.g.
    /// `["YouTube"]`. Lets a profile target a website inside a browser.
    /// Combined with the app rules above when both are present.
    #[serde(default)]
    pub window_title: Vec<String>,
}

impl AppMatch {
    fn app_matches(&self, id: &AppIdentity) -> bool {
        match id {
            AppIdentity::WindowsExe(exe) => {
                self.windows_exe.iter().any(|e| e.eq_ignore_ascii_case(exe))
            }
            AppIdentity::MacBundle(b) => self.macos_bundle.iter().any(|e| e == b),
        }
    }

    fn has_app_rule(&self) -> bool {
        !self.windows_exe.is_empty() || !self.macos_bundle.is_empty()
    }

    pub fn has_title_rule(&self) -> bool {
        !self.window_title.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        !self.has_app_rule() && !self.has_title_rule()
    }

    pub fn matches(&self, ctx: &AppContext) -> bool {
        if self.is_empty() {
            return false;
        }
        if self.has_app_rule() {
            match &ctx.id {
                Some(id) if self.app_matches(id) => {}
                _ => return false,
            }
        }
        if self.has_title_rule() {
            let Some(title) = &ctx.title else {
                return false;
            };
            let title = title.to_lowercase();
            if !self
                .window_title
                .iter()
                .any(|needle| title.contains(&needle.to_lowercase()))
            {
                return false;
            }
        }
        true
    }
}

/// One event's binding inside a profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    pub action: Action,
    #[serde(default)]
    pub accel: AccelPreset,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// Human-readable name shown in the UI.
    pub name: String,
    #[serde(default, rename = "match")]
    pub app_match: AppMatch,
    #[serde(default)]
    pub bindings: HashMap<SemanticEvent, Binding>,
}

impl Profile {
    pub fn binding(&self, ev: SemanticEvent) -> Option<&Binding> {
        self.bindings.get(&ev)
    }
}

/// Picks the profile for the current foreground app, falling back to default.
#[derive(Debug, Clone)]
pub struct ProfileResolver {
    default: Profile,
    apps: Vec<Profile>,
}

impl ProfileResolver {
    pub fn new(default: Profile, apps: Vec<Profile>) -> Self {
        Self { default, apps }
    }

    /// Most specific match wins: a profile with a title rule beats one that
    /// matches on the executable alone (YouTube-in-Chrome beats Browser).
    /// Ties go to config order.
    pub fn resolve(&self, ctx: &AppContext) -> &Profile {
        let mut best: Option<&Profile> = None;
        for p in &self.apps {
            if !p.app_match.matches(ctx) {
                continue;
            }
            match best {
                None => best = Some(p),
                Some(b) if !b.app_match.has_title_rule() && p.app_match.has_title_rule() => {
                    best = Some(p)
                }
                _ => {}
            }
        }
        best.unwrap_or(&self.default)
    }

    /// Look up the binding for an event: the app profile's binding if it has
    /// one, otherwise the default profile's (so an app profile only needs to
    /// override the events it cares about).
    pub fn binding(&self, ctx: &AppContext, ev: SemanticEvent) -> Option<&Binding> {
        self.resolve(ctx)
            .binding(ev)
            .or_else(|| self.default.binding(ev))
    }

    /// True if any profile needs the window title; the platform layer only
    /// reads titles when this is set.
    pub fn uses_titles(&self) -> bool {
        self.apps.iter().any(|p| p.app_match.has_title_rule())
    }

    pub fn default_profile(&self) -> &Profile {
        &self.default
    }

    pub fn app_profiles(&self) -> &[Profile] {
        &self.apps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{KeyChord, MediaKey};
    use crate::event::{Gesture, Module};

    fn keys(s: &str) -> Binding {
        Binding {
            action: Action::Keys {
                chord: KeyChord(s.into()),
            },
            accel: AccelPreset::None,
        }
    }

    fn win(exe: &str) -> AppContext {
        AppContext::app(AppIdentity::WindowsExe(exe.into()))
    }

    fn resolver() -> ProfileResolver {
        let cw = SemanticEvent::new(Module::Tune, Gesture::Cw);
        let press = SemanticEvent::new(Module::Tune, Gesture::Press);
        let default = Profile {
            name: "Default".into(),
            bindings: HashMap::from([
                (
                    cw,
                    Binding {
                        action: Action::Media {
                            key: MediaKey::VolumeUp,
                        },
                        accel: AccelPreset::Light,
                    },
                ),
                (
                    press,
                    Binding {
                        action: Action::Media {
                            key: MediaKey::Mute,
                        },
                        accel: AccelPreset::None,
                    },
                ),
            ]),
            ..Default::default()
        };
        let chrome = Profile {
            name: "Chrome".into(),
            app_match: AppMatch {
                windows_exe: vec!["chrome.exe".into(), "msedge.exe".into()],
                macos_bundle: vec!["com.google.Chrome".into()],
                window_title: vec![],
            },
            bindings: HashMap::from([(cw, keys("Ctrl+Tab"))]),
        };
        let youtube = Profile {
            name: "YouTube".into(),
            app_match: AppMatch {
                windows_exe: vec!["chrome.exe".into()],
                macos_bundle: vec![],
                window_title: vec!["YouTube".into()],
            },
            bindings: HashMap::from([(cw, keys("Right"))]),
        };
        ProfileResolver::new(default, vec![chrome, youtube])
    }

    #[test]
    fn same_event_differs_by_app() {
        let r = resolver();
        let cw = SemanticEvent::new(Module::Tune, Gesture::Cw);

        assert!(matches!(
            r.binding(&win("CHROME.EXE"), cw).unwrap().action,
            Action::Keys { .. }
        ));
        assert!(matches!(
            r.binding(&win("explorer.exe"), cw).unwrap().action,
            Action::Media {
                key: MediaKey::VolumeUp
            }
        ));
        assert!(matches!(
            r.binding(&AppContext::default(), cw).unwrap().action,
            Action::Media {
                key: MediaKey::VolumeUp
            }
        ));
    }

    #[test]
    fn app_profile_falls_back_to_default_for_unbound_events() {
        let r = resolver();
        let press = SemanticEvent::new(Module::Tune, Gesture::Press);
        assert!(matches!(
            r.binding(&win("chrome.exe"), press).unwrap().action,
            Action::Media {
                key: MediaKey::Mute
            }
        ));
    }

    #[test]
    fn bundle_id_matches_on_mac() {
        let r = resolver();
        let ctx = AppContext::app(AppIdentity::MacBundle("com.google.Chrome".into()));
        assert_eq!(r.resolve(&ctx).name, "Chrome");
    }

    #[test]
    fn title_rule_is_more_specific_than_exe_rule() {
        let r = resolver();
        assert!(r.uses_titles());
        let plain = win("chrome.exe").with_title("Inbox - Gmail - Google Chrome");
        assert_eq!(r.resolve(&plain).name, "Chrome");
        let yt = win("chrome.exe").with_title("Rust in 100 seconds - YOUTUBE - Google Chrome");
        assert_eq!(r.resolve(&yt).name, "YouTube");
        // Title rule requires the exe rule too when both are present.
        let other = win("firefox.exe").with_title("YouTube");
        assert_eq!(r.resolve(&other).name, "Default");
        // No title available -> exe-only profile.
        assert_eq!(r.resolve(&win("chrome.exe")).name, "Chrome");
    }

    #[test]
    fn empty_match_never_matches() {
        let m = AppMatch::default();
        assert!(!m.matches(&win("anything.exe")));
    }
}
