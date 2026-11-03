// Generates a small MIDI file with overlapping notes (chords) to test polyphony.
// Usage: cargo run --example gen_chord_midi -- out/test_chords.mid

use std::env;
use std::path::PathBuf;

use midly::num::{u15, u24, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("out/test_chords.mid"));
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let ticks_per_qn: u16 = 480;
    let header = Header {
        format: Format::SingleTrack,
        timing: Timing::Metrical(u15::from(ticks_per_qn)),
    };

    let ch = u4::from(0);
    let v = u7::from(96);
    let v_off = u7::from(64);
    let quarter = ticks_per_qn as u32;

    let mut track: Track = Vec::new();
    track.push(TrackEvent {
        delta: u28::from(0),
        kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::from(500_000))),
    });

    fn note_on(track: &mut Track, ch: u4, key: u8, vel: u7, dt: u32) {
        track.push(TrackEvent {
            delta: u28::from(dt),
            kind: TrackEventKind::Midi {
                channel: ch,
                message: MidiMessage::NoteOn { key: u7::from(key), vel },
            },
        });
    }
    fn note_off(track: &mut Track, ch: u4, key: u8, vel: u7, dt: u32) {
        track.push(TrackEvent {
            delta: u28::from(dt),
            kind: TrackEventKind::Midi {
                channel: ch,
                message: MidiMessage::NoteOff { key: u7::from(key), vel },
            },
        });
    }

    // 4 successive chords held for a half-note each:
    // Cmaj7, Am7, Fmaj7, G7
    let chords: &[&[u8]] = &[
        &[60, 64, 67, 71], // C E G B
        &[57, 60, 64, 67], // A C E G
        &[53, 57, 60, 64], // F A C E
        &[55, 59, 62, 65], // G B D F
    ];
    let half = quarter * 2;

    for (chord_idx, chord) in chords.iter().enumerate() {
        // All notes start simultaneously.
        for (i, &n) in chord.iter().enumerate() {
            note_on(&mut track, ch, n, v, if i == 0 && chord_idx > 0 { 0 } else { 0 });
        }
        // Hold for half a bar.
        for (i, &n) in chord.iter().enumerate() {
            let dt = if i == 0 { half } else { 0 };
            note_off(&mut track, ch, n, v_off, dt);
        }
    }

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
