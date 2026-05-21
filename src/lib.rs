pub mod dsp;
pub mod engine;
pub mod midi_player;
pub mod mod_matrix;
pub mod patch;
pub mod voice;

pub const SAMPLE_RATE: u32 = 44_100;

pub fn midi_note_to_hz(note: f32) -> f32 {
    440.0 * (2.0_f32).powf((note - 69.0) / 12.0)
}
