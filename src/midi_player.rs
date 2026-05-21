use anyhow::{Context, Result};
use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

#[derive(Debug, Clone, Copy)]
pub enum Event {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
    PitchBend { semitones: f32 },
    ModWheel { value: f32 },
}

#[derive(Debug, Clone, Copy)]
pub struct TimedEvent {
    pub sample: u64,
    pub event: Event,
}

pub struct Score {
    pub events: Vec<TimedEvent>,
    pub total_samples: u64,
}

impl Score {
    /// Build a one-note score for click-to-play previewing.
    pub fn single_note(
        note: u8,
        velocity: u8,
        hold_samples: u64,
        tail_samples: u64,
    ) -> Score {
        Score {
            events: vec![
                TimedEvent {
                    sample: 0,
                    event: Event::NoteOn { note, velocity },
                },
                TimedEvent {
                    sample: hold_samples,
                    event: Event::NoteOff { note },
                },
            ],
            total_samples: hold_samples + tail_samples,
        }
    }
}

const DEFAULT_TEMPO_US_PER_QN: u32 = 500_000; // 120 BPM
const PITCH_BEND_RANGE_SEMI: f32 = 2.0;

pub fn load_midi(bytes: &[u8], sample_rate: u32, tail_seconds: f32) -> Result<Score> {
    let smf = Smf::parse(bytes).context("parsing MIDI file")?;
    let ticks_per_qn = match smf.header.timing {
        Timing::Metrical(t) => t.as_int() as u32,
        Timing::Timecode(_, _) => {
            anyhow::bail!("SMPTE timecode MIDI not supported in MVP — re-export as PPQN")
        }
    };

    // Merge all tracks by absolute tick.
    let mut merged: Vec<(u64, TrackEventKind)> = Vec::new();
    for track in &smf.tracks {
        let mut abs_tick: u64 = 0;
        for ev in track {
            abs_tick += ev.delta.as_int() as u64;
            merged.push((abs_tick, ev.kind));
        }
    }
    merged.sort_by_key(|(t, _)| *t);

    // Walk events, tracking tempo changes to convert tick deltas to samples.
    let mut tempo_us_per_qn: u32 = DEFAULT_TEMPO_US_PER_QN;
    let mut last_tick: u64 = 0;
    let mut sample_pos: f64 = 0.0;
    let sr = sample_rate as f64;
    let mut out: Vec<TimedEvent> = Vec::new();

    for (tick, kind) in merged {
        if tick > last_tick {
            let dt_ticks = (tick - last_tick) as f64;
            let secs = dt_ticks * (tempo_us_per_qn as f64 / 1_000_000.0) / ticks_per_qn as f64;
            sample_pos += secs * sr;
            last_tick = tick;
        }
        let sample = sample_pos.round() as u64;

        match kind {
            TrackEventKind::Meta(MetaMessage::Tempo(t)) => {
                tempo_us_per_qn = t.as_int();
            }
            TrackEventKind::Midi { message, .. } => match message {
                MidiMessage::NoteOn { key, vel } => {
                    let vel = vel.as_int();
                    if vel == 0 {
                        out.push(TimedEvent {
                            sample,
                            event: Event::NoteOff { note: key.as_int() },
                        });
                    } else {
                        out.push(TimedEvent {
                            sample,
                            event: Event::NoteOn {
                                note: key.as_int(),
                                velocity: vel,
                            },
                        });
                    }
                }
                MidiMessage::NoteOff { key, .. } => {
                    out.push(TimedEvent {
                        sample,
                        event: Event::NoteOff { note: key.as_int() },
                    });
                }
                MidiMessage::PitchBend { bend } => {
                    // bend.as_f32() is in -1.0..=1.0
                    out.push(TimedEvent {
                        sample,
                        event: Event::PitchBend {
                            semitones: bend.as_f32() * PITCH_BEND_RANGE_SEMI,
                        },
                    });
                }
                MidiMessage::Controller { controller, value } => {
                    if controller.as_int() == 1 {
                        out.push(TimedEvent {
                            sample,
                            event: Event::ModWheel {
                                value: value.as_int() as f32 / 127.0,
                            },
                        });
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    let tail_samples = (tail_seconds * sample_rate as f32) as u64;
    let total_samples = sample_pos.round() as u64 + tail_samples;
    Ok(Score {
        events: out,
        total_samples,
    })
}
