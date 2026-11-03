use crate::dsp::{Chorus, Reverb};
use crate::midi_player::{Event, Score};
use crate::patch::Patch;
use crate::voice::Voice;

const POLYPHONY: usize = 8;

struct VoiceSlot {
    voice: Voice,
    /// `Some(note)` while a key is being held; cleared after the envelope finishes.
    note: Option<u8>,
    /// Counter used for oldest-wins voice stealing.
    started_at: u64,
}

/// 8-voice polyphonic engine. Mixes active voices each sample, then runs the
/// summed mono signal through chorus → reverb to produce stereo output.
pub struct Engine {
    sample_rate: u32,
    voices: Vec<VoiceSlot>,
    chorus: Chorus,
    reverb: Reverb,
    /// Performance state shared across voices (set by MIDI events).
    mod_wheel: f32,
    pitch_bend_semi: f32,
    sample_counter: u64,
}

impl Engine {
    pub fn new(sample_rate: u32) -> Self {
        let voices = (0..POLYPHONY)
            .map(|_| VoiceSlot {
                voice: Voice::new(sample_rate),
                note: None,
                started_at: 0,
            })
            .collect();
        Self {
            sample_rate,
            voices,
            chorus: Chorus::new(sample_rate),
            reverb: Reverb::new(sample_rate),
            mod_wheel: 0.0,
            pitch_bend_semi: 0.0,
            sample_counter: 0,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Render `score` into an interleaved stereo Vec (L0,R0,L1,R1,...).
    pub fn render_score(&mut self, patch: &Patch, score: &Score) -> Vec<f32> {
        let frames = score.total_samples.max(1) as usize;
        let mut buf = Vec::with_capacity(frames * 2);
        let mut ev_idx = 0;
        let events = &score.events;

        for sample_i in 0..frames as u64 {
            while ev_idx < events.len() && events[ev_idx].sample <= sample_i {
                self.apply_event(patch, events[ev_idx].event);
                ev_idx += 1;
            }

            // Sum all active voices into a mono pre-FX signal.
            let mut mono = 0.0_f32;
            for slot in &mut self.voices {
                if slot.voice.is_active() {
                    mono += slot.voice.next_sample(patch);
                } else if slot.note.is_some() {
                    // Voice just finished its release tail — free the slot for reuse.
                    slot.note = None;
                }
            }
            // Voice headroom: a hard sum of 8 voices can clip, so attenuate.
            // 1/sqrt(8) ≈ 0.354 keeps RMS roughly constant as voice count varies.
            mono *= 0.45;

            // Chorus: mono → stereo.
            let (mut l, mut r) = self.chorus.process(&patch.chorus, mono);
            // Reverb: stereo → stereo.
            let (lr, rr) = self.reverb.process(&patch.reverb, l, r);
            l = lr;
            r = rr;

            buf.push(l);
            buf.push(r);
            self.sample_counter += 1;
        }
        buf
    }

    fn apply_event(&mut self, patch: &Patch, e: Event) {
        match e {
            Event::NoteOn { note, velocity } => self.note_on(patch, note, velocity),
            Event::NoteOff { note } => self.note_off(note),
            Event::PitchBend { semitones } => {
                self.pitch_bend_semi = semitones;
                for slot in &mut self.voices {
                    slot.voice.set_pitch_bend_semi(semitones);
                }
            }
            Event::ModWheel { value } => {
                self.mod_wheel = value;
                for slot in &mut self.voices {
                    slot.voice.set_mod_wheel(value);
                }
            }
        }
    }

    fn note_on(&mut self, patch: &Patch, note: u8, velocity: u8) {
        // Prefer a free slot. Otherwise steal the slot whose voice started earliest.
        let slot_idx = match self.voices.iter().position(|v| v.note.is_none()) {
            Some(i) => i,
            None => self
                .voices
                .iter()
                .enumerate()
                .min_by_key(|(_, v)| v.started_at)
                .map(|(i, _)| i)
                .unwrap_or(0),
        };
        let slot = &mut self.voices[slot_idx];
        slot.voice.set_mod_wheel(self.mod_wheel);
        slot.voice.set_pitch_bend_semi(self.pitch_bend_semi);
        slot.voice.note_on(patch, note, velocity);
        slot.note = Some(note);
        slot.started_at = self.sample_counter;
    }

    fn note_off(&mut self, note: u8) {
        for slot in &mut self.voices {
            if slot.note == Some(note) {
                slot.voice.note_off();
                // Keep slot.note set until the envelope finishes; the render loop clears it.
            }
        }
    }
}
