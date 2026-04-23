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
}

pub async fn handler(
    ws: WebSocketUpgrade,
    Query(mut info): Query<types::JoinInfo>,
    State(state): State<AppState>,
) -> Response {
    crate::trim_in_place(&mut info.name);
    if info.name.is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let Some(lobby) = state.find_lobby(&info.lobby_id).await else {
        warn!("no lobby found with {}", info.lobby_id);
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
    lobby: Shared<Lobby>,
) {
    let (mut sender, mut receiver) = socket.split();

    let outcome = {
        let mut lobby = lobby.lock().await;
        lobby.join(&info)
    };

    let types::JoinInfo {
        player_id,
        name: _,
        lobby_id,
        password: _,
    } = info;

    let rx = match outcome {
        JoinOutcome::Joined(rx, _rejoined) => rx,
        JoinOutcome::Locked => {
            info!(
                player_id = %player_id,
                lobby_id = %lobby_id,
                "lobby is locked"
            );
            send!(sender, types::Toast::error("Lobby is locked"));
            return;
        }
        JoinOutcome::Kicked => {
            info!(
                player_id = %player_id,
                lobby_id = %lobby_id,
                "played attempt to rejoin after being kicked"
            );
            send!(sender, types::Kicked {});
            return;
        }
        JoinOutcome::LobbyFull => {
            info!(
                player_id = %player_id,
                lobby_id = %lobby_id,
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
                player_id = %player_id,
                lobby_id = %lobby_id,
                "incorrect password"
            );
            send!(sender, types::Toast::error("Incorrect password"));
            return;
        }
    };

    let mut rx = rx;

    let closed = loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_message(state.clone(), &mut sender, lobby.clone(), &text, player_id).await;
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break false;
                    },
                    _ => {}
                }
            }
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        let closed = matches!(msg, types::Outgoing::LobbyClosed(_));

                        if send!(sender, msg).is_err() {
                            break false;
                        }

                        if closed {
                            break true;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        send!(sender, types::LobbyClosed {});
                        break true;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Skip lagged messages
                    }
                }
            }
        }
    };

    tracing::info!("disconnecting player");

    {
        //     info!("Player {} disconnected from lobby {}", player_id, lobby_id);

        let mut lobby = lobby.lock().await;
        if closed {
            lobby.remove_player(player_id)
        } else {
            lobby.disconnect_player(player_id)
        }

        lobby.send_state();
    }

    if closed {
        return;
    }

    tokio::spawn({
        let player_id = player_id;
        let lobby = lobby.clone();

        async move {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;

            let lobby = lobby.lock().await;

        }
    });
}

/// Returns Some(error_json) if an error message should be sent back to the client only,
/// or None if the message was handled (broadcast already done inside).
async fn handle_message(
    state: AppState,
    sender: &mut SplitSink<WebSocket, Message>,
    lobby: Shared<Lobby>,
    text: &str,
    player_id: types::PlayerId,
) {
    let value: types::Incoming = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => {
            send!(sender, types::Error::new("invalid JSON"));
            return;
        }
    };

    let mut lobby = lobby.lock().await;

    match value {
        types::Incoming::SetName(msg) => {
            lobby.set_name(player_id, msg.name);
        }
        types::Incoming::AddGame(msg) => {
            lobby.add_game(player_id, msg.name);
        }
        types::Incoming::RemoveGame(msg) => {
            lobby.remove_game(player_id, msg.game_id);
        }
        types::Incoming::VetoGame(msg) => {
            lobby.veto_game(player_id, &msg.game_id);
        }
        types::Incoming::UnvetoGame(msg) => {
            lobby.unveto_game(player_id, &msg.game_id);
        }
        types::Incoming::SubmitVote(msg) => {
            lobby.submit_vote(player_id, msg.ranking);
        }
        types::Incoming::SetReady(msg) => {
            lobby.set_ready(player_id, msg.ready);
        }
        types::Incoming::AdvancePhase => {
            lobby.advance_phase(player_id);
        }
        types::Incoming::ResetSession => {
            lobby.reset(player_id);
        }
        types::Incoming::SetMaxVetoes(msg) => {
            lobby.set_max_vetoes(player_id, msg.count);
        }
        types::Incoming::KickPlayer(msg) => {
            lobby.kick_player(player_id, msg.target_id);
        }
        types::Incoming::SetLobbyPublic(msg) => {
            lobby.set_lobby_public(player_id, msg.public);
        }
        types::Incoming::SetLobbyPassword(msg) => {
            lobby.set_lobby_password(player_id, msg.password);
        }
        types::Incoming::SetLobbyLocked(msg) => {
            lobby.set_lobby_locked(player_id, msg.locked);
        }
        types::Incoming::CloseLobby => {
            if lobby.close(player_id) {
                state.remove_lobby(&lobby.id).await;
            }
        }
    }

    lobby.send_state();
}
