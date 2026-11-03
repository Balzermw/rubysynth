use serde::{Deserialize, Serialize};

use crate::dsp::{ChorusParams, EnvelopeParams, FilterParams, LfoParams, OscillatorParams, ReverbParams};
use crate::mod_matrix::ModRoute;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmpParams {
    /// Master voice level 0..=1.
    pub level: f32,
}

impl Default for AmpParams {
    fn default() -> Self {
        Self { level: 0.85 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Patch {
    pub name: String,
    pub osc1: OscillatorParams,
    pub osc2: OscillatorParams,
    pub filter: FilterParams,
    pub amp: AmpParams,
    pub env1: EnvelopeParams,
    pub lfo1: LfoParams,
    pub chorus: ChorusParams,
    pub reverb: ReverbParams,
    pub mod_routes: Vec<ModRoute>,
}

impl Default for Patch {
    fn default() -> Self {
        use crate::dsp::Waveform;
        use crate::mod_matrix::{ModDest, ModSource};

        Self {
            name: "Default Lead".to_string(),
            osc1: OscillatorParams {
                waveform: Waveform::Saw,
                detune_semi: 0.0,
                detune_cents: -7.0,
                level: 0.6,
            },
            osc2: OscillatorParams {
                waveform: Waveform::Saw,
                detune_semi: 0.0,
                detune_cents: 7.0,
                level: 0.6,
            },
            filter: FilterParams {
                cutoff_hz: 1400.0,
                resonance: 0.25,
                morph: 0.0,
                drive: 1.4,
            },
            amp: AmpParams::default(),
            env1: EnvelopeParams {
                delay: 0.0,
                attack: 0.005,
                hold: 0.0,
                decay: 0.25,
                sustain: 0.75,
                release: 0.35,
            },
            lfo1: LfoParams::default(),
            chorus: ChorusParams::default(),
            reverb: ReverbParams::default(),
            mod_routes: vec![
                ModRoute {
                    source: ModSource::Env1,
                    destination: ModDest::AmpLevel,
                    depth: 1.0,
                },
                ModRoute {
                    source: ModSource::Env1,
                    destination: ModDest::FilterCutoffHz,
                    depth: 1800.0,
                },
                ModRoute {
                    source: ModSource::Velocity,
                    destination: ModDest::FilterCutoffHz,
                    depth: 600.0,
                },
            ],
        }
    }
}
