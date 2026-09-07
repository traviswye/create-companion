//! hook -> decoder -> foreground app -> profile -> action.
//!
//! [`Engine`] holds the decision logic and is OS-free so it can be unit-tested
//! with a mock sink. [`run`] wraps it in the worker thread's loop.

use crate::ipc::IpcMessage;
use anyhow::{Context, Result};
use companion_core::accel::{AccelCurve, AccelSettings, RotaryState};
use companion_core::action::Action;
use companion_core::event::SemanticEvent;
use companion_core::profile::{AppContext, AppIdentity, ProfileResolver};
use companion_core::transport::TransportCode;
use companion_core::transport::TransportTable;
use companion_core::Config;
use companion_platform::{ActionSink, PlatformError, RawTransportEvent};
use crossbeam_channel::{select, Receiver};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Messages from the tray / config watcher to the worker.
#[derive(Debug)]
pub enum Control {
    Reload(Box<Config>),
    SetPaused(bool),
    /// Arm learn mode: the next F13-F24 press is reported over IPC as
    /// `learned` and learn mode disarms itself.
    SetLearn(bool),
    Quit,
}

/// What the tray shows.
#[derive(Debug, Clone, Default)]
pub struct Status {
    pub profile: String,
    pub paused: bool,
    pub last_event: Option<String>,
    pub last_action: Option<String>,
}

pub type SharedStatus = Arc<Mutex<Status>>;

#[derive(Debug)]
pub struct Handled {
    pub event: SemanticEvent,
    pub profile: String,
    pub action: Action,
    pub repeat: u32,
    pub result: Result<(), PlatformError>,
}

pub struct Engine {
    table: TransportTable,
    resolver: ProfileResolver,
    rotary: HashMap<SemanticEvent, RotaryState>,
    paused: bool,
    /// Set by `handle` when a decoded event had no binding anywhere, so the
    /// caller can tell the UI ("detected, but nothing is bound for it").
    unbound: Option<(SemanticEvent, String)>,
    /// Input-level "follow the swipe" defaults for streamed gestures.
    follow: HashMap<SemanticEvent, bool>,
    /// Keys of one streamed gesture closer together than this are one swipe.
    stream_gap: Duration,
    /// When each streamed gesture last produced a key (for collapsing).
    last_stream: HashMap<SemanticEvent, Instant>,
    /// Curve shapes and the repeat cap from `[engine.accel]`.
    accel: AccelSettings,
}

impl Engine {
    pub fn new(cfg: &Config) -> Result<Self> {
        Ok(Self {
            table: cfg.transport_table().context("building transport table")?,
            resolver: cfg.resolver(),
            unbound: None,
            follow: cfg
                .transport
                .iter()
                .map(|(ev, e)| (*ev, e.follow))
                .collect(),
            stream_gap: Duration::from_millis(u64::from(cfg.engine.stream_gap_ms)),
            last_stream: HashMap::new(),
            accel: cfg.engine.accel.clone(),
            rotary: HashMap::new(),
            paused: false,
        })
    }

    pub fn apply(&mut self, cfg: &Config) -> Result<()> {
        self.table = cfg.transport_table().context("building transport table")?;
        self.resolver = cfg.resolver();
        self.rotary.clear();
        self.follow = cfg
            .transport
            .iter()
            .map(|(ev, e)| (*ev, e.follow))
            .collect();
        self.stream_gap = Duration::from_millis(u64::from(cfg.engine.stream_gap_ms));
        self.last_stream.clear();
        self.accel = cfg.engine.accel.clone();
        Ok(())
    }

