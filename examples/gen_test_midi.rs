// Generates a tiny test MIDI file with a short melody.
// Usage: cargo run --example gen_test_midi -- patches/test_melody.mid

use std::env;
use std::path::PathBuf;

use midly::num::{u15, u24, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("out/test_melody.mid"));
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let ticks_per_qn: u16 = 480;
    let header = Header {
        format: Format::SingleTrack,
        timing: Timing::Metrical(u15::from(ticks_per_qn)),
    };

    let mut track: Track = Vec::new();

    // 120 BPM
    track.push(TrackEvent {
        delta: u28::from(0),
        kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::from(500_000))),
    });

    // Simple ascending arpeggio: C4 E4 G4 C5, each a quarter, then C5 sustains a half.
    let ch = u4::from(0);
    let velocity = u7::from(96);
    let off_velocity = u7::from(64);
    let notes = [60u8, 64, 67, 72];
    let quarter = ticks_per_qn as u32;

    for (i, n) in notes.iter().enumerate() {
        let delta = if i == 0 { 0 } else { 0 };
        track.push(TrackEvent {
            delta: u28::from(delta),
            kind: TrackEventKind::Midi {
                channel: ch,
                message: MidiMessage::NoteOn {
                    key: u7::from(*n),
                    vel: velocity,
                },
            },
        });
        track.push(TrackEvent {
            delta: u28::from(quarter),
            kind: TrackEventKind::Midi {
                channel: ch,
                message: MidiMessage::NoteOff {
                    key: u7::from(*n),
                    vel: off_velocity,
                },
            },
        });
    }

    // Final long C5 (half note).
    track.push(TrackEvent {
        delta: u28::from(0),
        kind: TrackEventKind::Midi {
            channel: ch,
            message: MidiMessage::NoteOn {
                key: u7::from(72),
                vel: velocity,
            },
        },
    });
    track.push(TrackEvent {
        delta: u28::from(quarter * 2),
        kind: TrackEventKind::Midi {
            channel: ch,
            message: MidiMessage::NoteOff {
                key: u7::from(72),
                vel: off_velocity,
            },
        },
    });

    track.push(TrackEvent {
        delta: u28::from(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });

    let smf = Smf {
        header,
        tracks: vec![track],
    };

    let mut bytes = Vec::new();
    smf.write(&mut bytes)?;
    std::fs::write(&out, &bytes)?;
    println!("wrote {} ({} bytes)", out.display(), bytes.len());
    Ok(())
}
