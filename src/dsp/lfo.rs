use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LfoShape {
    Sine,
    Triangle,
    Saw,
    Square,
    SampleHold,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LfoParams {
    pub shape: LfoShape,
    pub rate_hz: f32,
    /// 0..=1 starting phase.
    pub phase_offset: f32,
    /// Bipolar output amplitude 0..=1 (1.0 = full -1..1).
    pub depth: f32,
}

impl Default for LfoParams {
    fn default() -> Self {
        Self {
            shape: LfoShape::Sine,
            rate_hz: 5.0,
            phase_offset: 0.0,
            depth: 1.0,
        }
    }
}

pub struct Lfo {
    sample_rate: f32,
    phase: f32,
    sh_value: f32,
    sh_rng_state: u32,
}

impl Lfo {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: sample_rate as f32,
            phase: 0.0,
            sh_value: 0.0,
            sh_rng_state: 0xCAFE_F00D,
        }
    }

    pub fn reset(&mut self, params: &LfoParams) {
        self.phase = params.phase_offset.rem_euclid(1.0);
        self.sh_value = 0.0;
    }

    pub fn next_sample(&mut self, params: &LfoParams) -> f32 {
        let inc = params.rate_hz / self.sample_rate;
        let prev_phase = self.phase;
        self.phase = (self.phase + inc).rem_euclid(1.0);

        let raw = match params.shape {
            LfoShape::Sine => (prev_phase * TAU).sin(),
            LfoShape::Triangle => 4.0 * (prev_phase - 0.5).abs() - 1.0,
            LfoShape::Saw => 2.0 * prev_phase - 1.0,
            LfoShape::Square => {
                if prev_phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            LfoShape::SampleHold => {
                // Re-sample when phase wraps.
                if self.phase < prev_phase {
                    // xorshift32
                    let mut x = self.sh_rng_state;
                    x ^= x << 13;
                    x ^= x >> 17;
                    x ^= x << 5;
                    self.sh_rng_state = x;
                    self.sh_value = (x as f32 / u32::MAX as f32) * 2.0 - 1.0;
                }
                self.sh_value
            }
        };

        raw * params.depth
    }
}
