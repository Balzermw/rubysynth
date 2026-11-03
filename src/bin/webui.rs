use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use axum::body::Body;
use axum::extract::{Multipart, Query, State};
use serde::Deserialize;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use clap::Parser;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

use rubysynth::engine::Engine;
use rubysynth::midi_player;
use rubysynth::patch::Patch;
use rubysynth::SAMPLE_RATE;

#[derive(Parser, Debug)]
#[command(name = "webui", about = "Rubysynth browser editor.")]
struct Args {
    /// Address to bind on.
    #[arg(long, default_value = "127.0.0.1:3030")]
    addr: String,
    /// Directory containing index.html, style.css, app.js, sprites/.
    /// Defaults to `<project-root>/web/static` (auto-detected).
    #[arg(long)]
    static_dir: Option<PathBuf>,
    /// Patch JSON to load at startup.
    /// Defaults to `<project-root>/patches/default.json`.
    #[arg(long)]
    patch: Option<PathBuf>,
}

#[derive(Clone)]
struct AppState {
    patch: Arc<RwLock<Patch>>,
}

/// Walk upward from `start` looking for a directory containing `web/static`.
fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut cur = Some(start.to_path_buf());
    while let Some(dir) = cur {
        if dir.join("web").join("static").is_dir() {
            return Some(dir);
        }
        cur = dir.parent().map(|p| p.to_path_buf());
    }
    None
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Resolve project root: prefer cwd, fall back to directory of the executable.
    let cwd = std::env::current_dir().ok();
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));
    let project_root = cwd
        .as_deref()
        .and_then(find_project_root)
        .or_else(|| exe_dir.as_deref().and_then(find_project_root));

    let static_dir = args
        .static_dir
        .or_else(|| project_root.as_ref().map(|r| r.join("web/static")))
        .unwrap_or_else(|| PathBuf::from("web/static"));
    let patch_path = args
        .patch
        .or_else(|| project_root.as_ref().map(|r| r.join("patches/default.json")))
        .unwrap_or_else(|| PathBuf::from("patches/default.json"));

    if !static_dir.is_dir() {
        anyhow::bail!(
            "static dir not found: {} (run from the project root, or pass --static-dir)",
            static_dir.display()
        );
    }

    let patch = if patch_path.exists() {
        let s = std::fs::read_to_string(&patch_path)?;
        serde_json::from_str::<Patch>(&s).unwrap_or_default()
    } else {
        Patch::default()
    };

    let state = AppState {
        patch: Arc::new(RwLock::new(patch)),
    };

    let app = Router::new()
        .route("/api/patch", get(get_patch).put(put_patch))
        .route("/api/render", post(render_midi))
        .route("/api/note", get(render_note))
        .fallback_service(ServeDir::new(&static_dir))
        .with_state(state);

    let addr: SocketAddr = args.addr.parse()?;
    println!("rubysynth web UI listening on http://{}", addr);
    println!("  static dir: {}", static_dir.display());
    println!("  patch:      {}", patch_path.display());
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn get_patch(State(s): State<AppState>) -> Json<Patch> {
    Json(s.patch.read().await.clone())
}

async fn put_patch(
    State(s): State<AppState>,
    Json(patch): Json<Patch>,
) -> Result<Json<&'static str>, ApiError> {
    *s.patch.write().await = patch;
    Ok(Json("ok"))
}

#[derive(Deserialize)]
struct NoteQuery {
    note: u8,
    #[serde(default = "default_velocity")]
    velocity: u8,
    #[serde(default = "default_hold_ms")]
    hold_ms: u32,
}
fn default_velocity() -> u8 { 100 }
fn default_hold_ms() -> u32 { 350 }

async fn render_note(
    State(s): State<AppState>,
    Query(q): Query<NoteQuery>,
) -> Result<Response, ApiError> {
    let patch = s.patch.read().await.clone();
    let hold_samples = (q.hold_ms as u64) * (SAMPLE_RATE as u64) / 1000;
    let tail_samples = (SAMPLE_RATE as u64) * 2; // 2s release tail
    let score = rubysynth::midi_player::Score::single_note(
        q.note,
        q.velocity,
        hold_samples,
        tail_samples,
    );

    let samples = tokio::task::spawn_blocking(move || {
        let mut engine = Engine::new(SAMPLE_RATE);
        engine.render_score(&patch, &score)
    })
    .await
    .map_err(ApiError::internal)?;

    let wav_bytes = encode_wav(&samples, SAMPLE_RATE).map_err(ApiError::internal)?;
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "audio/wav"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        Body::from(wav_bytes),
    )
        .into_response())
}

async fn render_midi(
    State(s): State<AppState>,
    mut multipart: Multipart,
) -> Result<Response, ApiError> {
    let mut midi_bytes: Option<Vec<u8>> = None;
    while let Some(field) = multipart.next_field().await.map_err(ApiError::bad)? {
        if field.name() == Some("midi") {
            let bytes = field.bytes().await.map_err(ApiError::bad)?.to_vec();
            midi_bytes = Some(bytes);
        }
    }
    let midi_bytes = midi_bytes.ok_or_else(|| ApiError::bad("missing 'midi' field"))?;

    let patch = s.patch.read().await.clone();
    let score = midi_player::load_midi(&midi_bytes, SAMPLE_RATE, 2.0).map_err(ApiError::bad)?;

    let samples = tokio::task::spawn_blocking(move || {
        let mut engine = Engine::new(SAMPLE_RATE);
        engine.render_score(&patch, &score)
    })
    .await
    .map_err(ApiError::internal)?;

    let wav_bytes = encode_wav(&samples, SAMPLE_RATE).map_err(ApiError::internal)?;

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "audio/wav"),
            (header::CONTENT_DISPOSITION, "attachment; filename=render.wav"),
        ],
        Body::from(wav_bytes),
    )
        .into_response())
}

fn encode_wav(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>> {
    // samples is interleaved L,R,L,R,...
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let peak = samples.iter().fold(0.0_f32, |a, b| a.max(b.abs()));
    let norm = if peak > 1.0 { 0.98 / peak } else { 1.0 };

    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut buf, spec)?;
        for s in samples {
            let v = (s * norm).clamp(-1.0, 1.0);
            let i = (v * i16::MAX as f32) as i16;
            writer.write_sample(i)?;
        }
        writer.finalize()?;
    }
    Ok(buf.into_inner())
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad<E: std::fmt::Display>(e: E) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: e.to_string(),
        }
    }
    fn internal<E: std::fmt::Display>(e: E) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: e.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, self.message).into_response()
    }
}
