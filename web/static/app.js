// Rubysynth web editor — sprite-based knobs, drag-vertical, REST patch save + WAV render.

const MOD_SOURCES = ["env1", "lfo1", "velocity", "mod_wheel", "key_track", "pitch_bend"];
const MOD_DESTS = [
  "pitch_semi", "osc1_pitch_semi", "osc2_pitch_semi",
  "filter_cutoff_hz", "filter_resonance",
  "amp_level", "lfo1_rate",
];

const KNOB_MIN_DEG = -135;
const KNOB_MAX_DEG = 135;

let patch = null;

// ---- patch path helpers ----
function get(obj, path) {
  return path.split(".").reduce((o, k) => (o == null ? o : o[k]), obj);
}
function set(obj, path, value) {
  const parts = path.split(".");
  const last = parts.pop();
  parts.reduce((o, k) => o[k], obj)[last] = value;
}

// ---- formatting ----
function formatValue(v, unit) {
  if (typeof v !== "number") return String(v);
  let txt;
  if (Math.abs(v) >= 100) txt = v.toFixed(0);
  else if (Math.abs(v) >= 10) txt = v.toFixed(1);
  else if (Math.abs(v) >= 1) txt = v.toFixed(2);
  else txt = v.toFixed(3);
  return unit ? `${txt} ${unit}` : txt;
}

// ---- knob component ----
function buildKnob(cell) {
  const path = cell.dataset.path;
  const min = parseFloat(cell.dataset.min);
  const max = parseFloat(cell.dataset.max);
  const def = parseFloat(cell.dataset.default);
  const step = parseFloat(cell.dataset.step || "0.01");
  const curve = cell.dataset.curve || "linear";
  const label = cell.dataset.label || "";
  const unit = cell.dataset.unit || "";

  cell.innerHTML = `
    <div class="knob" tabindex="0" role="slider" aria-label="${label}"></div>
    <div class="knob-label">${label}</div>
    <div class="knob-value"></div>
  `;
  const knobEl = cell.querySelector(".knob");
  const valueEl = cell.querySelector(".knob-value");

  function valToNorm(v) {
    if (curve === "log") {
      const lo = Math.log(Math.max(min, 0.0001));
      const hi = Math.log(max);
      return (Math.log(Math.max(v, 0.0001)) - lo) / (hi - lo);
    }
    return (v - min) / (max - min);
  }
  function normToVal(n) {
    n = Math.max(0, Math.min(1, n));
    let v;
    if (curve === "log") {
      const lo = Math.log(Math.max(min, 0.0001));
      const hi = Math.log(max);
      v = Math.exp(lo + (hi - lo) * n);
    } else {
      v = min + (max - min) * n;
    }
    if (step) v = Math.round(v / step) * step;
    return Math.max(min, Math.min(max, v));
  }

  function render() {
    const v = get(patch, path);
    const n = valToNorm(v);
    const deg = KNOB_MIN_DEG + (KNOB_MAX_DEG - KNOB_MIN_DEG) * n;
    knobEl.style.setProperty("--rot", `${deg}deg`);
    valueEl.textContent = formatValue(v, unit);
  }

  // Drag interaction.
  let dragging = false;
  let startY = 0;
  let startNorm = 0;
  let fineMode = false;

  function onPointerDown(e) {
    e.preventDefault();
    dragging = true;
    fineMode = e.shiftKey;
    startY = e.clientY;
    startNorm = valToNorm(get(patch, path));
    knobEl.classList.add("dragging");
    knobEl.setPointerCapture(e.pointerId);
  }
  function onPointerMove(e) {
    if (!dragging) return;
    const dy = startY - e.clientY;
    const sensitivity = (e.shiftKey || fineMode) ? 1000 : 350;
    const newNorm = Math.max(0, Math.min(1, startNorm + dy / sensitivity));
    const v = normToVal(newNorm);
    set(patch, path, v);
    render();
  }
  function onPointerUp(e) {
    dragging = false;
    knobEl.classList.remove("dragging");
    try { knobEl.releasePointerCapture(e.pointerId); } catch {}
  }
  function onDoubleClick() {
    set(patch, path, def);
    render();
  }
  function onWheel(e) {
    e.preventDefault();
    const n = valToNorm(get(patch, path));
    const delta = -Math.sign(e.deltaY) * (e.shiftKey ? 0.005 : 0.02);
    set(patch, path, normToVal(n + delta));
    render();
  }

  knobEl.addEventListener("pointerdown", onPointerDown);
  knobEl.addEventListener("pointermove", onPointerMove);
  knobEl.addEventListener("pointerup",   onPointerUp);
  knobEl.addEventListener("pointercancel", onPointerUp);
  knobEl.addEventListener("dblclick", onDoubleClick);
  knobEl.addEventListener("wheel", onWheel, { passive: false });

  cell.render = render;
  return render;
}

