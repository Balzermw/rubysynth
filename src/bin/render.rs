use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use rubysynth::engine::Engine;
use rubysynth::midi_player;
use rubysynth::patch::Patch;
use rubysynth::SAMPLE_RATE;

#[derive(Parser, Debug)]
#[command(name = "render", about = "Render a MIDI file through a Rubysynth patch to a WAV.")]
struct Args {
    /// Path to a patch JSON file.
    #[arg(short, long)]
    patch: PathBuf,

    /// Path to an input MIDI file.
    #[arg(short, long)]
    midi: PathBuf,

    /// Output WAV path.
    #[arg(short, long, default_value = "out/render.wav")]
    out: PathBuf,

    /// Seconds of silence/tail to render after the last MIDI event (lets release tails ring).
    #[arg(long, default_value_t = 2.0)]
    tail: f32,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let patch_json = fs::read_to_string(&args.patch)
        .with_context(|| format!("reading patch file {}", args.patch.display()))?;
    let patch: Patch = serde_json::from_str(&patch_json).context("parsing patch JSON")?;

    let midi_bytes = fs::read(&args.midi)
        .with_context(|| format!("reading midi file {}", args.midi.display()))?;
    let score = midi_player::load_midi(&midi_bytes, SAMPLE_RATE, args.tail)?;

    let mut engine = Engine::new(SAMPLE_RATE);
    let samples = engine.render_score(&patch, &score);

    if let Some(parent) = args.out.parent() {
        fs::create_dir_all(parent).ok();
    }

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&args.out, spec)
        .with_context(|| format!("creating wav {}", args.out.display()))?;

    let peak = samples.iter().fold(0.0_f32, |a, b| a.max(b.abs()));
    let norm = if peak > 1.0 { 0.98 / peak } else { 1.0 };

    for s in &samples {
        let v = (s * norm).clamp(-1.0, 1.0);
        let i = (v * i16::MAX as f32) as i16;
        writer.write_sample(i)?;
    }
    writer.finalize()?;

    println!(
        "wrote {} samples ({:.2}s) to {}",
        samples.len(),
        samples.len() as f32 / SAMPLE_RATE as f32,
        args.out.display()
    );
    Ok(())
}
