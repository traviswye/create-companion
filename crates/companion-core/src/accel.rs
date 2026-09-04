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

impl AccelCurve {
    pub fn from_preset(p: AccelPreset) -> Self {
        let steps = match p {
            AccelPreset::None => vec![],
            AccelPreset::Light => vec![(150, 2), (80, 3)],
            // Default curve from scope §11.
            AccelPreset::Medium => vec![(250, 2), (150, 4), (80, 8)],
            AccelPreset::Aggressive => vec![(250, 3), (150, 6), (80, 12)],
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
