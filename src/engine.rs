use crate::midi_player::{Event, Score};
use crate::patch::Patch;
use crate::voice::Voice;

/// MVP engine: one voice, retriggered per note-on. Notes overlap → last note wins.
pub struct Engine {
    sample_rate: u32,
    voice: Voice,
    /// Track of the currently held note so a NoteOff for a different note doesn't kill it.
    held_note: Option<u8>,
}

impl Engine {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            voice: Voice::new(sample_rate),
            held_note: None,
        }
    }

    pub fn render_score(&mut self, patch: &Patch, score: &Score) -> Vec<f32> {
        let total = score.total_samples.max(1) as usize;
        let mut buf = Vec::with_capacity(total);
        let mut ev_idx = 0;
        let events = &score.events;

        for sample_i in 0..total as u64 {
            while ev_idx < events.len() && events[ev_idx].sample <= sample_i {
                self.apply_event(patch, events[ev_idx].event);
                ev_idx += 1;
            }
            buf.push(self.voice.next_sample(patch));
        }
        buf
    }

    fn apply_event(&mut self, patch: &Patch, e: Event) {
        match e {
            Event::NoteOn { note, velocity } => {
                self.voice.note_on(patch, note, velocity);
                self.held_note = Some(note);
            }
            Event::NoteOff { note } => {
                if self.held_note == Some(note) {
                    self.voice.note_off();
                    self.held_note = None;
                }
            }
            Event::PitchBend { semitones } => self.voice.set_pitch_bend_semi(semitones),
            Event::ModWheel { value } => self.voice.set_mod_wheel(value),
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}
