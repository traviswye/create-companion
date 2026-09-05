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

    /// How narrowly this rule targets: higher wins when several profiles
    /// match the same window. A title rule is the most specific thing a rule
    /// can have; among app rules, one that names a single executable is
    /// more specific than a group ("Browser" listing seven browsers).
    pub fn specificity(&self) -> (bool, i32) {
        let apps = (self.windows_exe.len() + self.macos_bundle.len()) as i32;
        (self.has_title_rule(), -apps)
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
    /// Plain-English label shown in the UI ("Increase Brush Size"); the
    /// action itself holds the chord. Optional for custom shortcuts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// For streamed gestures: `Some(true)` = every key of the run fires the
    /// action, `Some(false)` = one event per swipe, `None` = the input's
    /// default (`InputEntry::follow`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow: Option<bool>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// Human-readable name shown in the UI.
    pub name: String,
    /// Disabled profiles keep their bindings but never match (the UI's star).
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, rename = "match")]
    pub app_match: AppMatch,
    #[serde(default)]
    pub bindings: HashMap<SemanticEvent, Binding>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: String::new(),
            enabled: true,
            app_match: AppMatch::default(),
            bindings: HashMap::new(),
        }
    }
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
    /// Bindings here beat every app profile, whatever is in the foreground.
    god: Profile,
    apps: Vec<Profile>,
}

impl ProfileResolver {
    pub fn new(default: Profile, apps: Vec<Profile>) -> Self {
        Self::with_god_mode(
            default,
            Profile {
                name: "God Mode".into(),
                ..Profile::default()
            },
            apps,
        )
    }

    pub fn with_god_mode(default: Profile, god: Profile, apps: Vec<Profile>) -> Self {
        Self { default, god, apps }
    }

    pub fn god_mode(&self) -> &Profile {
        &self.god
    }

    /// The override binding for an event, if God Mode has one (exact finger
    /// count first, then the any-count form).
    pub fn god_binding(&self, ev: SemanticEvent) -> Option<&Binding> {
        self.god.binding(ev).or_else(|| {
            (ev.fingers.is_some())
                .then(|| self.god.binding(ev.without_fingers()))
                .flatten()
        })
    }

    /// Most specific match wins: a profile with a title rule beats one that
    /// matches on the executable alone (YouTube-in-Chrome beats Browser), and
    /// a profile naming one executable beats a group that lists several
    /// (Google Chrome beats Browser). Remaining ties go to config order, which
    /// the UI lets the user drag.
    pub fn resolve(&self, ctx: &AppContext) -> &Profile {
        let mut best: Option<&Profile> = None;
        for p in &self.apps {
            if !p.enabled || !p.app_match.matches(ctx) {
                continue;
            }
            match best {
                None => best = Some(p),
                Some(b) if p.app_match.specificity() > b.app_match.specificity() => best = Some(p),
                _ => {}
            }
        }
        best.unwrap_or(&self.default)
    }

