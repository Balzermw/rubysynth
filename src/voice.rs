use crate::dsp::{Envelope, Filter, Lfo, Oscillator};
use crate::midi_note_to_hz;
use crate::mod_matrix::{ModDest, ModSource};
use crate::patch::Patch;

/// Per-note state. The MVP exposes a single Voice; polyphony is a layer on top.
pub struct Voice {
    osc1: Oscillator,
    osc2: Oscillator,
    filter: Filter,
    env1: Envelope,
    lfo1: Lfo,

    note: f32,
    velocity: f32,
    /// Persistent performance state (not per-voice in real hardware, but trivial to keep here).
    mod_wheel: f32,
    pitch_bend_semi: f32,
}

#[derive(Default, Clone, Copy)]
struct ModAccumulator {
    pitch_semi: f32,
    osc1_pitch_semi: f32,
    osc2_pitch_semi: f32,
    filter_cutoff_hz: f32,
    filter_resonance: f32,
    filter_morph: f32,
    amp_level: f32,
}

impl Voice {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            osc1: Oscillator::new(sample_rate),
            osc2: Oscillator::new(sample_rate),
            filter: Filter::new(sample_rate),
            env1: Envelope::new(sample_rate),
            lfo1: Lfo::new(sample_rate),
            note: 60.0,
            velocity: 0.0,
            mod_wheel: 0.0,
            pitch_bend_semi: 0.0,
        }
    }

    pub fn note_on(&mut self, patch: &Patch, note: u8, velocity: u8) {
        self.note = note as f32;
        self.velocity = velocity as f32 / 127.0;
        self.osc1.reset_phase(0.0);
        self.osc2.reset_phase(0.0);
        self.filter.reset();
        self.lfo1.reset(&patch.lfo1);
        self.env1.note_on();
    }

    pub fn note_off(&mut self) {
        self.env1.note_off();
    }

    pub fn is_active(&self) -> bool {
        self.env1.is_active()
    }

    pub fn set_mod_wheel(&mut self, v: f32) {
        self.mod_wheel = v.clamp(0.0, 1.0);
    }

    pub fn set_pitch_bend_semi(&mut self, v: f32) {
        self.pitch_bend_semi = v;
    }

    /// Render one mono sample.
    pub fn next_sample(&mut self, patch: &Patch) -> f32 {
        // 1. Tick modulation sources first so destinations see their values.
        let env1_val = self.env1.next_sample(&patch.env1);
        // LFO rate can itself be modulated, but for MVP simplicity LFO rate mod
        // only takes effect on the *next* sample's evaluation order. We accumulate
        // mod targeting LFO1 rate after running the LFO with a transient adjusted
        // params copy.
        // For simplicity: compute LFO with route depth applied to a copy.
        let lfo1_rate_mod = self.collect_lfo1_rate_mod(patch, env1_val);
        let mut lfo_params = patch.lfo1;
        lfo_params.rate_hz = (patch.lfo1.rate_hz + lfo1_rate_mod).max(0.001);
        let lfo1_val = self.lfo1.next_sample(&lfo_params);

        // 2. Apply mod matrix to accumulate per-sample destination contributions.
        let acc = self.accumulate_mods(patch, env1_val, lfo1_val);

        // 3. Pitch.
        let base_hz = midi_note_to_hz(self.note + self.pitch_bend_semi + acc.pitch_semi);
        let s1 = self.osc1.next_sample(&patch.osc1, base_hz, acc.osc1_pitch_semi);
        let s2 = self.osc2.next_sample(&patch.osc2, base_hz, acc.osc2_pitch_semi);
        let mixed = s1 + s2;

        // 4. Filter (mod additions get blended in).
        let mut filt_params = patch.filter;
        filt_params.resonance = (patch.filter.resonance + acc.filter_resonance).clamp(0.0, 0.99);
        filt_params.morph = (patch.filter.morph + acc.filter_morph).clamp(0.0, 1.0);
        let filtered = self.filter.process(&filt_params, mixed, acc.filter_cutoff_hz);

        // 5. Amp. Base level * envelope-driven amp level (acc.amp_level holds env*depth).
        // If there's at least one route driving AmpLevel we trust it; otherwise fall back
        // to env1 directly so silence isn't the default.
        let amp_env_factor = if self.has_amp_route(patch) {
            acc.amp_level.max(0.0)
        } else {
            env1_val
        };
        filtered * patch.amp.level * amp_env_factor
    }

    fn collect_lfo1_rate_mod(&self, patch: &Patch, env1_val: f32) -> f32 {
        let mut total = 0.0;
        for r in &patch.mod_routes {
            if r.destination == ModDest::Lfo1Rate {
                total += self.source_value_no_lfo(r.source, env1_val) * r.depth;
            }
        }
        total
    }

    fn accumulate_mods(&self, patch: &Patch, env1_val: f32, lfo1_val: f32) -> ModAccumulator {
        let mut acc = ModAccumulator::default();
        for r in &patch.mod_routes {
            // Lfo1Rate was handled above to avoid feedback within a sample.
            if r.destination == ModDest::Lfo1Rate {
                continue;
            }
            let src = self.source_value(r.source, env1_val, lfo1_val);
            let v = src * r.depth;
            match r.destination {
                ModDest::PitchSemi => acc.pitch_semi += v,
                ModDest::Osc1PitchSemi => acc.osc1_pitch_semi += v,
                ModDest::Osc2PitchSemi => acc.osc2_pitch_semi += v,
                ModDest::FilterCutoffHz => acc.filter_cutoff_hz += v,
                ModDest::FilterResonance => acc.filter_resonance += v,
                ModDest::FilterMorph => acc.filter_morph += v,
                ModDest::AmpLevel => acc.amp_level += v,
                ModDest::Lfo1Rate => {}
            }
        }
        acc
    }

    fn has_amp_route(&self, patch: &Patch) -> bool {
        patch
            .mod_routes
            .iter()
            .any(|r| r.destination == ModDest::AmpLevel)
    }

    fn source_value(&self, src: ModSource, env1_val: f32, lfo1_val: f32) -> f32 {
        match src {
            ModSource::Env1 => env1_val,
            ModSource::Lfo1 => lfo1_val,
            ModSource::Velocity => self.velocity,
            ModSource::ModWheel => self.mod_wheel,
            // Centered on C2 (MIDI 36) like the spec mentions for filter keytrack.
            ModSource::KeyTrack => (self.note - 36.0) / 60.0,
            ModSource::PitchBend => self.pitch_bend_semi / 12.0,
        }
    }

    fn source_value_no_lfo(&self, src: ModSource, env1_val: f32) -> f32 {
        // Used while computing LFO rate mod; LFO1 mod-of-self resolves to 0 this sample.
        match src {
            ModSource::Lfo1 => 0.0,
            other => self.source_value(other, env1_val, 0.0),
        }
    }
}
