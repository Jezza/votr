use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tracing::info;

use crate::lobby::{Lobby, LobbyManager};
use crate::session::{Game, Phase, Player, VoteResult, MAX_PLAYERS};

#[derive(Clone)]
pub struct AppState {
    pub lobbies: Arc<Mutex<LobbyManager>>,
}

#[derive(Serialize)]
struct StateMessage<'a> {
    #[serde(rename = "type")]
    msg_type: &'static str,
    phase: &'a Phase,
    players: &'a Vec<Player>,
    games: &'a Vec<Game>,
    votes_submitted: &'a Vec<String>,
    results: &'a Option<Vec<VoteResult>>,
    host_id: Option<&'a str>,
    max_vetoes: u32,
    lobby_id: &'a str,
    lobby_name: &'a str,
    lobby_public: bool,
    lobby_locked: bool,
    lobby_has_password: bool,
}

fn serialize_state(lobby: &Lobby) -> String {
    let msg = StateMessage {
        msg_type: "state",
        phase: &lobby.session.phase,
        players: &lobby.session.players,
        games: &lobby.session.games,
        votes_submitted: &lobby.session.votes_submitted,
        results: &lobby.session.results,
        host_id: lobby.session.host_id.as_deref(),
        max_vetoes: lobby.session.max_vetoes,
        lobby_id: &lobby.id,
        lobby_name: &lobby.name,
        lobby_public: lobby.public,
        lobby_locked: lobby.locked,
        lobby_has_password: lobby.password.is_some(),
    };
    serde_json::to_string(&msg)
        .unwrap_or_else(|_| r#"{"type":"error","message":"serialization error"}"#.to_string())
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    let stored_id = params.get("player_id").cloned();
    let name = params.get("name").cloned().unwrap_or_default();
    let lobby_id = params.get("lobby_id").cloned().unwrap_or_default();
    let password = params.get("password").cloned();
    ws.on_upgrade(|socket| handle_socket(socket, state, stored_id, name, lobby_id, password))
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    stored_id: Option<String>,
    name: String,
    lobby_id: String,
    password: Option<String>,
) {
    // Look up the lobby Arc from the manager (brief lock)
    let lobby_arc = {
        let manager = state.lobbies.lock().await;
        manager.get_lobby(&lobby_id)
    };

    let lobby_arc = match lobby_arc {
        Some(arc) => arc,
        None => {
            let (mut sender, _) = socket.split();
            let msg = r#"{"type":"toast","message":"Lobby not found"}"#;
            let _ = sender.send(Message::Text(msg.into())).await;
            return;
        }
    };

    // Lock lobby for validation and player setup
    let player_id = {
        let mut lobby = lobby_arc.lock().await;

        // Check if kicked
        if let Some(ref id) = stored_id {
            if lobby.session.is_kicked(id) {
                drop(lobby);
                let (mut sender, _) = socket.split();
                let msg = r#"{"type":"kicked"}"#;
                let _ = sender.send(Message::Text(msg.into())).await;
                return;
            }
        }

        // Determine if this is a reconnecting player
        let is_reconnecting = stored_id.as_ref().map_or(false, |id| {
            lobby.session.players.iter().any(|p| p.id == *id)
        });

        // Check locked — but allow reconnecting players
        if lobby.locked && !is_reconnecting {
            drop(lobby);
            let (mut sender, _) = socket.split();
            let msg = r#"{"type":"toast","message":"Lobby is locked"}"#;
            let _ = sender.send(Message::Text(msg.into())).await;
            return;
        }

        // Check password — skip for reconnecting players
        if !is_reconnecting {
            if let Some(ref lobby_pw) = lobby.password {
                let provided = password.as_deref().unwrap_or("");
                if provided != lobby_pw {
                    drop(lobby);
                    let (mut sender, _) = socket.split();
                    let msg = r#"{"type":"toast","message":"Incorrect password"}"#;
                    let _ = sender.send(Message::Text(msg.into())).await;
                    return;
                }
            }
        }

        // Rejoin or add player
        let pid = if let Some(ref id) = stored_id {
            if lobby.session.rejoin(id) {
                // Update name on rejoin in case it changed
                if !name.trim().is_empty() {
                    if let Some(player) = lobby.session.players.iter_mut().find(|p| p.id == id.as_str()) {
                        player.name = name.clone();
                    }
                }
                id.clone()
            } else {
                let new_id = uuid::Uuid::new_v4().to_string();
                if lobby.session.add_player(&new_id, &name).is_err() {
                    drop(lobby);
                    let (mut sender, _) = socket.split();
                    let msg = serde_json::json!({
                        "type": "toast",
                        "message": format!("Lobby is full (max {} players)", MAX_PLAYERS)
                    });
                    let _ = sender.send(Message::Text(msg.to_string().into())).await;
                    return;
                }
                new_id
            }
        } else {
            let new_id = uuid::Uuid::new_v4().to_string();
            if lobby.session.add_player(&new_id, &name).is_err() {
                drop(lobby);
                let (mut sender, _) = socket.split();
                let msg = serde_json::json!({
                    "type": "toast",
                    "message": format!("Lobby is full (max {} players)", MAX_PLAYERS)
                });
                let _ = sender.send(Message::Text(msg.to_string().into())).await;
                return;
            }
            new_id
        };

        // Clear last_empty since someone is now connected
        lobby.last_empty = None;

        pid
    };

    // Subscribe BEFORE broadcasting so this client receives the state
    let rx = {
        let lobby = lobby_arc.lock().await;
        let rx = lobby.tx.subscribe();
        let state_json = serialize_state(&lobby);
        let _ = lobby.tx.send(state_json);
        rx
    };

    let (mut sender, mut receiver) = socket.split();

    // Send welcome message to this client only
    let welcome = serde_json::json!({
        "type": "welcome",
        "player_id": player_id,
        "lobby_id": lobby_id,
    });
    if sender
        .send(Message::Text(welcome.to_string().into()))
        .await
        .is_err()
    {
        return;
    }

    let mut rx = rx;

    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let response = handle_message(&text, &player_id, &lobby_arc, &state).await;
                        if let Some(err_msg) = response {
                            let _ = sender.send(Message::Text(err_msg.into())).await;
                        }
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
    {
        let mut lobby = lobby_arc.lock().await;
        lobby.session.remove_player(&player_id);
        info!("Player {} disconnected from lobby {}", player_id, lobby_id);

        // Set last_empty if no connected players remain
        if !lobby.has_connected_players() {
            lobby.last_empty = Some(Instant::now());
        }

        let state_json = serialize_state(&lobby);
        let _ = lobby.tx.send(state_json);
    }

    // After 20s, fully remove the player if they haven't reconnected
    let timeout_lobby = lobby_arc.clone();
    let timeout_id = player_id.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
        let mut lobby = timeout_lobby.lock().await;
        let still_disconnected = lobby
            .session
            .players
            .iter()
            .any(|p| p.id == timeout_id && !p.connected);
        if still_disconnected {
            info!("Player {} timed out, removing from lobby", timeout_id);
            lobby.session.players.retain(|p| p.id != timeout_id);
            // Clean up their data
            lobby.session.votes.remove(&timeout_id);
            lobby.session.votes_submitted.retain(|id| id != &timeout_id);
            lobby.session.games.retain(|g| g.suggested_by != timeout_id);
            for game in lobby.session.games.iter_mut() {
                game.vetoed_by.retain(|id| id != &timeout_id);
            }
            // Reassign host if needed
            if lobby.session.host_id.as_deref() == Some(&timeout_id) {
                lobby.session.host_id = lobby
                    .session
                    .players
                    .iter()
                    .find(|p| p.connected)
                    .map(|p| p.id.clone());
            }
            let state_json = serialize_state(&lobby);
            let _ = lobby.tx.send(state_json);
        }
    });
}