    /// Look up the binding for an event: the app profile's binding if it has
    /// one, otherwise the default profile's (so an app profile only needs to
    /// override the events it cares about). A binding without a finger count
    /// serves every count of that gesture, so `TUNE_SWIPE_LEFT_3F` falls back
    /// to `TUNE_SWIPE_LEFT`.
    pub fn binding(&self, ctx: &AppContext, ev: SemanticEvent) -> Option<&Binding> {
        if let Some(b) = self.god_binding(ev) {
            return Some(b);
        }
        let app = self.resolve(ctx);
        let any = ev.without_fingers();
        app.binding(ev)
            .or_else(|| (ev.fingers.is_some()).then(|| app.binding(any)).flatten())
            .or_else(|| self.default.binding(ev))
            .or_else(|| {
                (ev.fingers.is_some())
                    .then(|| self.default.binding(any))
                    .flatten()
            })
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
                hold_ms: None,
            },
            accel: AccelPreset::None,
            name: None,
            follow: None,
        }
    }

    fn win(exe: &str) -> AppContext {
        AppContext::app(AppIdentity::WindowsExe(exe.into()))
    }

    fn resolver() -> ProfileResolver {
        let cw = SemanticEvent::new(Module::Tune, Gesture::Cw);
        let press = SemanticEvent::with_fingers(Module::Tune, Gesture::Tap, 1);
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
                        name: None,
                        follow: None,
                    },
                ),
                (
                    press,
                    Binding {
                        action: Action::Media {
                            key: MediaKey::Mute,
                        },
                        accel: AccelPreset::None,
                        name: None,
                        follow: None,
                    },
                ),
            ]),
            ..Default::default()
        };
        let chrome = Profile {
            name: "Chrome".into(),
            enabled: true,
            app_match: AppMatch {
                windows_exe: vec!["chrome.exe".into(), "msedge.exe".into()],
                macos_bundle: vec!["com.google.Chrome".into()],
                window_title: vec![],
            },
            bindings: HashMap::from([(cw, keys("Ctrl+Tab"))]),
        };
        let youtube = Profile {
            name: "YouTube".into(),
            enabled: true,
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
        let press = SemanticEvent::with_fingers(Module::Tune, Gesture::Tap, 1);
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
    fn god_mode_binding_wins_everywhere() {
        let mut r = resolver();
        let ev: SemanticEvent = "TUNE_CW".parse().unwrap();
        let chrome = win("chrome.exe");
        // Chrome binds the dial itself...
        assert!(r.resolve(&chrome).name != r.god_mode().name);
        let before = r.binding(&chrome, ev).unwrap().action.clone();
        // ...but a God Mode binding takes over, in Chrome and on the desktop.
        r.god.bindings.insert(
            ev,
            Binding {
                action: Action::Keys {
                    chord: KeyChord("Alt+Tab".into()),
                    hold_ms: None,
                },
                accel: Default::default(),
                name: None,
                follow: None,
            },
        );
        let after = r.binding(&chrome, ev).unwrap().action.clone();
        assert_ne!(before, after);
        assert_eq!(
            after,
            Action::Keys {
                chord: KeyChord("Alt+Tab".into()),
                hold_ms: None
            }
        );
        assert_eq!(r.binding(&AppContext::default(), ev).unwrap().action, after);
    }

    #[test]
    fn single_exe_profile_beats_a_group_listed_earlier() {
        let group = Profile {
            name: "Browser".into(),
            enabled: true,
            app_match: AppMatch {
                windows_exe: vec![
                    "chrome.exe".into(),
                    "msedge.exe".into(),
                    "firefox.exe".into(),
                ],
                macos_bundle: vec![],
                window_title: vec![],
            },
            bindings: Default::default(),
        };
        let one = Profile {
            name: "Google Chrome".into(),
            enabled: true,
            app_match: AppMatch {
                windows_exe: vec!["chrome.exe".into()],
                macos_bundle: vec![],
                window_title: vec![],
            },
            bindings: Default::default(),
        };
        let r = ProfileResolver::new(Profile::default(), vec![group.clone(), one.clone()]);
        assert_eq!(r.resolve(&win("chrome.exe")).name, "Google Chrome");
        assert_eq!(r.resolve(&win("msedge.exe")).name, "Browser");
        // Same specificity: config order decides.
        let twin = Profile {
            name: "Chrome again".into(),
            ..one.clone()
        };
        let r = ProfileResolver::new(Profile::default(), vec![group, twin, one]);
        assert_eq!(r.resolve(&win("chrome.exe")).name, "Chrome again");
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
    fn finger_less_binding_serves_any_finger_count() {
        let r = resolver();
        let any: SemanticEvent = "TUNE_SWIPE_LEFT".parse().unwrap();
        let three: SemanticEvent = "TUNE_SWIPE_LEFT_3F".parse().unwrap();
        let mut default = r.default.clone();
        default.bindings.insert(any, keys("Alt+Left"));
        let r = ProfileResolver::new(default, r.apps.clone());
        assert!(matches!(
            r.binding(&win("explorer.exe"), three).unwrap().action,
            Action::Keys { .. }
        ));
        // A finger-specific binding wins over the any-count one.
        let mut d2 = r.default.clone();
        d2.bindings.insert(three, keys("Ctrl+Left"));
        let r2 = ProfileResolver::new(d2, vec![]);
        match &r2.binding(&win("x.exe"), three).unwrap().action {
            Action::Keys { chord, .. } => assert_eq!(chord.0, "Ctrl+Left"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn disabled_profile_is_skipped_but_kept() {
        let mut r = resolver();
        r.apps[0].enabled = false;
        assert_eq!(r.resolve(&win("chrome.exe")).name, "Default");
        assert_eq!(r.app_profiles()[0].name, "Chrome");
        assert!(!r.app_profiles()[0].bindings.is_empty());
    }

    #[test]
    fn empty_match_never_matches() {
        let m = AppMatch::default();
        assert!(!m.matches(&win("anything.exe")));
    }
}
