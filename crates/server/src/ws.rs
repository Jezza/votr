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

type Shared<T> = Arc<Mutex<T>>;

#[derive(Clone)]
pub struct AppState {
    pub lobbies: Shared<HashMap<types::LobbyId, Shared<Lobby>>>,
}

impl AppState {
    pub async fn find_lobby(&self, lobby_id: &types::LobbyId) -> Option<Shared<Lobby>> {
        let lobbies = self.lobbies.lock().await;
        lobbies.get(lobby_id).cloned()
    }

    pub async fn remove_lobby(&self, lobby_id: &types::LobbyId) -> Option<Shared<Lobby>> {
        let mut lobbies = self.lobbies.lock().await;
        lobbies.remove(lobby_id)
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

    let Some(lobby) = state.find_lobby(&info.lobby_id).await else {
        warn!("someone sent a request without a `lobby_id`");
        return StatusCode::BAD_REQUEST.into_response();
    };

    ws.on_upgrade(|socket| handle_socket(socket, state, info, lobby))
}

macro_rules! send {
    ($sender:expr, $msg:expr) => {{
        let msg = types::Outgoing::from($msg);

        match serde_json::to_string(&msg) {
            Ok(value) => {
                let _ = $sender.send(Message::Text(value.into())).await;
                Ok(())
            }
            Err(err) => {
                ::tracing::error!("unable to serialise message: {}", err);
                Err(())
            }
        }
    }};
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    info: types::JoinInfo,
    mut lobby: Shared<Lobby>,
) {
    let (mut sender, mut receiver) = socket.split();

    let outcome = {
        let mut lobby = lobby.lock().await;
        lobby.join(&info)
    };

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
                    Ok(msg) => {
                        let closed = matches!(msg, types::Outgoing::LobbyClosed(_));

                        if send!(sender, msg).is_err() {
                            break;
                        }

                        if closed {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        send!(sender, types::LobbyClosed {});
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
    {
        
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
    }

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
    lobby: Shared<Lobby>,
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

    match value {
        types::Incoming::SetName(msg) => {
            let player_id = msg.player_id.as_ref().unwrap_or(&info.player_id);

            let mut lobby = lobby.lock().await;
            lobby.set_name(&player_id, msg.name);
        }
        types::Incoming::AddGame(msg) => {
            let mut lobby = lobby.lock().await;
            lobby.add_game(&info.player_id, msg.name);
        }
        types::Incoming::RemoveGame(msg) => {
            let mut lobby = lobby.lock().await;
            lobby.remove_game(&info.player_id, &msg.game_id);
        }
        types::Incoming::VetoGame(msg) => {
            let mut lobby = lobby.lock().await;
            lobby.veto_game(&info.player_id, &msg.game_id);
        }
        types::Incoming::UnvetoGame(msg) => {
            let mut lobby = lobby.lock().await;
            lobby.unveto_game(&info.player_id, &msg.game_id);
        }
        types::Incoming::SubmitVote(msg) => {
            let mut lobby = lobby.lock().await;
            lobby.submit_vote(&info.player_id, msg.ranking);
        }
        types::Incoming::SetReady(msg) => {
            let mut lobby = lobby.lock().await;
            lobby.set_ready(&info.player_id, msg.ready);
        }
        types::Incoming::AdvancePhase => {
            let mut lobby = lobby.lock().await;
            lobby.advance_phase();
        }
        types::Incoming::ResetSession => {
            let mut lobby = lobby.lock().await;
            lobby.reset();
        }
        types::Incoming::SetMaxVetoes(msg) => {
            let mut lobby = lobby.lock().await;
            lobby.set_max_vetoes(msg.count);
        }
        types::Incoming::KickPlayer(msg) => {
            let mut lobby = lobby.lock().await;
            lobby.kick_player(&msg.target_id);
        }
        types::Incoming::SetLobbyPublic(msg) => {
            let mut lobby = lobby.lock().await;
            lobby.set_lobby_public(&info.player_id, msg.public);
        }
        types::Incoming::SetLobbyPassword(msg) => {
            let mut lobby = lobby.lock().await;
            lobby.set_lobby_password(&info.player_id, msg.password);
        }
        types::Incoming::SetLobbyLocked(msg) => {
            let mut lobby = lobby.lock().await;
            lobby.set_lobby_locked(&info.player_id, msg.locked);
        }
        types::Incoming::CloseLobby => {
            let mut lobby = lobby.lock().await;
            lobby.close(&info.player_id);
        }
    }

    // let state_json = serialize_state(&lobby_guard);
    // let _ = lobby_guard.tx.send(state_json);
}
