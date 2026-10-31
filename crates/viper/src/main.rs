//! Battlesnake webhook server. Pure logic lives in the library crate
//! (`lib.rs` + `core` module) so it stays unit-testable; this binary only
//! binds axum routes and adapts HTTP to `engine::choose`.
//!
//! Run: `PORT=8000 cargo run -p snake`

use axum::{Json, Router, routing::post};
use std::{
    net::SocketAddr,
    sync::atomic::{AtomicU64, Ordering},
};
use viper::{GameState, MoveResponse, engine};

static TURNS: AtomicU64 = AtomicU64::new(0);

async fn info() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "apiversion": "1",
        "author": "WyattAu",
        "color": "#0ea5e9",
        "head": "sand-worm",
        "tail": "bolt",
        "version": "0.1.0"
    }))
}

async fn start() -> &'static str {
    ""
}

async fn end() -> &'static str {
    ""
}

async fn choose_move(Json(state): Json<GameState>) -> Json<MoveResponse> {
    TURNS.fetch_add(1, Ordering::Relaxed);
    let (dir, shout) = engine::choose(&state);
    Json(MoveResponse {
        direction: dir,
        shout: (!shout.is_empty()).then(|| shout.to_string()),
    })
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000);
    let app = Router::new()
        .route("/", post(info))
        .route("/start", post(start))
        .route("/move", post(choose_move))
        .route("/end", post(end));
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    println!("wyatt-viper listening on {addr}");
    axum::serve(listener, app).await.expect("serve");
}
