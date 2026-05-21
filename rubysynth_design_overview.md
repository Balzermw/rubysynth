# Rubysynth Design Overview

At its core, the Rubysynth is an 8-voice polyphonic digital synthesizer built around a fully modular signal path with an aggressive modulation system layered over the top. The design philosophy is that every meaningful parameter is a destination, almost every interesting time-varying signal is a source, and the user interface exposes that flexibility without burying it in menus. Here's how the pieces fit together.

## Voice Architecture and Signal Flow

Each of the 8 voices is an identical instance of the same DSP graph. The left-to-right signal path is: three oscillators feed through optional "mutator" stages, then into a mixer (which also hosts a ring modulator and noise generator), through two filters that can run in series or in parallel, into a simple amp stage, and finally into a four-stage effects chain. Modulation sources (envelopes, LFOs, performance controls, MIDI, CV) sit alongside the audio path and can address almost any parameter at any stage.

The signal path is fixed in structure but highly configurable in routing — for example, the mixer determines per-source how much of each oscillator goes to Filter 1 vs Filter 2, and a separate flag toggles the filters between series and parallel topologies. This is one of the key design decisions: rather than building a fully patchable graph (which would explode UI complexity), the designers fixed the topology and made the *routing weights* freely modulatable.

## Oscillator Section

There are three oscillators per voice. Oscillators 1 and 2 are full-featured and operate in either Single mode (one waveform from a 219-entry waveform bank) or WaveScan mode. Oscillator 3 is Single mode only. The waveform bank is curated rather than synthesized procedurally — it contains classic shapes, harmonic-rich variants, and a large library of designed wavetables grouped by character.

WaveScan is the more interesting design choice. Each oscillator holds an ordered list of up to 8 waveforms, indexed 1.0 through 8.0. A continuous WaveScan parameter lets you sit at any fractional position, and the engine crossfades between adjacent slots. That position is itself a mod destination, so an LFO or envelope can sweep through the wavelist over time. If you're rebuilding this, the key insight is that the wavelist is *per-oscillator data*, not a global table — each preset can define its own 8-slot palette.

## Mutators

This is the most distinctive part of the design. Sitting after Osc 1 and Osc 2 are two Mutator stages each (four total). Each Mutator implements one of seven algorithms: linear FM, wave stacking (5 detuned copies), oscillator sync, three flavors of pulse-width modulation (classic, time-squeezed, and an 8-point ASM "warp" mode), and a harmonic emphasis filter. Mutators in a pair can be chained, and the FM and OscSync modes can take their modulator/sync source from any of the oscillators, other mutators, ring mod, noise, or even the external CV inputs (which can accept audio-rate signals).

The clever architectural choice here is that the "ratio" parameter in OscSync and PWM modes isn't a frequency ratio in the usual analog sense — it's the number of mutation events per waveform cycle. So you can have PWM happening 64 times per cycle or once every 4 cycles, which gives a much wider sonic range than the analog circuits these are modeling. The PW-ASM mode is essentially user-drawn FM: 8 warp points define an arbitrary modulator shape, and each warp point is independently modulatable.

## Mixer, Ring Mod, and Noise

The mixer is more than a level/pan stage. For each of the five sources (Osc 1-3, Ring, Noise) it provides level, pan, and a filter routing ratio that determines how much of that source goes to Filter 1 vs Filter 2. A single global flag toggles whether the two filters are wired in series (Filter 1 → Filter 2 → Amp) or parallel (each goes directly to Amp). A Solo function isolates any single source for editing — useful during sound design.

The ring modulator takes any two internal or external sources and produces sum-and-difference output. The noise generator offers three spectral shapes (white, pink, brown). Both have their levels and pans exposed in the mixer.

## Dual Filter Architecture

The two filters are deliberately asymmetric in personality.

Filter 1 is a model bank: eleven different filter types covering low-pass ladder filters in both compensated and uncompensated variants (12 and 24 dB/oct), MS-20-style LP and HP, a low-pass gate, a three-mode boutique-modular model in LP/BP/HP, and a vocal formant filter with 8 selectable vowel orderings. The compensation distinction matters for rebuilding: uncompensated filters lose bass as resonance increases (the classic analog behavior), compensated ones maintain it.

Filter 2 is a single state-variable design (2-pole) with a Morph parameter that sweeps continuously between LP, BP, and HP responses. This is the "always available" filter that pairs with Filter 1's character.

Both filters share the same control set: cutoff, resonance, dedicated Env 1 and LFO 1 amount controls (pre-wired but bypassable), velocity-to-envelope scaling, and keyboard tracking centered on C2. Filter 1 adds a Drive parameter that can be inserted either pre- or post-filter.

## Amp Stage

Intentionally minimal: three parameters. Level sets the maximum output, LFO 2 amount provides pre-wired tremolo modulation (bipolar — negative values invert phase), and Velocity scales response to note dynamics. The simplicity is deliberate; complex amplitude shaping is expected to come from Envelope 2 (which is pre-wired here) and from the mod matrix.

## Envelopes and LFOs as Identical Resources

Five envelopes and five LFOs, all identical. This is a clean architectural choice — there's no "filter envelope" vs "amp envelope" distinction in the engine, just five interchangeable envelopes. Envelope 1 and LFO 1 happen to be pre-wired into the filter section, and Envelope 2 and LFO 2 into the amp stage, but those are routing defaults, not hardware-fixed connections.