    pub fn table(&self) -> &TransportTable {
        &self.table
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// Forget dial velocity, e.g. when the foreground app changes.
    pub fn reset_acceleration(&mut self) {
        self.rotary.clear();
    }

    pub fn profile_name(&self, ctx: &AppContext) -> &str {
        &self.resolver.resolve(ctx).name
    }

    /// Whether any profile needs the foreground window title.
    pub fn uses_titles(&self) -> bool {
        self.resolver.uses_titles()
    }

    /// The event `handle` most recently decoded but found no binding for.
    pub fn take_unbound(&mut self) -> Option<(SemanticEvent, String)> {
        self.unbound.take()
    }

    /// Whether this exact key + modifier namespace is one of the configured inputs.
    pub fn is_assigned(&self, code: TransportCode) -> bool {
        self.table.decode(code).is_some()
    }

    /// Decide and act on one raw transport event. Returns `None` when nothing
    /// was executed (key-up, paused, unmapped code, or no binding).
    pub fn handle(
        &mut self,
        raw: RawTransportEvent,
        ctx: &AppContext,
        sink: &mut dyn ActionSink,
    ) -> Option<Handled> {
        self.unbound = None;
        if !raw.pressed {
            return None; // Phase 3 will use key-up for hold / long-press.
        }
        let event = match self.table.decode(raw.code) {
            Some(ev) => ev,
            None => {
                tracing::debug!(?raw.code, "reserved key with unmapped modifier namespace");
                return None;
            }
        };
        if self.paused {
            tracing::debug!(%event, "paused, dropped");
            return None;
        }
        let profile = self.resolver.resolve(ctx);
        let profile_name = profile.name.clone();
        let Some(binding) = self.resolver.binding(ctx, event) else {
            tracing::info!(%event, profile = %profile_name, "no binding");
            self.unbound = Some((event, profile_name));
            return None;
        };
        let profile_name = if self.resolver.god_binding(event).is_some() {
            self.resolver.god_mode().name.clone()
        } else {
            profile_name
        };

        // The firmware's namespace modifier is still held; release it before
        // the action's own chord goes out (docs/PLAN.md §4.3).
        if !raw.code.mods.is_empty() {
            if let Err(e) = sink.release_modifiers(raw.code.mods) {
                tracing::warn!("could not release transport modifiers: {e}");
            }
        }

        // Streamed gestures (a run of keys per swipe): either follow the run,
        // one action per key with no extra acceleration, or collapse it so
        // only the first key of the swipe acts.
        let streamed = event.streams();
        if streamed {
            let follow = binding
                .follow
                .unwrap_or_else(|| self.follow.get(&event).copied().unwrap_or(false));
            if !follow {
                let same_swipe = self
                    .last_stream
                    .get(&event)
                    .is_some_and(|t| raw.at.duration_since(*t) < self.stream_gap);
                self.last_stream.insert(event, raw.at);
                if same_swipe {
                    tracing::debug!(%event, "collapsed: same swipe");
                    return None;
                }
            }
        }

        // Repeat = the binding's multiplier, scaled by the dial's speed curve
        // (never for streamed gestures: the firmware already scales those),
        // capped by engine.accel.max_repeat.
        let repeat = if binding.action.supports_repeat() {
            let speed = if streamed {
                1
            } else {
                let curve = AccelCurve::from_settings(binding.accel, &self.accel);
                self.rotary.entry(event).or_default().tick(raw.at, &curve)
            };
            (speed * binding.multiplier.max(1)).min(self.accel.max_repeat.max(1))
        } else {
            1
        };

        let result = sink.execute(&binding.action, repeat);
        Some(Handled {
            event,
            profile: profile_name,
            action: binding.action.clone(),
            repeat,
            result,
        })
    }
}

fn describe(action: &Action) -> String {
    match action {
        Action::Noop => "nothing".into(),
        Action::Keys { chord, .. } => chord.0.clone(),
        Action::Release => "release held keys".into(),
        Action::Sequence { chords } => chords
            .iter()
            .map(|c| c.0.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        Action::Media { key } => format!("{key:?}"),
        Action::Scroll { direction, lines } => format!("scroll {direction:?} x{lines}"),
        Action::Launch { program, .. } => format!("launch {program}"),
        Action::Command { command } => format!("run {command}"),
    }
}

/// Worker loop. `on_status` is called whenever [`Status`] changes so the tray
/// can refresh; `foreground` supplies the current app identity.
#[allow(clippy::too_many_arguments)]
pub fn run(
    cfg: Config,
    events: Receiver<RawTransportEvent>,
    control: Receiver<Control>,
    status: SharedStatus,
    on_status: impl Fn(),
    foreground: impl Fn() -> Option<AppIdentity>,
    foreground_title: impl Fn() -> Option<String>,
    on_table_change: impl Fn(&TransportTable),
    on_learn: impl Fn(bool),
    ipc: crossbeam_channel::Sender<IpcMessage>,
    mut sink: impl ActionSink,
) -> Result<()> {
    let mut engine = Engine::new(&cfg)?;
    on_table_change(engine.table());
    let _ = ipc.send(IpcMessage::Status {
        profile: engine.profile_name(&AppContext::default()).to_owned(),
        paused: false,
    });

    let update = |f: &dyn Fn(&mut Status)| {
        let snapshot = match status.lock() {
            Ok(mut s) => {
                f(&mut s);
                Some((s.profile.clone(), s.paused))
            }
            Err(_) => None,
        };
        on_status();
        if let Some((profile, paused)) = snapshot {
            let _ = ipc.send(IpcMessage::Status { profile, paused });
        }
    };

    let mut last_app: Option<AppIdentity> = None;
    let mut last_profile = String::new();
    let mut learning = false;
    loop {
        select! {
            recv(control) -> msg => match msg {
                Ok(Control::Reload(new_cfg)) => {
                    match engine.apply(&new_cfg) {
                        Ok(()) => {
                            on_table_change(engine.table());
                            let ctx = AppContext { id: last_app.clone(), title: None };
                            let name = engine.profile_name(&ctx).to_owned();
                            last_profile = name.clone();
                            update(&|s| s.profile = name.clone());
                            tracing::info!(
                                transport_codes = engine.table().len(),
                                "configuration applied"
                            );
                            let _ = ipc.send(IpcMessage::ConfigApplied);
                        }
                        Err(e) => {
                            tracing::warn!("config rejected, keeping previous: {e:#}");
                            let _ = ipc.send(IpcMessage::ConfigRejected {
                                error: format!("{e:#}"),
                            });
                        }
                    }
                }
                Ok(Control::SetPaused(p)) => {
                    engine.set_paused(p);
                    update(&|s| s.paused = p);
                    tracing::info!(paused = p, "pause state changed");
                }
                Ok(Control::SetLearn(on)) => {
                    learning = on;
                    on_learn(on);
                    tracing::info!(learning = on, "learn mode");
                }
                Ok(Control::Quit) | Err(_) => break,
            },
            recv(events) -> ev => {
                let Ok(raw) = ev else { break };
                if learning && raw.pressed {
                    learning = false;
                    on_learn(false);
                    tracing::info!(?raw.code, "learned input");
                    let _ = ipc.send(IpcMessage::Learned {
                        key: raw.code.key,
                        mods: raw.code.mods,
                    });
                }
                if raw.pressed && !raw.held.is_empty() {
                    tracing::info!(?raw.code, held = %raw.held, "modifiers held by the user ignored (down longer than the namespace window)");
                }
                if !raw.reserved || !engine.is_assigned(raw.code) {
                    // The keyboard sent an F-key (or a modifier namespace of one)
                    // that no input uses. Never act on it; tell the UI so the
                    // user can see it arrived and add it under Inputs.
                    if raw.pressed {
                        tracing::info!(?raw.code, reserved = raw.reserved, "unassigned input");
                        let _ = ipc.send(IpcMessage::Unassigned {
                            key: raw.code.key,
                            mods: raw.code.mods,
                        });
                    }
                    continue;
                }
                let app = foreground();
                // Titles are read only when a profile asks for them, only on a
                // dial event, and never persisted (scope Â§20).
                let title = if engine.uses_titles() { foreground_title() } else { None };
                let ctx = AppContext { id: app.clone(), title };
                if app != last_app {
                    tracing::info!(app = ?app, "foreground changed");
                    last_app = app;
                    engine.reset_acceleration();
                }
                let name = engine.profile_name(&ctx).to_owned();
                if name != last_profile {
                    tracing::info!(profile = %name, "profile changed");
                    last_profile = name.clone();
                    update(&|s| s.profile = name.clone());
                }
                if let Some(h) = engine.handle(raw, &ctx, &mut sink) {
                    let action = describe(&h.action);
                    match &h.result {
                        Ok(()) => tracing::info!(event = %h.event, profile = %h.profile, %action, repeat = h.repeat, "executed"),
                        Err(e) => tracing::warn!(event = %h.event, profile = %h.profile, %action, "execute failed: {e}"),
                    }
                    let ev_name = h.event.to_string();
                    let _ = ipc.send(IpcMessage::Event {
                        event: ev_name.clone(),
                        profile: h.profile.clone(),
                        action: action.clone(),
                        repeat: h.repeat,
                    });
                    update(&|s| {
                        s.last_event = Some(ev_name.clone());
                        s.last_action = Some(action.clone());
                    });
                } else if let Some((ev, profile)) = engine.take_unbound() {
                    let _ = ipc.send(IpcMessage::Unbound {
                        event: ev.to_string(),
                        profile,
                    });
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use companion_core::presets::DEFAULT_CONFIG_TOML_WINDOWS;

    /// The tests press the Windows default keys (F24 = dial clockwise), so
    /// they pin the Windows configuration whatever platform runs them.
    fn default_config() -> Config {
        Config::from_toml(DEFAULT_CONFIG_TOML_WINDOWS).unwrap()
    }
    use companion_core::transport::{FunctionKey, Modifiers, TransportCode};
    use std::time::Instant;

    #[derive(Default)]
    struct MockSink {
        calls: Vec<String>,
    }

    impl ActionSink for MockSink {
        fn execute(&mut self, action: &Action, repeat: u32) -> Result<(), PlatformError> {
            self.calls
                .push(format!("exec {} x{repeat}", describe(action)));
            Ok(())
        }
        fn release_modifiers(&mut self, mods: Modifiers) -> Result<(), PlatformError> {
            self.calls.push(format!("release {mods}"));
            Ok(())
        }
    }

    fn press(key: FunctionKey, mods: Modifiers) -> RawTransportEvent {
        press_at(key, mods, Instant::now())
    }

    fn press_at(key: FunctionKey, mods: Modifiers, at: Instant) -> RawTransportEvent {
        RawTransportEvent {
            code: TransportCode { key, mods },
            held: Modifiers::NONE,
            pressed: true,
            at,
            reserved: true,
        }
    }

    #[test]
    fn same_dial_turn_differs_by_app() {
        let mut engine = Engine::new(&default_config()).unwrap();
        let mut sink = MockSink::default();
        let chrome = AppContext::app(AppIdentity::WindowsExe("chrome.exe".into()));

        let h = engine
            .handle(press(FunctionKey::F24, Modifiers::NONE), &chrome, &mut sink)
            .unwrap();
        assert_eq!(h.profile, "Browser");
        assert_eq!(sink.calls, ["exec Ctrl+Tab x1"]);

        // A second later, on the desktop: same key, different action.
        let later = Instant::now() + std::time::Duration::from_secs(1);
        let h = engine
            .handle(
                press_at(FunctionKey::F24, Modifiers::NONE, later),
                &AppContext::default(),
                &mut sink,
            )
            .unwrap();
        assert_eq!(h.profile, "System");
        assert_eq!(sink.calls.last().unwrap(), "exec VolumeUp x1");

        // Fast second detent: Light curve accelerates volume.
        let fast = later + std::time::Duration::from_millis(50);
        let h = engine
            .handle(
                press_at(FunctionKey::F24, Modifiers::NONE, fast),
                &AppContext::default(),
                &mut sink,
            )
            .unwrap();
        assert_eq!(h.repeat, 3);

        // ...but tab switching never accelerates.
        let h = engine
            .handle(
                press_at(FunctionKey::F24, Modifiers::NONE, fast),
                &chrome,
                &mut sink,
            )
            .unwrap();
        assert_eq!(h.repeat, 1);
    }

    #[test]
    fn key_up_and_unreserved_do_nothing() {
        let mut engine = Engine::new(&default_config()).unwrap();
        let mut sink = MockSink::default();
        let mut up = press(FunctionKey::F24, Modifiers::NONE);
        up.pressed = false;
        assert!(engine
            .handle(up, &AppContext::default(), &mut sink)
            .is_none());
        // F13 is not in the bundled transport table.
        assert!(engine
            .handle(
                press(FunctionKey::F13, Modifiers::NONE),
                &AppContext::default(),
                &mut sink
            )
            .is_none());
        assert!(sink.calls.is_empty());
    }

    #[test]
    fn paused_drops_events() {
        let mut engine = Engine::new(&default_config()).unwrap();
        let mut sink = MockSink::default();
        engine.set_paused(true);
        assert!(engine
            .handle(
                press(FunctionKey::F24, Modifiers::NONE),
                &AppContext::default(),
                &mut sink
            )
            .is_none());
        engine.set_paused(false);
        assert!(engine
            .handle(
                press(FunctionKey::F24, Modifiers::NONE),
                &AppContext::default(),
                &mut sink
            )
            .is_some());
    }

    #[test]
    fn streamed_gesture_collapses_unless_told_to_follow() {
        use std::time::Duration;
        // A Tune 2-finger swipe arrives as a run of keys ~10 ms apart.
        let mut cfg = default_config();
        let ev: SemanticEvent = "TUNE_SWIPE_UP_2F".parse().unwrap();
        assert!(ev.streams());
        cfg.transport.insert(
            ev,
            TransportCode {
                key: FunctionKey::F16,
                mods: Modifiers::SHIFT,
            }
            .into(),
        );
        cfg.default_profile.bindings.insert(
            ev,
            companion_core::profile::Binding {
                action: Action::Media {
                    key: companion_core::action::MediaKey::PlayPause,
                },
                accel: Default::default(),
                name: None,
                follow: None,
                multiplier: 1,
            },
        );
        let t0 = Instant::now();
        let run = |engine: &mut Engine, sink: &mut MockSink, start_ms: u64| {
            for i in 0..8u64 {
                let at = t0 + Duration::from_millis(start_ms + i * 10);
                engine.handle(
                    press_at(FunctionKey::F16, Modifiers::SHIFT, at),
                    &AppContext::default(),
                    sink,
                );
            }
        };
        let execs = |sink: &MockSink| sink.calls.iter().filter(|c| c.starts_with("exec")).count();

        // Default: collapse. Eight keys in 70 ms -> one action.
        let mut engine = Engine::new(&cfg).unwrap();
        let mut sink = MockSink::default();
        run(&mut engine, &mut sink, 0);
        assert_eq!(
            execs(&sink),
            1,
            "first key acts, the rest of the swipe is dropped"
        );
        // A second swipe after the gap is a new event.
        run(&mut engine, &mut sink, 1000);
        assert_eq!(execs(&sink), 2);

        // Input default "follow": every key acts, no acceleration multiplier.
        cfg.transport.get_mut(&ev).unwrap().follow = true;
        let mut engine = Engine::new(&cfg).unwrap();
        let mut sink = MockSink::default();
        run(&mut engine, &mut sink, 0);
        assert_eq!(execs(&sink), 8);
        assert!(sink
            .calls
            .iter()
            .filter(|c| c.starts_with("exec"))
            .all(|c| c.ends_with("x1")));

        // The binding overrides the input default in either direction.
        cfg.default_profile.bindings.get_mut(&ev).unwrap().follow = Some(false);
        let mut engine = Engine::new(&cfg).unwrap();
        let mut sink = MockSink::default();
        run(&mut engine, &mut sink, 0);
        assert_eq!(execs(&sink), 1);
    }

    #[test]
    fn modifier_namespace_is_released_before_executing() {
        // Left Touch swipe-left on Shift+F20 -> browser back.
        let mut cfg = default_config();
        let ev: SemanticEvent = "LEFT_TOUCH_SWIPE_LEFT".parse().unwrap();
        cfg.transport.insert(
            ev,
            TransportCode {
                key: FunctionKey::F20,
                mods: Modifiers::SHIFT,
            }
            .into(),
        );
        cfg.default_profile.bindings.insert(
            ev,
            companion_core::profile::Binding {
                action: Action::Keys {
                    chord: companion_core::action::KeyChord("Alt+Left".into()),
                    hold_ms: None,
                },
                accel: Default::default(),
                name: None,
                follow: None,
                multiplier: 1,
            },
        );
        let mut engine = Engine::new(&cfg).unwrap();
        let mut sink = MockSink::default();
        engine
            .handle(
                press(FunctionKey::F20, Modifiers::SHIFT),
                &AppContext::default(),
                &mut sink,
            )
            .unwrap();
        assert_eq!(sink.calls, ["release shift", "exec Alt+Left x1"]);

        // Plain F20 is a different transport code: the Tune swipe, not the Touch one.
        let h = engine
            .handle(
                press(FunctionKey::F20, Modifiers::NONE),
                &AppContext::default(),
                &mut sink,
            )
            .unwrap();
        assert_eq!(h.event.to_string(), "TUNE_SWIPE_LEFT");
        assert_eq!(sink.calls.last().unwrap(), "exec PreviousTrack x1");
    }

    #[test]
    fn reload_swaps_mappings() {
        let mut engine = Engine::new(&default_config()).unwrap();
        let mut sink = MockSink::default();
        let mut cfg = default_config();
        cfg.default_profile
            .bindings
            .get_mut(&"TUNE_CW".parse().unwrap())
            .unwrap()
            .action = Action::Noop;
        engine.apply(&cfg).unwrap();
        let h = engine
            .handle(
                press(FunctionKey::F24, Modifiers::NONE),
                &AppContext::default(),
                &mut sink,
            )
            .unwrap();
        assert_eq!(h.action, Action::Noop);
    }
}
