use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

/// Juno-style stereo chorus: two short delay lines modulated by LFOs 90° apart
/// to produce a wide stereo image from a mono source. Tuned for the gentle,
/// shimmery character of the Juno-60/106 chorus circuit.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ChorusParams {
    /// LFO rate in Hz. Juno I ≈ 0.5, Juno II ≈ 0.8.
    pub rate_hz: f32,
    /// Peak delay modulation depth in milliseconds (typ. 2–6 ms).
    pub depth_ms: f32,
    /// Wet/dry mix, 0..=1.
    pub mix: f32,
}

impl Default for ChorusParams {
    fn default() -> Self {
        Self {
            rate_hz: 0.6,
            depth_ms: 3.0,
            mix: 0.4,
        }
    }
}

pub struct Chorus {
    sample_rate: f32,
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    write_pos: usize,
    lfo_phase: f32,
    /// One-pole highpass state, applied to the wet signal to keep DC and rumble out.
    hp_state_l: f32,
    hp_state_r: f32,
}

const BASE_DELAY_MS: f32 = 7.5;
const MAX_DELAY_MS: f32 = 30.0;
const HP_CUTOFF_HZ: f32 = 80.0;

impl Chorus {
    pub fn new(sample_rate: u32) -> Self {
        let max_samples = (sample_rate as f32 * MAX_DELAY_MS / 1000.0).ceil() as usize + 4;
        Self {
            sample_rate: sample_rate as f32,
            buf_l: vec![0.0; max_samples],
            buf_r: vec![0.0; max_samples],
            write_pos: 0,
            lfo_phase: 0.0,
            hp_state_l: 0.0,
            hp_state_r: 0.0,
        }
    }

    pub fn process(&mut self, params: &ChorusParams, input: f32) -> (f32, f32) {
        let base = BASE_DELAY_MS * 0.001 * self.sample_rate;
        let depth = params.depth_ms.max(0.0) * 0.001 * self.sample_rate;

        let phase_a = self.lfo_phase;
        let phase_b = (self.lfo_phase + 0.25).rem_euclid(1.0); // +90°
        let lfo_a = (phase_a * TAU).sin();
        let lfo_b = (phase_b * TAU).sin();

        let inc = params.rate_hz / self.sample_rate;
        self.lfo_phase = (self.lfo_phase + inc).rem_euclid(1.0);

        // Each side gets its own modulated delay length, in samples.
        let delay_l = base + depth * lfo_a;
        let delay_r = base + depth * lfo_b;

        // Write current sample into both lines, then read with fractional delay.
        let len = self.buf_l.len();
        self.buf_l[self.write_pos] = input;
        self.buf_r[self.write_pos] = input;
        let mut wet_l = read_interp(&self.buf_l, self.write_pos, delay_l);
        let mut wet_r = read_interp(&self.buf_r, self.write_pos, delay_r);
        self.write_pos = (self.write_pos + 1) % len;

        // Subtle highpass on the wet signal keeps the bass clean.
        let hp_coef = (-TAU * HP_CUTOFF_HZ / self.sample_rate).exp();
        let hp_l = wet_l - self.hp_state_l;
        let hp_r = wet_r - self.hp_state_r;
        self.hp_state_l = self.hp_state_l * hp_coef + wet_l * (1.0 - hp_coef);
        self.hp_state_r = self.hp_state_r * hp_coef + wet_r * (1.0 - hp_coef);
        wet_l = hp_l;
        wet_r = hp_r;

        let mix = params.mix.clamp(0.0, 1.0);
        // Equal-power-ish blend; dry retains its mono image, wet is stereo.
        let l = input * (1.0 - mix * 0.5) + wet_l * mix;
        let r = input * (1.0 - mix * 0.5) + wet_r * mix;
        (l, r)
    }
}

/// Read `buf` at `write_pos - delay` samples ago, with linear interpolation.
fn read_interp(buf: &[f32], write_pos: usize, delay_samples: f32) -> f32 {
    let len = buf.len() as f32;
    let read = (write_pos as f32 - delay_samples).rem_euclid(len);
    let i0 = read as usize;
    let i1 = (i0 + 1) % buf.len();
    let frac = read - i0 as f32;
    buf[i0] * (1.0 - frac) + buf[i1] * frac
}
