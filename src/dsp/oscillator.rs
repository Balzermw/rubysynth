use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Waveform {
    Sine,
    Triangle,
    Saw,
    Square,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OscillatorParams {
    pub waveform: Waveform,
    /// Semitone offset from incoming pitch.
    pub detune_semi: f32,
    /// Fine detune in cents (-100..=100).
    pub detune_cents: f32,
    /// Output level 0..=1.
    pub level: f32,
}

impl Default for OscillatorParams {
    fn default() -> Self {
        Self {
            waveform: Waveform::Saw,
            detune_semi: 0.0,
            detune_cents: 0.0,
            level: 0.7,
        }
    }
}

pub struct Oscillator {
    phase: f32,
    sample_rate: f32,
}

impl Oscillator {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            phase: 0.0,
            sample_rate: sample_rate as f32,
        }
    }

    pub fn reset_phase(&mut self, phase: f32) {
        self.phase = phase.rem_euclid(1.0);
    }

    /// Render one sample at the given base frequency (Hz).
    /// `pitch_mod_semi` is a per-sample modulation in semitones (e.g. from an LFO/env).
    pub fn next_sample(&mut self, params: &OscillatorParams, base_hz: f32, pitch_mod_semi: f32) -> f32 {
        let total_semi = params.detune_semi + (params.detune_cents / 100.0) + pitch_mod_semi;
        let freq = base_hz * (2.0_f32).powf(total_semi / 12.0);

        let phase_inc = freq / self.sample_rate;
        let p = self.phase;
        self.phase = (self.phase + phase_inc).rem_euclid(1.0);

        let s = match params.waveform {
            Waveform::Sine => (p * TAU).sin(),
            Waveform::Triangle => {
                // 4|p-0.5| - 1, ranges -1..1
                4.0 * (p - 0.5).abs() - 1.0
            }
            Waveform::Saw => 2.0 * p - 1.0,
            Waveform::Square => {
                if p < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
        };

        s * params.level
    }
}
