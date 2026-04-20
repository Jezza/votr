use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tracing::{debug, info, warn};

use crate::lobby::{JoinOutcome, Lobby, MAX_PLAYERS};
use crate::types;

pub const MAX_LOBBIES: usize = 128;
pub const LOBBY_EMPTY_TIMEOUT: u64 = 60;
pub const LOBBY_CLEANUP_INTERVAL: u64 = 10;

#[derive(Clone)]
pub struct AppState {
    pub lobbies: Arc<Mutex<HashMap<types::LobbyId, Lobby>>>,
    // pub lobbies: Arc<Mutex<LobbyManager>>,
    // pub loggies: LoggyManager,
    // pub loppies: LoggyManager,
}

impl AppState {
    pub fn find_lobby(&self, lobby_id: &types::LobbyId) -> Option<Lobby> {
        // let types::JoinInfo {
        //     player_id,
        //     name,
        //     lobby_id,
        //     password,
        // } = info;

        // JoinOutcome::Kicked
        None
    }

    pub fn create_lobby(
        &mut self,
        public: bool,
        password: Option<String>,
    ) -> Result<(String, String), &'static str> {
        // if self.lobbies.len() >= MAX_LOBBIES {
        //     return Err("too many lobbies");
        // }
        // let lobby = Lobby::new(public, password);
        // self.lobbies.insert(id.clone(), Arc::new(Mutex::new(lobby)));
        // Ok((id, name))
        Err("test")
    }

    // pub fn remove_lobby(&mut self, id: &str) {
    //     self.lobbies.remove(id);
    // }
    //
    // pub fn get_lobby(&self, id: &str) -> Option<Arc<Mutex<Lobby>>> {
    //     self.lobbies.get(id).cloned()
    // }
}

// #[derive(Serialize)]
// struct StateMessage<'a> {
//     #[serde(rename = "type")]
//     msg_type: &'static str,
//     phase: &'a Phase,
//     players: &'a Vec<Player>,
//     games: &'a Vec<Opt>,
//     votes_submitted: &'a Vec<String>,
//     results: &'a Option<Vec<VoteResult>>,
//     host_id: Option<&'a str>,
//     max_vetoes: u32,
//     lobby_id: &'a str,
//     lobby_name: &'a str,
//     lobby_public: bool,
//     lobby_locked: bool,
//     lobby_has_password: bool,
// }
//
// fn serialize_state(lobby: &Lobby) -> String {
//     let msg = StateMessage {
//         msg_type: "state",
//         phase: &lobby.phase,
//         players: &lobby.players,
//         games: &lobby.options,
//         votes_submitted: &lobby.votes_submitted,
//         results: &lobby.results,
//         host_id: lobby.host_id.as_deref(),
//         max_vetoes: lobby.max_vetoes,
//         lobby_id: &lobby.id,
//         lobby_name: &lobby.name,
//         lobby_public: lobby.public,
//         lobby_locked: lobby.locked,
//         lobby_has_password: lobby.password.is_some(),
//     };
//     serde_json::to_string(&msg)
//         .unwrap_or_else(|_| r#"{"type":"error","message":"serialization error"}"#.to_string())
// }