/// Returns Some(error_json) if an error message should be sent back to the client only,
/// or None if the message was handled (broadcast already done inside).
async fn handle_message(
    text: &str,
    player_id: &str,
    lobby: &Arc<Mutex<Lobby>>,
    state: &AppState,
) -> Option<String> {
    let value: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => {
            return Some(r#"{"type":"error","message":"invalid JSON"}"#.to_string());
        }
    };

    let msg_type = match value.get("type").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => {
            return Some(r#"{"type":"error","message":"missing type field"}"#.to_string());
        }
    };

    let mut lobby_guard = lobby.lock().await;
    let is_host = lobby_guard.session.get_host_id() == Some(player_id);

    match msg_type.as_str() {
        "set_name" => {
            let name = value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if let Some(player) = lobby_guard.session.players.iter_mut().find(|p| p.id == player_id) {
                if !name.trim().is_empty() {
                    player.name = name;
                }
            }
        }
        "add_game" => {
            let name = value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            lobby_guard.session.add_game(player_id, &name);
        }
        "remove_game" => {
            let game_id = value
                .get("game_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            lobby_guard.session.remove_game(player_id, &game_id);
        }
        "veto_game" => {
            let game_id = value
                .get("game_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            lobby_guard.session.veto_game(player_id, &game_id);
        }
        "unveto_game" => {
            let game_id = value
                .get("game_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            lobby_guard.session.unveto_game(player_id, &game_id);
        }
        "submit_vote" => {
            let ranking: Vec<String> = value
                .get("ranking")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let all_voted = lobby_guard.session.submit_vote(player_id, ranking);
            if all_voted {
                lobby_guard.session.advance_phase();
            }
        }
        "set_ready" => {
            let ready = value
                .get("ready")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            lobby_guard.session.set_ready(player_id, ready);
        }
        "advance_phase" => {
            if !is_host {
                return Some(
                    r#"{"type":"error","message":"only the host can advance the phase"}"#
                        .to_string(),
                );
            }
            lobby_guard.session.advance_phase();
        }
        "reset_session" => {
            if !is_host {
                return Some(
                    r#"{"type":"error","message":"only the host can reset the session"}"#
                        .to_string(),
                );
            }
            lobby_guard.session.reset();
        }
        "kick_player" => {
            if !is_host {
                return Some(
                    r#"{"type":"error","message":"only the host can kick players"}"#.to_string(),
                );
            }
            let target_id = value
                .get("target_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if target_id == player_id {
                return Some(
                    r#"{"type":"error","message":"you cannot kick yourself"}"#.to_string(),
                );
            }
            lobby_guard.session.kick_player(&target_id);
        }
        "set_max_vetoes" => {
            if !is_host {
                return Some(
                    r#"{"type":"error","message":"only the host can change veto count"}"#
                        .to_string(),
                );
            }
            let count = value
                .get("count")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as u32;
            lobby_guard.session.set_max_vetoes(count);
        }
        "set_lobby_public" => {
            if !is_host {
                return Some(
                    r#"{"type":"error","message":"only the host can change lobby visibility"}"#
                        .to_string(),
                );
            }
            let public = value
                .get("public")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            lobby_guard.public = public;
        }
        "set_lobby_password" => {
            if !is_host {
                return Some(
                    r#"{"type":"error","message":"only the host can change the password"}"#
                        .to_string(),
                );
            }
            let pw = value
                .get("password")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let trimmed: String = pw.chars().take(64).collect();
            lobby_guard.password = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            };
        }
        "set_lobby_locked" => {
            if !is_host {
                return Some(
                    r#"{"type":"error","message":"only the host can lock/unlock the lobby"}"#
                        .to_string(),
                );
            }
            let locked = value
                .get("locked")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            lobby_guard.locked = locked;
        }
        "close_lobby" => {
            if !is_host {
                return Some(
                    r#"{"type":"error","message":"only the host can close the lobby"}"#
                        .to_string(),
                );
            }
            let closed_msg = r#"{"type":"lobby_closed"}"#.to_string();
            let _ = lobby_guard.tx.send(closed_msg);
            let lobby_id = lobby_guard.id.clone();
            drop(lobby_guard);
            let mut manager = state.lobbies.lock().await;
            manager.remove_lobby(&lobby_id);
            return None;
        }
        _ => {
            return Some(r#"{"type":"error","message":"unknown message type"}"#.to_string());
        }
    }

    let state_json = serialize_state(&lobby_guard);
    let _ = lobby_guard.tx.send(state_json);
    None
}
