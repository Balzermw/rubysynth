use serde::{Deserialize, Serialize};

/// All available modulation sources for the MVP.
/// Per-spec the full synth has many more (env2-5, lfo2-5, ribbon, CC, etc).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ModSource {
    Env1,
    Lfo1,
    Velocity,
    ModWheel,
    KeyTrack,
    PitchBend,
}

/// Destinations the MVP can address. Each is applied as a signed depth times the source value.
/// Units differ by destination — the voice interprets each route in its native units.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ModDest {
    /// Adds to oscillator pitch in semitones (applied to both oscs).
    PitchSemi,
    /// Adds to oscillator 1 pitch in semitones (independent of osc2).
    Osc1PitchSemi,
    /// Adds to oscillator 2 pitch in semitones.
    Osc2PitchSemi,
    /// Adds to filter cutoff in Hz.
    FilterCutoffHz,
    /// Adds to filter resonance (0..1 space).
    FilterResonance,
    /// Adds to filter morph 0..1.
    FilterMorph,
    /// Multiplies amp (0..1; combined with envelope and base level).
    AmpLevel,
    /// Adds to LFO1 rate in Hz.
    Lfo1Rate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ModRoute {
    pub source: ModSource,
    pub destination: ModDest,
    pub depth: f32,
}
