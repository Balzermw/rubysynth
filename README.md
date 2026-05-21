# rubysynth

A pure-Rust soft-synth based on the [Rubysynth design overview](./rubysynth_design_overview.md),
with a skinned browser editor and a click-to-play on-screen keyboard.

This is an **MVP** of the full spec — see [Status](#status) for what's implemented today.

## What's in here

- **Engine** (`src/`): a single-voice synth with two oscillators, a lowpass state-variable
  filter, a DAHDSR envelope, an LFO, and a mod matrix routing sources (env / LFO /
  velocity / mod wheel / keytrack / pitch bend) to destinations (pitch, cutoff, resonance,
  amp level, LFO rate).
- **CLI renderer** (`src/bin/render.rs`): take a `.mid` + a patch JSON, render a `.wav`.
- **Web editor** (`src/bin/webui.rs` + `web/static/`): browser UI with sprite-based
  rotating knobs, fader sliders for the amp envelope, a live envelope-shape SVG, a
  mod-matrix editor, and an on-screen MIDI keyboard.
- **Asset pipeline** (`tools/`): Python helpers that slice the supplied sprite sheets
  (`image-assets/`) into individual control sprites under `web/static/sprites/`.

## Quick start

Requires Rust (1.70+) and Python 3 with Pillow for the sprite slicer.

```bash
# Build
cargo build --release

# Render a MIDI file through the default patch
cargo run --release --bin render -- \
    --patch patches/default.json \
    --midi  out/test_melody.mid \
    --out   out/foo.wav

# Or use the web UI
./target/release/webui
# then open http://127.0.0.1:3030
```

The `webui` binary walks upward from its location to find the project root, so it works
no matter what directory you launch it from. Override paths with `--static-dir`,
`--patch`, or `--addr` if you want.

### Generate a test MIDI

```bash
cargo run --example gen_test_midi -- out/test_melody.mid
```

### Re-slice the sprites

If you tweak `image-assets/synth knobs.png` etc., regenerate the sprite PNGs:

```bash
python3 -m pip install --user Pillow
python3 tools/slice_sprites.py
```

## Status

| Feature                              | State                                     |
|--------------------------------------|-------------------------------------------|
| Single-voice engine                  | ✅                                         |
| Two oscillators (sine/tri/saw/sqr)   | ✅                                         |
| Lowpass state-variable filter        | ✅                                         |
| DAHDSR amp envelope + visualization  | ✅                                         |
| LFO (5 shapes incl. S&H)             | ✅                                         |
| Mod matrix (small, fixed set)        | ✅                                         |
| MIDI file rendering (`.mid` → `.wav`)| ✅                                         |
| Web editor (knobs, sliders, kbd)     | ✅                                         |
| Click-to-play keyboard               | ✅ (server renders per-note, ~50–150ms latency) |
| Polyphony                            | ⛔ — last note wins                        |
| WaveScan oscillators                 | ⛔                                         |
| Mutators (FM / sync / PWM / ASM)     | ⛔                                         |
| Second filter (the 11-type bank)     | ⛔                                         |
| FX chain (delay/reverb/etc)          | ⛔                                         |
| Macros & scale system                | ⛔                                         |
| Real-time audio (WASM/cpal)          | ⛔                                         |

## Project layout

```
src/
├── lib.rs                 # crate root, sample rate constant, midi → Hz helper
├── dsp/
│   ├── oscillator.rs      # wavetable osc with detune/level
│   ├── filter.rs          # Chamberlin SVF (LP-tapped for now)
│   ├── envelope.rs        # DAHDSR
│   └── lfo.rs             # 5-shape LFO
├── voice.rs               # per-voice graph: osc → osc → filter → amp
├── mod_matrix.rs          # ModSource / ModDest / ModRoute types
├── patch.rs               # serde-serialized patch data
├── midi_player.rs         # midly-based MIDI parser, Score type
├── engine.rs              # single-voice scheduler
└── bin/
    ├── render.rs          # CLI renderer
    └── webui.rs           # axum web UI server

web/
└── static/                # served as-is by the webui
    ├── index.html
    ├── style.css
    ├── app.js
    └── sprites/           # sliced control images

image-assets/              # source PNG sheets (knobs, buttons, panel)
tools/                     # Python helpers (sprite slicing, grid analysis)
patches/                   # patch JSONs (default.json shipped)
examples/                  # cargo examples (gen_test_midi, inspect_wav)
```

## Web editor tips

- **Knobs**: click-and-drag vertically. Shift = fine. Double-click = reset to default.
  Mouse wheel = small nudges.
- **Faders** (envelope): same — drag, double-click to reset.
- **Keyboard**: click keys, or use ASDFGHJK (white) + WETYU (black). Z/X shift octaves.
- **Hold-time slider** controls how long each clicked note holds before release.

## License

MIT. See [LICENSE](./LICENSE).