// ---- vertical slider (used by amp envelope DAHDSR) ----
function buildSlider(cell) {
  const path = cell.dataset.path;
  const min = parseFloat(cell.dataset.min);
  const max = parseFloat(cell.dataset.max);
  const def = parseFloat(cell.dataset.default);
  const step = parseFloat(cell.dataset.step || "0.01");
  const label = cell.dataset.label || "";
  const unit = cell.dataset.unit || "";

  cell.innerHTML = `
    <div class="env-track" tabindex="0" role="slider" aria-label="${label}">
      <div class="env-cap"></div>
    </div>
    <div class="env-label">${label}</div>
    <div class="env-value"></div>
  `;
  const track = cell.querySelector(".env-track");
  const cap = cell.querySelector(".env-cap");
  const valueEl = cell.querySelector(".env-value");

  function valToNorm(v) {
    return (v - min) / (max - min);
  }
  function normToVal(n) {
    n = Math.max(0, Math.min(1, n));
    let v = min + (max - min) * n;
    if (step) v = Math.round(v / step) * step;
    return Math.max(min, Math.min(max, v));
  }

  function render() {
    const v = get(patch, path);
    const norm = valToNorm(v);
    const trackH = track.clientHeight || 130;
    const capH = 18;
    const usable = trackH - capH - 4;
    const top = 2 + (1 - Math.max(0, Math.min(1, norm))) * usable;
    cap.style.top = `${top}px`;
    valueEl.textContent = formatValue(v, unit);
    updateEnvViz();
  }

  // Drag model: capture the cursor position and starting value on pointerdown.
  // Pointermove changes the value by the accumulated delta only — clicking
  // without dragging leaves the value unchanged. Matches the knob behaviour.
  let dragging = false;
  let startY = 0;
  let startNorm = 0;

  track.addEventListener("pointerdown", (e) => {
    e.preventDefault();
    dragging = true;
    startY = e.clientY;
    startNorm = valToNorm(get(patch, path));
    cell.classList.add("dragging");
    track.setPointerCapture(e.pointerId);
  });
  track.addEventListener("pointermove", (e) => {
    if (!dragging) return;
    const dy = startY - e.clientY;
    const sensitivity = e.shiftKey ? 1000 : 350;
    set(patch, path, normToVal(startNorm + dy / sensitivity));
    render();
  });
  const stop = (e) => {
    dragging = false;
    cell.classList.remove("dragging");
    try { track.releasePointerCapture(e.pointerId); } catch {}
  };
  track.addEventListener("pointerup", stop);
  track.addEventListener("pointercancel", stop);
  track.addEventListener("dblclick", () => {
    set(patch, path, def);
    render();
  });
  track.addEventListener("wheel", (e) => {
    e.preventDefault();
    const norm = valToNorm(get(patch, path));
    const delta = -Math.sign(e.deltaY) * (e.shiftKey ? 0.005 : 0.02);
    set(patch, path, normToVal(norm + delta));
    render();
  }, { passive: false });

  cell.render = render;
  return render;
}

