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

    // For stereo, compare left vs right RMS to detect stereo signal (chorus/reverb width).
    let mut l_rms = 0.0_f64;
    let mut r_rms = 0.0_f64;
    let mut diff_rms = 0.0_f64;
    if spec.channels == 2 {
        let frames = n / 2;
        for i in 0..frames {
            let l = samples[2 * i] as f64;
            let r = samples[2 * i + 1] as f64;
            l_rms += l * l;
            r_rms += r * r;
            diff_rms += (l - r) * (l - r);
        }
        l_rms = (l_rms / frames as f64).sqrt();
        r_rms = (r_rms / frames as f64).sqrt();
        diff_rms = (diff_rms / frames as f64).sqrt();
    }

    println!("file:    {}", path);
    println!("spec:    {} ch, {} Hz, {} bit", spec.channels, spec.sample_rate, spec.bits_per_sample);
    println!("samples: {}", n);
    println!("peak:    {} ({:.3} of full scale)", peak, peak as f64 / i16::MAX as f64);
    println!("rms:     {:.1} ({:.3} of full scale)", rms, rms / i16::MAX as f64);
    if spec.channels == 2 {
        println!("L rms:   {:.1}", l_rms);
        println!("R rms:   {:.1}", r_rms);
        println!("L-R rms: {:.1}  (>{}: real stereo)", diff_rms, 100);
    }
    Ok(())
}