pub async fn handler(
    ws: WebSocketUpgrade,
    Query(mut params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    let Some(player_id) = params.remove("player_id") else {
        warn!("someone sent a request without a `player_id`");
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Some(name) = params.remove("name") else {
        warn!("someone sent a request without a `name`");
        return StatusCode::BAD_REQUEST.into_response();
    };
    if name.trim().is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let Some(lobby_id) = params.remove("lobby_id") else {
        warn!("someone sent a request without a `lobby_id`");
        return StatusCode::BAD_REQUEST.into_response();
    };
    let password = params.remove("password");

    let info = types::JoinInfo {
        player_id: types::PlayerId(player_id),
        name,
        lobby_id: types::LobbyId(lobby_id),
        password,
    };

    let Some(lobby) = state.find_lobby(&info.lobby_id) else {
        warn!("someone sent a request without a `lobby_id`");
        return StatusCode::BAD_REQUEST.into_response();
    };

    ws.on_upgrade(|socket| handle_socket(socket, state, info, lobby))
}

macro_rules! send {
    ($sender:expr, $msg:expr) => {{
        let msg = types::Outgoing::from($msg);

        let value = match serde_json::to_string(&msg) {
            Ok(value) => value,
            Err(err) => {
                ::tracing::error!("unable to serialise message: {}", err);
                return;
            }
        };

        let _ = $sender.send(Message::Text(value.into())).await;
    }};
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    info: types::JoinInfo,
    mut lobby: Lobby,
) {
    let (mut sender, mut receiver) = socket.split();

    // let t = send!(sender, "");

    // let Some(lobby) = state.find_lobby(&info.lobby_id) else {
    //     info!(
    //         player_id = info.player_id,
    //         lobby_id = info.lobby_id,
    //         "unable to find lobby"
    //     );
    //     send!(sender, types::Toast::error("Lobby not found"));
    //     return;
    // };

    let outcome = lobby.join(&info);

    let rx = match outcome {
        JoinOutcome::Joined(rx, _rejoined) => rx,
        JoinOutcome::Locked => {
            info!(
                player_id = %info.player_id,
                lobby_id = %info.lobby_id,
                "lobby is locked"
            );
            send!(sender, types::Toast::error("Lobby is locked"));
            return;
        }
        JoinOutcome::Kicked => {
            info!(
                player_id = %info.player_id,
                lobby_id = %info.lobby_id,
                "played attempt to rejoin after being kicked"
            );
            send!(sender, types::Kicked {});
            return;
        }
        JoinOutcome::LobbyFull => {
            info!(
                player_id = %info.player_id,
                lobby_id = %info.lobby_id,
                "lobby is full"
            );
            send!(
                sender,
                types::Toast::warn(format!("Lobby is full (max {} players)", MAX_PLAYERS))
            );
            return;
        }
        JoinOutcome::IncorrectPassword => {
            info!(
                player_id = %info.player_id,
                lobby_id = %info.lobby_id,
                "incorrect password"
            );
            send!(sender, types::Toast::error("Incorrect password"));
            return;
        }
    };

    // Send welcome message to this client only
    // let welcome = serde_json::json!({
    //     "type": "welcome",
    //     "player_id": player_id,
    //     "lobby_id": lobby_id,
    // });
    // if sender
    //     .send(Message::Text(welcome.to_string().into()))
    //     .await
    //     .is_err()
    // {
    //     return;
    // }

    let mut rx = rx;

    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_message(state.clone(), &mut sender, lobby.clone(), &text, &info).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            result = rx.recv() => {
                match result {
                    Ok(broadcast_msg) => {
                        if sender.send(Message::Text(broadcast_msg.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        let msg = r#"{"type":"lobby_closed"}"#;
                        let _ = sender.send(Message::Text(msg.into())).await;
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Skip lagged messages
                    }
                }
            }
        }
    }

    // Player disconnected — mark as disconnected
    // {
    //     let mut lobby = lobby_arc.lock().await;
    //     lobby.session.remove_player(&player_id);
    //     info!("Player {} disconnected from lobby {}", player_id, lobby_id);
    //
    //     // Set last_empty if no connected players remain
    //     if !lobby.has_connected_players() {
    //         lobby.last_empty = Some(Instant::now());
    //     }
    //
    //     let state_json = serialize_state(&lobby);
    //     let _ = lobby.tx.send(state_json);
    // }

    // After 20s, fully remove the player if they haven't reconnected
    // let timeout_lobby = lobby_arc.clone();
    // let timeout_id = player_id.clone();
    // tokio::spawn(async move {
    //     tokio::time::sleep(std::time::Duration::from_secs(20)).await;
    //     let mut lobby = timeout_lobby.lock().await;
    //     let still_disconnected = lobby
    //         .session
    //         .players
    //         .iter()
    //         .any(|p| p.id == timeout_id && !p.connected);
    //     if still_disconnected {
    //         info!("Player {} timed out, removing from lobby", timeout_id);
    //         lobby.session.players.retain(|p| p.id != timeout_id);
    //         // Clean up their data
    //         lobby.session.votes.remove(&timeout_id);
    //         lobby.session.votes_submitted.retain(|id| id != &timeout_id);
    //         lobby.session.games.retain(|g| g.suggested_by != timeout_id);
    //         for game in lobby.session.games.iter_mut() {
    //             game.vetoed_by.retain(|id| id != &timeout_id);
    //         }
    //         // Reassign host if needed
    //         if lobby.session.host_id.as_deref() == Some(&timeout_id) {
    //             lobby.session.host_id = lobby
    //                 .session
    //                 .players
    //                 .iter()
    //                 .find(|p| p.connected)
    //                 .map(|p| p.id.clone());
    //         }
    //         let state_json = serialize_state(&lobby);
    //         let _ = lobby.tx.send(state_json);
    //     }
    // });
}

/// Returns Some(error_json) if an error message should be sent back to the client only,
/// or None if the message was handled (broadcast already done inside).
async fn handle_message(
    state: AppState,
    sender: &mut SplitSink<WebSocket, Message>,
    lobby: Lobby,
    text: &str,
    info: &types::JoinInfo,
) {
    let value: types::Incoming = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => {
            send!(sender, types::Error::new("invalid JSON"));
            return;
        }
    };

    // let msg_type = match value.get("type").and_then(|v| v.as_str()) {
    //     Some(t) => t.to_string(),
    //     None => {
    //         return Some(r#"{"type":"error","message":"missing type field"}"#.to_string());
    //     }
    // };
    //
    // let mut lobby_guard = lobby.lock().await;
    // let is_host = lobby_guard.session.get_host_id() == Some(player_id);
    // let lobby_id = lobby_guard.id.clone();
    //
    // debug!(player_id, %lobby_id, %msg_type, "received message");
    //
    // match msg_type.as_str() {
    //     "set_name" => {
    //         let name = value
    //             .get("name")
    //             .and_then(|v| v.as_str())
    //             .unwrap_or("")
    //             .to_string();
    //         if let Some(player) = lobby_guard
    //             .session
    //             .players
    //             .iter_mut()
    //             .find(|p| p.id == player_id)
    //         {
    //             if !name.trim().is_empty() {
    //                 player.name = name;
    //             }
    //         }
    //     }
    //     "add_game" => {
    //         let name = value
    //             .get("name")
    //             .and_then(|v| v.as_str())
    //             .unwrap_or("")
    //             .to_string();
    //         lobby_guard.session.add_game(player_id, &name);
    //     }
    //     "remove_game" => {
    //         let game_id = value
    //             .get("game_id")
    //             .and_then(|v| v.as_str())
    //             .unwrap_or("")
    //             .to_string();
    //         lobby_guard.session.remove_game(player_id, &game_id);
    //     }
    //     "veto_game" => {
    //         let game_id = value
    //             .get("game_id")
    //             .and_then(|v| v.as_str())
    //             .unwrap_or("")
    //             .to_string();
    //         lobby_guard.session.veto_game(player_id, &game_id);
    //     }
    //     "unveto_game" => {
    //         let game_id = value
    //             .get("game_id")
    //             .and_then(|v| v.as_str())
    //             .unwrap_or("")
    //             .to_string();
    //         lobby_guard.session.unveto_game(player_id, &game_id);
    //     }
    //     "submit_vote" => {
    //         let ranking: Vec<String> = value
    //             .get("ranking")
    //             .and_then(|v| v.as_array())
    //             .map(|arr| {
    //                 arr.iter()
    //                     .filter_map(|v| v.as_str().map(|s| s.to_string()))
    //                     .collect()
    //             })
    //             .unwrap_or_default();
    //         let all_voted = lobby_guard.session.submit_vote(player_id, ranking);
    //         if all_voted {
    //             lobby_guard.session.advance_phase();
    //         }
    //     }
    //     "set_ready" => {
    //         let ready = value
    //             .get("ready")
    //             .and_then(|v| v.as_bool())
    //             .unwrap_or(false);
    //         lobby_guard.session.set_ready(player_id, ready);
    //     }
    //     "advance_phase" => {
    //         if !is_host {
    //             return Some(
    //                 r#"{"type":"error","message":"only the host can advance the phase"}"#
    //                     .to_string(),
    //             );
    //         }
    //         lobby_guard.session.advance_phase();
    //         info!(lobby_id, phase = ?lobby_guard.session.phase, "phase advanced");
    //     }
    //     "reset_session" => {
    //         if !is_host {
    //             return Some(
    //                 r#"{"type":"error","message":"only the host can reset the session"}"#
    //                     .to_string(),
    //             );
    //         }
    //         lobby_guard.session.reset();
    //         info!(lobby_id, "session reset");
    //     }
    //     "kick_player" => {
    //         if !is_host {
    //             return Some(
    //                 r#"{"type":"error","message":"only the host can kick players"}"#.to_string(),
    //             );
    //         }
    //         let target_id = value
    //             .get("target_id")
    //             .and_then(|v| v.as_str())
    //             .unwrap_or("")
    //             .to_string();
    //         if target_id == player_id {
    //             return Some(
    //                 r#"{"type":"error","message":"you cannot kick yourself"}"#.to_string(),
    //             );
    //         }
    //         lobby_guard.session.kick_player(&target_id);
    //         info!(lobby_id, target_id, "player kicked");
    //     }
    //     "set_max_vetoes" => {
    //         if !is_host {
    //             return Some(
    //                 r#"{"type":"error","message":"only the host can change veto count"}"#
    //                     .to_string(),
    //             );
    //         }
    //         let count = value.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
    //         lobby_guard.session.set_max_vetoes(count);
    //     }
    //     "set_lobby_public" => {
    //         if !is_host {
    //             return Some(
    //                 r#"{"type":"error","message":"only the host can change lobby visibility"}"#
    //                     .to_string(),
    //             );
    //         }
    //         let public = value
    //             .get("public")
    //             .and_then(|v| v.as_bool())
    //             .unwrap_or(true);
    //         lobby_guard.public = public;
    //     }
    //     "set_lobby_password" => {
    //         if !is_host {
    //             return Some(
    //                 r#"{"type":"error","message":"only the host can change the password"}"#
    //                     .to_string(),
    //             );
    //         }
    //         let pw = value
    //             .get("password")
    //             .and_then(|v| v.as_str())
    //             .unwrap_or("")
    //             .to_string();
    //         let trimmed: String = pw.chars().take(64).collect();
    //         lobby_guard.password = if trimmed.is_empty() {
    //             None
    //         } else {
    //             Some(trimmed)
    //         };
    //     }
    //     "set_lobby_locked" => {
    //         if !is_host {
    //             return Some(
    //                 r#"{"type":"error","message":"only the host can lock/unlock the lobby"}"#
    //                     .to_string(),
    //             );
    //         }
    //         let locked = value
    //             .get("locked")
    //             .and_then(|v| v.as_bool())
    //             .unwrap_or(false);
    //         lobby_guard.locked = locked;
    //     }
    //     "close_lobby" => {
    //         if !is_host {
    //             return Some(
    //                 r#"{"type":"error","message":"only the host can close the lobby"}"#.to_string(),
    //             );
    //         }
    //         info!(lobby_id, "lobby closed by host");
    //         let closed_msg = r#"{"type":"lobby_closed"}"#.to_string();
    //         let _ = lobby_guard.tx.send(closed_msg);
    //         let lobby_id_owned = lobby_guard.id.clone();
    //         drop(lobby_guard);
    //         let mut manager = state.lobbies.lock().await;
    //         manager.remove_lobby(&lobby_id_owned);
    //         return None;
    //     }
    //     _ => {
    //         return Some(r#"{"type":"error","message":"unknown message type"}"#.to_string());
    //     }
    // }

    // let state_json = serialize_state(&lobby_guard);
    // let _ = lobby_guard.tx.send(state_json);
    // None
}