// ---- envelope visualization ----
function updateEnvViz() {
  const svg = document.getElementById("env-viz");
  if (!svg || !patch) return;
  const W = 600, H = 140;
  const PAD_L = 6, PAD_R = 6, PAD_T = 8, PAD_B = 10;
  const drawW = W - PAD_L - PAD_R;
  const drawH = H - PAD_T - PAD_B;

  const { delay, attack, hold, decay, sustain, release } = patch.env1;
  const sustainDuration = 0.5; // synthetic, just for display
  const totalTime = delay + attack + hold + decay + sustainDuration + release;
  const t = totalTime > 0.001 ? totalTime : 0.001;

  function px(time, level) {
    const x = PAD_L + (time / t) * drawW;
    const y = PAD_T + (1 - Math.max(0, Math.min(1, level))) * drawH;
    return [x, y];
  }

  const points = [];
  let t0 = 0;
  points.push(px(t0, 0));                          // delay start
  t0 += delay; points.push(px(t0, 0));             // delay end
  t0 += attack; points.push(px(t0, 1));            // attack peak
  t0 += hold; points.push(px(t0, 1));              // hold end
  t0 += decay; points.push(px(t0, sustain));       // decay → sustain
  t0 += sustainDuration; points.push(px(t0, sustain)); // sustain end
  t0 += release; points.push(px(t0, 0));           // release end

  const path = points.map(([x, y], i) => `${i === 0 ? "M" : "L"} ${x.toFixed(1)} ${y.toFixed(1)}`).join(" ");
  const last = points[points.length - 1];
  const first = points[0];
  const fillPath = `${path} L ${last[0].toFixed(1)} ${H - PAD_B} L ${first[0].toFixed(1)} ${H - PAD_B} Z`;

  document.getElementById("env-line").setAttribute("d", path);
  document.getElementById("env-fill-path").setAttribute("d", fillPath);

  // Stage tick marks + labels (only where the segment has nonzero duration).
  const labels = document.getElementById("env-stage-labels");
  labels.innerHTML = "";
  const stages = [
    { name: "D", end: delay,                                 dur: delay },
    { name: "A", end: delay + attack,                        dur: attack },
    { name: "H", end: delay + attack + hold,                 dur: hold },
    { name: "D", end: delay + attack + hold + decay,         dur: decay },
    { name: "S", end: delay + attack + hold + decay + sustainDuration, dur: sustainDuration },
    { name: "R", end: totalTime,                              dur: release },
  ];
  let lastX = PAD_L;
  for (const s of stages) {
    const [x] = px(s.end, 0);
    if (s.dur > 0.001) {
      const tick = document.createElementNS("http://www.w3.org/2000/svg", "line");
      tick.setAttribute("x1", x); tick.setAttribute("x2", x);
      tick.setAttribute("y1", PAD_T); tick.setAttribute("y2", H - PAD_B);
      labels.appendChild(tick);
      const cx = (lastX + x) / 2;
      const text = document.createElementNS("http://www.w3.org/2000/svg", "text");
      text.setAttribute("x", cx); text.setAttribute("y", H - 2);
      text.setAttribute("text-anchor", "middle");
      text.textContent = s.name;
      labels.appendChild(text);
    }
    lastX = x;
  }
}

// ---- selects ----
function bindSelects() {
  document.querySelectorAll("[data-path]:not(.knob-cell)").forEach((el) => {
    if (el.tagName !== "SELECT") return;
    el.value = get(patch, el.dataset.path);
    el.addEventListener("change", () => set(patch, el.dataset.path, el.value));
  });
}

// ---- mod rows ----
function renderModRows() {
  const tbody = document.getElementById("mod-rows");
  tbody.innerHTML = "";
  patch.mod_routes.forEach((route, idx) => {
    const tr = document.createElement("tr");

    function mkSelect(opts, value, onChange) {
      const sel = document.createElement("select");
      opts.forEach((o) => sel.add(new Option(o, o)));
      sel.value = value;
      sel.addEventListener("change", () => onChange(sel.value));
      return sel;
    }

    const tdSrc = document.createElement("td");
    tdSrc.appendChild(mkSelect(MOD_SOURCES, route.source, (v) => (route.source = v)));
    const tdDst = document.createElement("td");
    tdDst.appendChild(mkSelect(MOD_DESTS, route.destination, (v) => (route.destination = v)));
    const tdDepth = document.createElement("td");
    const depthIn = document.createElement("input");
    depthIn.type = "number";
    depthIn.step = "0.01";
    depthIn.value = route.depth;
    depthIn.addEventListener("input", () => (route.depth = parseFloat(depthIn.value) || 0));
    tdDepth.appendChild(depthIn);
    const tdRm = document.createElement("td");
    const rm = document.createElement("button");
    rm.className = "remove";
    rm.textContent = "✕";
    rm.addEventListener("click", () => {
      patch.mod_routes.splice(idx, 1);
      renderModRows();
    });
    tdRm.appendChild(rm);
    tr.append(tdSrc, tdDst, tdDepth, tdRm);
    tbody.appendChild(tr);
  });
}

