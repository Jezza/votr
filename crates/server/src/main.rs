use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
// use lobby::{LOBBY_CLEANUP_INTERVAL, LOBBY_EMPTY_TIMEOUT, LobbyManager};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing::info;
use ws::{AppState, handler};

mod lobby;
mod types;
mod ws;

#[derive(rust_embed::RustEmbed, Clone, Copy)]
#[folder = "../../ui/dist"]
pub struct Assets;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // let lobbies = Arc::new(Mutex::new(LobbyManager::new()));
    let state = AppState {
        lobbies: Default::default(),
    };

    // // Spawn cleanup task — every 10s, remove lobbies empty for 60s+
    // tokio::spawn({
    //     let lobbies = state.loggies.clone();
    //
    //     async move {
    //         let mut interval =
    //             tokio::time::interval(std::time::Duration::from_secs(LOBBY_CLEANUP_INTERVAL));
    //         loop {
    //             interval.tick().await;
    //             let mut manager = lobbies.lock().await;
    //             let mut to_remove = Vec::new();
    //             for (id, lobby_arc) in manager.lobbies.iter() {
    //                 if let Ok(mut lobby) = lobby_arc.try_lock() {
    //                     if !lobby.has_connected_players() {
    //                         if let Some(last_empty) = lobby.last_empty {
    //                             if last_empty.elapsed().as_secs() >= LOBBY_EMPTY_TIMEOUT {
    //                                 to_remove.push(id.clone());
    //                             }
    //                         } else {
    //                             lobby.last_empty = Some(tokio::time::Instant::now());
    //                         }
    //                     } else {
    //                         lobby.last_empty = None;
    //                     }
    //                 }
    //             }
    //             for id in &to_remove {
    //                 info!("Removing empty lobby {}", id);
    //                 manager.remove_lobby(id);
    //             }
    //         }
    //     }
    // });

    let app = Router::new()
        .route("/ws", get(handler))
        .route("/api/lobbies", get(list_lobbies).post(create_lobby))
        .fallback_service(axum_embed::ServeEmbed::<Assets>::new())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    info!(
        "listening on {}",
        listener
            .local_addr()
            .expect("unable to discover local addr")
    );
    axum::serve(listener, app).await.unwrap();
}

async fn list_lobbies(State(state): State<AppState>) -> Json<Vec<types::LobbyInfo>> {
    let lobbies: Vec<_> = {
        let lobbies = state.lobbies.lock().await;
        lobbies.values().cloned().collect()
    };

    let mut info = vec![];

    for lobby in lobbies {
        let lobby = lobby.lock().await;

        info.push(types::LobbyInfo {
            id: lobby.id,
            name: lobby.name.clone(),
            player_count: lobby.players.len(),
            max_players: lobby::MAX_PLAYERS,
            has_password: lobby.password.is_some(),
            locked: lobby.locked,
            phase: lobby.phase,
        });
    }
    Json(info)
}

async fn create_lobby(
    State(state): State<AppState>,
    Json(req): Json<types::CreateLobbyRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let public = req.public.unwrap_or(true);
    let password = match req.password {
        Some(pw) if pw.len() > 64 => {
            return Err(StatusCode::BAD_REQUEST);
        }
        Some(pw) if pw.is_empty() => {
            return Err(StatusCode::BAD_REQUEST);
        }
        pw => pw,
    };

    let lobby = lobby::Lobby::new(public, password);
    let id = lobby.id;

    // @TODO jezza - 21 Apr 2026: Replace this with a real type
    let response = serde_json::json!({ "id": id, "name": lobby.name });

    {
        let mut lobbies = state.lobbies.lock().await;
        lobbies.insert(id, Arc::new(Mutex::new(lobby)));
    }

    Ok(Json(response))
}

fn trim_in_place(s: &mut String) {
    let trimmed = s.trim();
    let start = trimmed.as_ptr() as usize - s.as_ptr() as usize;
    let len = trimmed.len();
    s.truncate(start + len);
    s.drain(..start);
}
