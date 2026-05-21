use serde::{Deserialize, Serialize};

/// DAHDSR envelope (delay, attack, hold, decay, sustain, release).
/// All times in seconds; sustain is a level 0..=1.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EnvelopeParams {
    pub delay: f32,
    pub attack: f32,
    pub hold: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Default for EnvelopeParams {
    fn default() -> Self {
        Self {
            delay: 0.0,
            attack: 0.01,
            hold: 0.0,
            decay: 0.2,
            sustain: 0.7,
            release: 0.3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Idle,
    Delay,
    Attack,
    Hold,
    Decay,
    Sustain,
    Release,
}

pub struct Envelope {
    sample_rate: f32,
    stage: Stage,
    /// Seconds elapsed in current stage.
    t_in_stage: f32,
    /// Last output level (for smooth transitions out of release).
    last_level: f32,
    /// Level captured at the moment note_off was received.
    release_start_level: f32,
}

impl Envelope {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: sample_rate as f32,
            stage: Stage::Idle,
            t_in_stage: 0.0,
            last_level: 0.0,
            release_start_level: 0.0,
        }
    }

    pub fn note_on(&mut self) {
        self.stage = Stage::Delay;
        self.t_in_stage = 0.0;
        self.last_level = 0.0;
    }

    pub fn note_off(&mut self) {
        if self.stage != Stage::Idle && self.stage != Stage::Release {
            self.release_start_level = self.last_level;
            self.stage = Stage::Release;
            self.t_in_stage = 0.0;
        }
    }

    pub fn is_active(&self) -> bool {
        self.stage != Stage::Idle
    }

    pub fn next_sample(&mut self, p: &EnvelopeParams) -> f32 {
        let dt = 1.0 / self.sample_rate;
        let level = match self.stage {
            Stage::Idle => 0.0,
            Stage::Delay => {
                if self.t_in_stage >= p.delay {
                    self.advance(Stage::Attack);
                }
                0.0
            }
            Stage::Attack => {
                if p.attack <= 0.0 {
                    self.advance(Stage::Hold);
                    1.0
                } else {
                    let v = (self.t_in_stage / p.attack).clamp(0.0, 1.0);
                    if v >= 1.0 {
                        self.advance(Stage::Hold);
                    }
                    v
                }
            }
            Stage::Hold => {
                if self.t_in_stage >= p.hold {
                    self.advance(Stage::Decay);
                }
                1.0
            }
            Stage::Decay => {
                if p.decay <= 0.0 {
                    self.advance(Stage::Sustain);
                    p.sustain
                } else {
                    let t = (self.t_in_stage / p.decay).clamp(0.0, 1.0);
                    // Exponential-ish: blend 1.0 → sustain.
                    let v = 1.0 + (p.sustain - 1.0) * t;
                    if t >= 1.0 {
                        self.advance(Stage::Sustain);
                    }
                    v
                }
            }
            Stage::Sustain => p.sustain,
            Stage::Release => {
                if p.release <= 0.0 {
                    self.stage = Stage::Idle;
                    0.0
                } else {
                    let t = (self.t_in_stage / p.release).clamp(0.0, 1.0);
                    let v = self.release_start_level * (1.0 - t);
                    if t >= 1.0 {
                        self.stage = Stage::Idle;
                    }
                    v
                }
            }
        };

        self.t_in_stage += dt;
        self.last_level = level;
        level
    }

    fn advance(&mut self, next: Stage) {
        self.stage = next;
        self.t_in_stage = 0.0;
    }
}