// ---- patch lifecycle ----
const knobRenders = [];

async function loadPatch() {
  const res = await fetch("/api/patch");
  patch = await res.json();
  document.getElementById("patch-name").textContent = patch.name || "(unnamed)";

  knobRenders.length = 0;
  document.querySelectorAll("[data-knob]").forEach((cell) => {
    knobRenders.push(buildKnob(cell));
  });
  document.querySelectorAll("[data-env]").forEach((cell) => {
    knobRenders.push(buildSlider(cell));
  });
  knobRenders.forEach((r) => r());
  bindSelects();
  renderModRows();
  updateEnvViz();
}

async function savePatch() {
  const res = await fetch("/api/patch", {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(patch),
  });
  setStatus(res.ok ? "Saved." : "Save failed.");
}

async function renderWav() {
  const file = document.getElementById("midi-input").files[0];
  if (!file) {
    setStatus("Pick a MIDI file first.");
    return;
  }
  setStatus("Saving patch…");
  await savePatch();
  setStatus("Rendering…");

  const form = new FormData();
  form.append("midi", file, file.name);
  const res = await fetch("/api/render", { method: "POST", body: form });
  if (!res.ok) {
    setStatus("Render failed: " + (await res.text()));
    return;
  }
  const blob = await res.blob();
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = "render.wav";
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
  setStatus(`Rendered ${(blob.size / 1024).toFixed(1)} KB.`);
}

function setStatus(msg) {
  document.getElementById("render-status").textContent = msg;
}

document.getElementById("reload").addEventListener("click", loadPatch);
document.getElementById("save").addEventListener("click", savePatch);
document.getElementById("render").addEventListener("click", renderWav);
document.getElementById("add-mod").addEventListener("click", () => {
  patch.mod_routes.push({ source: "env1", destination: "filter_cutoff_hz", depth: 0 });
  renderModRows();
});
document.getElementById("midi-input").addEventListener("change", (e) => {
  const f = e.target.files[0];
  document.getElementById("midi-filename").textContent = f ? f.name : "— none —";
});

// ---- on-screen keyboard ----
let baseOctave = 4; // C4 = MIDI 60
const NOTE_NAMES = ["C","C#","D","D#","E","F","F#","G","G#","A","A#","B"];
const WHITE_OFFSETS = [0, 2, 4, 5, 7, 9, 11];
const BLACK_OFFSETS = [
  { offset: 1, after: 0 }, // C#
  { offset: 3, after: 1 }, // D#
  { offset: 6, after: 3 }, // F#
  { offset: 8, after: 4 }, // G#
  { offset: 10, after: 5 }, // A#
];

// Two-octave layout, plus the top C.
function buildKeyboard() {
  const kb = document.getElementById("keyboard");
  kb.innerHTML = "";
  const octaves = 2;
  const whiteKeys = [];

  // White keys: 7 per octave * octaves + 1 (top C)
  for (let o = 0; o < octaves; o++) {
    for (let i = 0; i < 7; i++) {
      const semis = WHITE_OFFSETS[i];
      const midi = 12 * (baseOctave + 1) + o * 12 + semis; // C(baseOctave) = 12*(baseOctave+1)
      const el = makeKey(midi, "white");
      kb.appendChild(el);
      whiteKeys.push({ el, semis: o * 12 + semis });
    }
  }
  const topMidi = 12 * (baseOctave + 1) + octaves * 12;
  const topC = makeKey(topMidi, "white");
  kb.appendChild(topC);
  whiteKeys.push({ el: topC, semis: octaves * 12 });

  // Black keys: positioned absolutely relative to white-key layout.
  // First compute white key width as a percentage of the keyboard width.
  // We do this after first layout pass via requestAnimationFrame.
  requestAnimationFrame(() => {
    const kbWidth = kb.clientWidth - 20; // subtract padding (10 each side)
    const totalWhites = whiteKeys.length;
    const whiteW = kbWidth / totalWhites;
    const blackW = whiteW * 0.6;

    for (let o = 0; o < octaves; o++) {
      for (const { offset, after } of BLACK_OFFSETS) {
        const midi = 12 * (baseOctave + 1) + o * 12 + offset;
        const el = makeKey(midi, "black");
        // Position: centered on the boundary between white[after] and white[after+1] within this octave.
        const whiteIdxInBlock = o * 7 + after;
        const leftPx = 10 + (whiteIdxInBlock + 1) * whiteW - blackW / 2;
        el.style.left = `${leftPx}px`;
        el.style.width = `${blackW}px`;
        kb.appendChild(el);
      }
    }
  });
}

