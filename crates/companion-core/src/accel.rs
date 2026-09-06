//! Host-side rotary acceleration (scope §11). Measures the interval between
//! successive CW or CCW events and returns a repeat multiplier.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccelPreset {
    #[default]
    None,
    Light,
    Medium,
    Aggressive,
}

/// Interval thresholds (ms, descending) and the multiplier applied when the
/// interval falls below each. The last entry is the cap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccelCurve {
    pub steps: Vec<(u64, u32)>,
}

/// The three preset curves and the overall repeat cap, tunable from the
/// config (`[engine.accel]`). Each curve is a list of `[interval_ms, repeat]`
/// pairs: turning faster than `interval_ms` between detents yields `repeat`
/// actions per detent; the tightest matching pair wins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AccelSettings {
    pub light: Vec<(u64, u32)>,
    pub medium: Vec<(u64, u32)>,
    pub aggressive: Vec<(u64, u32)>,
    /// Upper bound on actions per detent after the binding's multiplier.
    pub max_repeat: u32,
}

impl Default for AccelSettings {
    fn default() -> Self {
        Self {
            light: vec![(150, 2), (80, 3)],
            // Default curve from scope §11.
            medium: vec![(250, 2), (150, 4), (80, 8)],
            aggressive: vec![(250, 3), (150, 6), (80, 12)],
            max_repeat: 32,
        }
    }
}

impl AccelCurve {
    pub fn from_preset(p: AccelPreset) -> Self {
        Self::from_settings(p, &AccelSettings::default())
    }

    pub fn from_settings(p: AccelPreset, s: &AccelSettings) -> Self {
        let steps = match p {
            AccelPreset::None => vec![],
            AccelPreset::Light => s.light.clone(),
            AccelPreset::Medium => s.medium.clone(),
            AccelPreset::Aggressive => s.aggressive.clone(),
        };
        Self { steps }
    }

    pub fn multiplier(&self, interval: Duration) -> u32 {
        let ms = interval.as_millis() as u64;
        self.steps
            .iter()
            .filter(|(threshold, _)| ms < *threshold)
            .map(|(_, mult)| *mult)
            .max()
            .unwrap_or(1)
    }
}

/// Per-direction state. Keep one per (profile, event) that has acceleration.
#[derive(Debug, Default)]
pub struct RotaryState {
    last: Option<Instant>,
}

impl RotaryState {
    pub fn tick(&mut self, now: Instant, curve: &AccelCurve) -> u32 {
        let mult = match self.last {
            Some(prev) => curve.multiplier(now.saturating_duration_since(prev)),
            None => 1,
        };
        self.last = Some(now);
        mult
    }

    pub fn reset(&mut self) {
        self.last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medium_curve_matches_scope_table() {
        let c = AccelCurve::from_preset(AccelPreset::Medium);
        assert_eq!(c.multiplier(Duration::from_millis(300)), 1);
        assert_eq!(c.multiplier(Duration::from_millis(200)), 2);
        assert_eq!(c.multiplier(Duration::from_millis(100)), 4);
        assert_eq!(c.multiplier(Duration::from_millis(40)), 8);
    }

    #[test]
    fn settings_override_the_curve() {
        let s = AccelSettings {
            medium: vec![(500, 10)],
            ..AccelSettings::default()
        };
        let c = AccelCurve::from_settings(AccelPreset::Medium, &s);
        assert_eq!(c.multiplier(Duration::from_millis(400)), 10);
        assert_eq!(c.multiplier(Duration::from_millis(600)), 1);
        assert_eq!(AccelSettings::default().max_repeat, 32);
    }

    #[test]
    fn none_never_accelerates() {
        let c = AccelCurve::from_preset(AccelPreset::None);
        assert_eq!(c.multiplier(Duration::from_millis(1)), 1);
    }

    #[test]
    fn first_tick_is_one() {
        let c = AccelCurve::from_preset(AccelPreset::Medium);
        let mut s = RotaryState::default();
        let t0 = Instant::now();
        assert_eq!(s.tick(t0, &c), 1);
        assert_eq!(s.tick(t0 + Duration::from_millis(50), &c), 8);
        assert_eq!(s.tick(t0 + Duration::from_millis(1000), &c), 1);
    }
}
