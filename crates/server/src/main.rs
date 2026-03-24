mod lobby;
mod session;
mod ws;

use axum::{
    extract::State,
    http::StatusCode,
    routing::get,
    Json, Router,
};
use lobby::{LobbyManager, LOBBY_CLEANUP_INTERVAL, LOBBY_EMPTY_TIMEOUT};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing::info;
use ws::{ws_handler, AppState};

#[derive(rust_embed::RustEmbed, Clone, Copy)]
#[folder = "../../ui/dist"]
pub struct Assets;

#[derive(Deserialize)]
struct CreateLobbyRequest {
    public: Option<bool>,
    password: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let lobbies = Arc::new(Mutex::new(LobbyManager::new()));
    let state = AppState {
        lobbies: lobbies.clone(),
    };

    // Spawn cleanup task — every 10s, remove lobbies empty for 60s+
    let cleanup_lobbies = lobbies.clone();
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(LOBBY_CLEANUP_INTERVAL));
        loop {
            interval.tick().await;
            let mut manager = cleanup_lobbies.lock().await;
            let mut to_remove = Vec::new();
            for (id, lobby_arc) in manager.lobbies.iter() {
                if let Ok(mut lobby) = lobby_arc.try_lock() {
                    if !lobby.has_connected_players() {
                        if let Some(last_empty) = lobby.last_empty {
                            if last_empty.elapsed().as_secs() >= LOBBY_EMPTY_TIMEOUT {
                                to_remove.push(id.clone());
                            }
                        } else {
                            lobby.last_empty = Some(tokio::time::Instant::now());
                        }
                    } else {
                        lobby.last_empty = None;
                    }
                }
            }
            for id in &to_remove {
                info!("Removing empty lobby {}", id);
                manager.remove_lobby(id);
            }
        }
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/api/lobbies", get(list_lobbies).post(create_lobby))
        .fallback_service(axum_embed::ServeEmbed::<Assets>::new())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    info!("listening on http://0.0.0.0:3001");
    axum::serve(listener, app).await.unwrap();
}

async fn list_lobbies(State(state): State<AppState>) -> Json<Vec<lobby::LobbyInfo>> {
    let manager = state.lobbies.lock().await;
    let mut lobbies = Vec::new();
    for lobby_arc in manager.lobbies.values() {
        if let Ok(lobby) = lobby_arc.try_lock() {
            if lobby.public {
                lobbies.push(lobby::LobbyInfo {
                    id: lobby.id.clone(),
                    name: lobby.name.clone(),
                    player_count: lobby.session.players.iter().filter(|p| p.connected).count(),
                    max_players: session::MAX_PLAYERS,
                    has_password: lobby.password.is_some(),
                    locked: lobby.locked,
                    phase: format!("{:?}", lobby.session.phase).to_lowercase(),
                });
            }
        }
    }
    Json(lobbies)
}

async fn create_lobby(
    State(state): State<AppState>,
    Json(req): Json<CreateLobbyRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut manager = state.lobbies.lock().await;
    let password = req.password.and_then(|p| {
        let trimmed: String = p.chars().take(64).collect();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });
    match manager.create_lobby(req.public.unwrap_or(true), password) {
        Ok((id, name)) => Ok(Json(serde_json::json!({ "id": id, "name": name }))),
        Err(_) => Err(StatusCode::TOO_MANY_REQUESTS),
    }
}