function makeKey(midi, color) {
  const el = document.createElement("div");
  el.className = `key ${color}`;
  el.dataset.midi = midi;
  const noteName = NOTE_NAMES[midi % 12] + Math.floor(midi / 12 - 1);
  el.title = `${noteName} (MIDI ${midi})`;
  const label = document.createElement("span");
  label.className = "key-label";
  label.textContent = noteName;
  el.appendChild(label);

  el.addEventListener("pointerdown", (e) => {
    e.preventDefault();
    playNote(midi, el);
  });
  return el;
}

const activeAudio = new Set();
let inflightAbort = null;

// ---- Web Audio: shared AudioContext + AnalyserNode for the scope ----
let audioCtx = null;
let analyser = null;
let scopeBuf = null;
let scopeRaf = null;

function ensureAudioCtx() {
  if (!audioCtx) {
    const Ctx = window.AudioContext || window.webkitAudioContext;
    audioCtx = new Ctx();
    analyser = audioCtx.createAnalyser();
    analyser.fftSize = 2048;
    analyser.smoothingTimeConstant = 0;
    scopeBuf = new Float32Array(analyser.fftSize);
    analyser.connect(audioCtx.destination);
    startScope();
  }
  if (audioCtx.state === "suspended") audioCtx.resume();
}

function attachToAnalyser(audio) {
  // Each new <audio> element gets its own MediaElementAudioSourceNode that
  // routes through the analyser before reaching the speakers.
  try {
    const src = audioCtx.createMediaElementSource(audio);
    src.connect(analyser);
  } catch (e) {
    // If the element was already wired (shouldn't happen with new Audio()),
    // fall back to direct playback so we don't lose audio.
    console.warn("scope tap failed; playing dry:", e);
  }
}

function startScope() {
  const canvas = document.getElementById("scope");
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  const status = document.getElementById("scope-status");

  function resize() {
    const r = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.round(r.width * dpr);
    canvas.height = Math.round(r.height * dpr);
  }
  resize();
  window.addEventListener("resize", resize);

  function findTrigger(buf) {
    // Find first rising zero crossing in the first half of the buffer. Falls back to 0.
    const half = buf.length >> 1;
    for (let i = 1; i < half; i++) {
      if (buf[i - 1] <= 0 && buf[i] > 0) return i;
    }
    return 0;
  }

  function draw() {
    scopeRaf = requestAnimationFrame(draw);
    if (!analyser) return;
    analyser.getFloatTimeDomainData(scopeBuf);

    const w = canvas.width, h = canvas.height;
    ctx.clearRect(0, 0, w, h);

    // grid
    ctx.strokeStyle = "rgba(0, 255, 80, 0.08)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, h / 2); ctx.lineTo(w, h / 2);
    for (let i = 1; i < 8; i++) {
      const x = (w * i) / 8;
      ctx.moveTo(x, 0); ctx.lineTo(x, h);
    }
    for (let i = 1; i < 4; i++) {
      const y = (h * i) / 4;
      ctx.moveTo(0, y); ctx.lineTo(w, y);
    }
    ctx.stroke();

    // sweep window: half the buffer, starting at the zero-crossing trigger.
    const trig = findTrigger(scopeBuf);
    const N = scopeBuf.length - trig;
    const step = w / N;
    let peak = 0;
    ctx.lineWidth = 2;
    ctx.strokeStyle = "rgba(140, 255, 160, 0.95)";
    ctx.shadowBlur = 8;
    ctx.shadowColor = "rgba(120, 255, 150, 0.6)";
    ctx.beginPath();
    for (let i = 0; i < N; i++) {
      const s = scopeBuf[trig + i];
      if (Math.abs(s) > peak) peak = Math.abs(s);
      const x = i * step;
      const y = h / 2 - s * (h / 2 - 4);
      if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
    }
    ctx.stroke();
    ctx.shadowBlur = 0;

    if (status) {
      if (peak < 0.001) status.textContent = "idle";
      else status.textContent = `peak ${peak.toFixed(2)}`;
    }
  }
  draw();
}

