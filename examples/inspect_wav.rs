// Quick stats on a WAV file: sample count, peak, RMS, count of distinct values.
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args().nth(1).ok_or("usage: inspect_wav <file>")?;
    let mut reader = hound::WavReader::open(&path)?;
    let spec = reader.spec();
    let samples: Vec<i32> = reader.samples::<i16>().map(|s| s.unwrap() as i32).collect();

    let n = samples.len();
    let peak = samples.iter().map(|s| s.abs()).max().unwrap_or(0);
    let rms = {
        let sum_sq: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
        (sum_sq / n as f64).sqrt()
    };
    let mut distinct = std::collections::HashSet::new();
    for s in &samples {
        distinct.insert(*s);
        if distinct.len() > 5000 {
            break;
        }
    }

    println!("file:    {}", path);
    println!("spec:    {} ch, {} Hz, {} bit", spec.channels, spec.sample_rate, spec.bits_per_sample);
    println!("samples: {}", n);
    println!("peak:    {} ({:.3} of full scale)", peak, peak as f64 / i16::MAX as f64);
    println!("rms:     {:.1} ({:.3} of full scale)", rms, rms / i16::MAX as f64);
    println!("distinct values (capped at 5000): {}", distinct.len());
    Ok(())
}
