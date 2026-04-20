use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use lobby::{LOBBY_CLEANUP_INTERVAL, LOBBY_EMPTY_TIMEOUT, LobbyManager};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing::info;
use ws::{AppState, handler};

mod lobby;
mod lobby;
mod ws;
mod types;

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

    // let lobbies = Arc::new(Mutex::new(LobbyManager::new()));
    let state = AppState {
        // loppies: Default::default(),
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
        // .route("/api/lobbies", get(list_lobbies).post(create_lobby))
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

// async fn list_lobbies(State(state): State<AppState>) -> Json<Vec<lobby::LobbyInfo>> {
//     let manager = state.lobbies.lock().await;
//     let mut lobbies = Vec::new();
//     for lobby_arc in manager.lobbies.values() {
//         let Ok(lobby) = lobby_arc.try_lock() else {
//             continue;
//         };
//
//         if !lobby.public {
//             continue;
//         }
//
//         let player_count = lobby
//             .session
//             .players
//             .iter()
//             .filter(|p| p.is_connected())
//             .count();
//
//         lobbies.push(lobby::LobbyInfo {
//             id: lobby.id.clone(),
//             name: lobby.name.clone(),
//             player_count,
//             max_players: session::MAX_PLAYERS,
//             has_password: lobby.password.is_some(),
//             locked: lobby.locked,
//             phase: format!("{:?}", lobby.session.phase).to_lowercase(),
//         });
//     }
//     Json(lobbies)
// }
//
// async fn create_lobby(
//     State(state): State<AppState>,
//     Json(req): Json<CreateLobbyRequest>,
// ) -> Result<Json<serde_json::Value>, StatusCode> {
//     let mut manager = state.lobbies.lock().await;
//
//     let public = req.public.unwrap_or(true);
//     let password = req.password.and_then(|mut pw| {
//         pw.truncate(64);
//         (!pw.is_empty()).then_some(pw)
//     });
//
//     match manager.create_lobby(public, password) {
//         Ok((id, name)) => {
//             info!(lobby_id = id, lobby_name = name, public, "lobby created");
//             Ok(Json(serde_json::json!({ "id": id, "name": name })))
//         }
//         Err(_) => {
//             info!("lobby creation rejected: too many lobbies");
//             Err(StatusCode::TOO_MANY_REQUESTS)
//         }
//     }
// }