Envelopes are DAHDSR (delay, attack, hold, decay, sustain, release) with independently adjustable curves on the attack, decay, and release segments (log to linear to exponential, continuously). They can free-run, reset on each note, respect legato, and loop a defined number of times or indefinitely — which means an envelope can serve as a complex LFO if you run out of LFOs.

LFOs offer 11 waveforms including a Step LFO that's effectively an 8-step sequencer per voice. They have delay, fade-in, phase offset, and a smoothing/slew parameter. Critically, they have a TrigSync setting that determines whether each voice gets its own independent LFO phase (Poly), all voices share one phase reset on each note (Single), or the LFO free-runs without reset (Off). Both envelopes and LFOs can sync segment durations to BPM with full musical subdivisions including triplets and dotted values.

## Effects Chain

Four serial effect blocks: Pre-FX → Delay → Reverb → Post-FX. Pre and Post are identical eight-algorithm blocks (chorus, flanger, rotary, phaser, lo-fi, tremolo, EQ, compressor). Delay offers five topologies (mono, stereo, ping-pong, LRC, reverse) with full BPM sync. Reverb is a four-type algorithmic processor (hall, room, plate, cloud) with separate hi and lo damping and an explicit Freeze state.

A subset of effect parameters is exposed as mod destinations (typically the two main parameters plus dry/wet), and certain parameters have finer internal resolution for smooth modulation.

## The Modulation Matrix

This is the spine. 32 modulation slots per patch, each with a source, a destination (module + specific parameter), and a signed depth. Sources include all five envelopes, all five LFOs (each available in bipolar and unipolar form), mono and poly aftertouch, velocity on and off, pitch and mod wheels, three different ribbon readings (absolute bipolar, absolute positive, relative), expression pedal, the two CV inputs, MIDI keyboard tracking, and any of the 128 MIDI CCs.

Destinations span essentially everything: oscillator pitch and wavescan, all mutator parameters including individual ASM warp points, mixer routing, both filters' full parameter sets, amp, envelope segment times, LFO rates and levels, effect parameters, the two CV outputs, MIDI CCs back out, and — recursively — the depth of any other mod slot. That last point matters: it means the matrix is two-layer (a source can modulate another route's depth), giving you VCA-like behavior without dedicated VCA modules.

## Macros as a Performance Layer

On top of the mod matrix sits the Macro system. Each patch has 8 macros, each tied to one of the 8 encoder/button pairs on the home screen. A macro is essentially a mini mod matrix of its own: up to 8 destinations per macro, each with its own depth, and the macro knob acts as a single source that drives all of them simultaneously. The associated button can be configured (globally) as toggle, momentary trigger, exclusive switch, or reset-to-zero. When saving a patch, the macros' current positions can be returned to zero, saved as-is, or *converted* — meaning the current modulated values are baked into the underlying parameters.

## Voice Module

A patch-level container for things that don't belong in the per-voice signal path: polyphony mode (poly, mono, mono-low, mono-high, and three unison variants with density and detune), voice stealing behavior, glide settings, pitch bend range, a dedicated vibrato LFO (the sixth LFO, hardwired to the mod wheel), and the active scale. The scale system supports 38 presets plus a custom 8-note user scale, with a separate Key Lock parameter that defines the root.

Two parameters here are worth highlighting for a rebuild: Random Phase (oscillators don't reset to phase 0 on note-on, emulating free-running analog oscillators) and Analog Feel (a global parameter introducing graduated drift/instability throughout the signal path).

## Performance Hardware

The keyboard model has 49 polyphonic-aftertouch keys, two backlit wheels, octave/chord buttons, a ribbon controller, and a glide button. The ribbon has three modes: pitch bend (anywhere-is-center), theremin (a dedicated voice that the ribbon plays as a monophonic instrument, with its own quantize-to-scale and glide), and mod-only. The ribbon offers three distinct readings to the mod matrix simultaneously.

The desktop model substitutes 24 RGB velocity- and pressure-sensitive pads with a sophisticated pad mode system. The pads can lay out chromatically, in fretboard fourths, in octave rows, or in single-octave-per-row, all combined with 16 scale options and 12 root keys. Pad colors indicate root, in-scale, and out-of-scale notes.

## I/O and Synchronization

USB MIDI is class-compliant — no driver layer. Five-pin MIDI In/Out/Thru. The CV section is the differentiator: two CV inputs (configurable voltage range and offset per input, accepting audio-rate signals so they can serve as FM sources), and five outputs — pitch (1V/oct or Hz/V, with reference note and tuning offset), gate (V-trig or S-trig, three voltage levels), two assignable mod outputs, and a clock output with selectable PPQ rates and division.

Clock sync priorities (USB → MIDI → internal) and overflow mode (linking two units over MIDI for 16-voice polyphony with patch mirroring) are handled at the system level rather than per-patch.

## UI Design Principles

Two OLED displays — the left one shows patch identity and waveform animation, the right one is the editing surface with eight encoder/button pairs around it. Module Select buttons on the panel double as shortcuts: holding one and pressing another creates a mod route at the first empty slot; holding Save and pressing two compatible modules copies parameters between them; holding Init or Random and pressing a module resets or randomizes it. The same buttons select macro destinations when on the macro assign page.

The orange-text-vs-white-text convention on the panel labels distinguishes Access buttons (open a page) from Function buttons (perform an action). Encoders are exclusively endless rotary, so parameter values never jump to a knob's physical position — they edit from current state, which matters for live performance and for patch consistency.

---

That's the Rubysynth design at the level someone could use to scope a rebuild. Subsystems worth deeper treatment if needed: the wavetable interpolation, the mutator algorithms, the modulation matrix implementation, or the CV calibration.
