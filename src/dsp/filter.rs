use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// Chamberlin state-variable filter with continuous LP→BP→HP morph.
/// This stands in for Rubysynth's Filter 2 (the 2-pole SVF with Morph).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FilterParams {
    /// Cutoff in Hz.
    pub cutoff_hz: f32,
    /// Resonance 0..=1 (mapped to damping internally).
    pub resonance: f32,
    /// 0.0 = LP, 0.5 = BP, 1.0 = HP.
    pub morph: f32,
    /// Saturation drive in 1.0..=4.0 (pre-filter soft clip).
    pub drive: f32,
}

impl Default for FilterParams {
    fn default() -> Self {
        Self {
            cutoff_hz: 1200.0,
            resonance: 0.2,
            morph: 0.0,
            drive: 1.0,
        }
    }
}

pub struct Filter {
    sample_rate: f32,
    low: f32,
    band: f32,
}

impl Filter {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: sample_rate as f32,
            low: 0.0,
            band: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.low = 0.0;
        self.band = 0.0;
    }

    /// One-pole-per-sample SVF. `cutoff_mod` is added to cutoff in Hz; clamped sanely.
    pub fn process(&mut self, params: &FilterParams, input: f32, cutoff_mod_hz: f32) -> f32 {
        let cutoff = (params.cutoff_hz + cutoff_mod_hz).clamp(20.0, self.sample_rate * 0.45);
        // f coefficient: 2*sin(pi*fc/fs). Standard SVF.
        let f = 2.0 * (PI * cutoff / self.sample_rate).sin();
        // Map resonance 0..1 to damping q: more resonance = less damping.
        let q = (1.0 - params.resonance.clamp(0.0, 0.99)).max(0.02);

        let driven = if params.drive > 1.0 {
            (input * params.drive).tanh()
        } else {
            input
        };

        // SVF update (single pass; for stability at high cutoff one could oversample 2x).
        self.low += f * self.band;
        let high = driven - self.low - q * self.band;
        self.band += f * high;
        let low = self.low;
        let band = self.band;

        // LP-only for the MVP. The morph machinery is still here (band/high
        // are computed for future use) but we always tap the LP output.
        let _ = (band, high, params.morph);
        low
    }
}
