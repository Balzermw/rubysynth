use serde::{Deserialize, Serialize};

/// Schroeder-style reverb: 4 parallel comb filters with feedback damping,
/// followed by 2 series allpass filters per channel. Right channel uses a small
/// offset on every delay length so the two sides decorrelate without two
/// independent reverb engines.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ReverbParams {
    /// Tail length / room size. 0..=1; maps to comb feedback.
    pub size: f32,
    /// High-frequency damping in the comb feedback loop. 0..=1.
    pub damping: f32,
    /// Wet/dry mix. 0..=1.
    pub mix: f32,
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            size: 0.6,
            damping: 0.4,
            mix: 0.25,
        }
    }
}

// Base comb delay lengths in samples (at 44.1 kHz). The classic Freeverb
// numbers; we scale at construction if the sample rate differs.
const COMB_LENGTHS_44K: [usize; 4] = [1116, 1356, 1422, 1617];
const ALLPASS_LENGTHS_44K: [usize; 2] = [556, 341];
const STEREO_OFFSET_SAMPLES: usize = 23;

struct Comb {
    buf: Vec<f32>,
    pos: usize,
    /// One-pole lowpass state for the damping filter in the feedback path.
    lp: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(1)],
            pos: 0,
            lp: 0.0,
        }
    }
    fn process(&mut self, input: f32, feedback: f32, damp: f32) -> f32 {
        let out = self.buf[self.pos];
        // Lowpass the feedback so the tail darkens over time (high-freq damping).
        self.lp = out * (1.0 - damp) + self.lp * damp;
        self.buf[self.pos] = input + self.lp * feedback;
        self.pos = (self.pos + 1) % self.buf.len();
        out
    }
}

struct Allpass {
    buf: Vec<f32>,
    pos: usize,
}

impl Allpass {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(1)],
            pos: 0,
        }
    }
    fn process(&mut self, input: f32) -> f32 {
        let g = 0.5;
        let buf_out = self.buf[self.pos];
        let output = -input + buf_out;
        self.buf[self.pos] = input + buf_out * g;
        self.pos = (self.pos + 1) % self.buf.len();
        output
    }
}

pub struct Reverb {
    combs_l: Vec<Comb>,
    combs_r: Vec<Comb>,
    aps_l: Vec<Allpass>,
    aps_r: Vec<Allpass>,
}

fn scale_delay(d: usize, sample_rate: u32) -> usize {
    ((d as f32) * (sample_rate as f32 / 44_100.0)).round() as usize
}

impl Reverb {
    pub fn new(sample_rate: u32) -> Self {
        let mut combs_l = Vec::with_capacity(COMB_LENGTHS_44K.len());
        let mut combs_r = Vec::with_capacity(COMB_LENGTHS_44K.len());
        for &d in &COMB_LENGTHS_44K {
            combs_l.push(Comb::new(scale_delay(d, sample_rate)));
            combs_r.push(Comb::new(scale_delay(d + STEREO_OFFSET_SAMPLES, sample_rate)));
        }
        let mut aps_l = Vec::with_capacity(ALLPASS_LENGTHS_44K.len());
        let mut aps_r = Vec::with_capacity(ALLPASS_LENGTHS_44K.len());
        for &d in &ALLPASS_LENGTHS_44K {
            aps_l.push(Allpass::new(scale_delay(d, sample_rate)));
            aps_r.push(Allpass::new(scale_delay(d + STEREO_OFFSET_SAMPLES, sample_rate)));
        }
        Self {
            combs_l,
            combs_r,
            aps_l,
            aps_r,
        }
    }

    pub fn process(&mut self, params: &ReverbParams, l: f32, r: f32) -> (f32, f32) {
        // Map size 0..1 → feedback 0.7..0.96 for a usable range of tail times.
        let feedback = 0.70 + 0.26 * params.size.clamp(0.0, 1.0);
        let damping = params.damping.clamp(0.0, 1.0);
        // Lower the input level so the recursion stays stable as size grows.
        let drive = 0.18;

        let mut wet_l = 0.0;
        let mut wet_r = 0.0;
        let in_l = l * drive;
        let in_r = r * drive;
        for c in &mut self.combs_l {
            wet_l += c.process(in_l, feedback, damping);
        }
        for c in &mut self.combs_r {
            wet_r += c.process(in_r, feedback, damping);
        }
        for a in &mut self.aps_l {
            wet_l = a.process(wet_l);
        }
        for a in &mut self.aps_r {
            wet_r = a.process(wet_r);
        }

        let mix = params.mix.clamp(0.0, 1.0);
        let out_l = l * (1.0 - mix) + wet_l * mix;
        let out_r = r * (1.0 - mix) + wet_r * mix;
        (out_l, out_r)
    }
}