function stopAllAudio() {
  for (const a of activeAudio) {
    try { a.pause(); a.currentTime = 0; URL.revokeObjectURL(a.src); } catch {}
  }
  activeAudio.clear();
}

async function playNote(midi, keyEl) {
  // Kill anything currently in-flight or playing before starting a new note.
  // Otherwise an older fetch can resolve and play *after* the newer one,
  // producing two simultaneous notes from a single keypress.
  stopAllAudio();
  if (inflightAbort) inflightAbort.abort();
  inflightAbort = new AbortController();
  const signal = inflightAbort.signal;

  // Audio context must be created/resumed in a user gesture handler. The
  // keyboard click counts as one.
  ensureAudioCtx();

  if (keyEl) {
    keyEl.classList.add("active");
    setTimeout(() => keyEl.classList.remove("active"), 180);
  }
  await savePatchQuiet();
  if (signal.aborted) return;

  const holdMs = parseInt(document.getElementById("hold-ms").value, 10);
  const url = `/api/note?note=${midi}&velocity=100&hold_ms=${holdMs}`;
  try {
    const res = await fetch(url, { signal });
    if (signal.aborted || !res.ok) return;
    const blob = await res.blob();
    if (signal.aborted) return;

    const audio = new Audio(URL.createObjectURL(blob));
    audio.crossOrigin = "anonymous";
    attachToAnalyser(audio);
    activeAudio.add(audio);
    audio.addEventListener("ended", () => {
      activeAudio.delete(audio);
      URL.revokeObjectURL(audio.src);
    });
    await audio.play();
  } catch (err) {
    if (err.name !== "AbortError") console.error(err);
  }
}

async function savePatchQuiet() {
  await fetch("/api/patch", {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(patch),
  });
}

// Keyboard shortcuts: Ableton-style mapping.
const KEY_TO_SEMI = {
  // White keys: A S D F G H J K = C D E F G A B C(+12)
  "a": 0,  "s": 2,  "d": 4,  "f": 5,  "g": 7,  "h": 9,  "j": 11, "k": 12,
  // Black keys: W E T Y U = C# D# F# G# A#
  "w": 1,  "e": 3,  "t": 6,  "y": 8,  "u": 10,
};
const pressed = new Set();
document.addEventListener("keydown", (e) => {
  if (e.repeat) return;
  const tag = (e.target.tagName || "").toLowerCase();
  if (tag === "input" || tag === "select" || tag === "textarea") return;

  const k = e.key.toLowerCase();
  if (k === "z") { setOctave(baseOctave - 1); return; }
  if (k === "x") { setOctave(baseOctave + 1); return; }
  if (k in KEY_TO_SEMI) {
    const midi = 12 * (baseOctave + 1) + KEY_TO_SEMI[k];
    const keyEl = document.querySelector(`.key[data-midi="${midi}"]`);
    pressed.add(k);
    playNote(midi, keyEl);
  }
});
document.addEventListener("keyup", (e) => {
  pressed.delete(e.key.toLowerCase());
});

function setOctave(o) {
  baseOctave = Math.max(0, Math.min(8, o));
  document.getElementById("oct-label").textContent = baseOctave;
  buildKeyboard();
}

document.getElementById("oct-down").addEventListener("click", () => setOctave(baseOctave - 1));
document.getElementById("oct-up").addEventListener("click", () => setOctave(baseOctave + 1));
document.getElementById("hold-ms").addEventListener("input", (e) => {
  document.getElementById("hold-ms-val").textContent = `${e.target.value} ms`;
});

setOctave(baseOctave);
window.addEventListener("resize", buildKeyboard);

loadPatch();
