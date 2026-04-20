use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio::time::Instant;

use crate::types;

#[derive(Debug)]
pub enum JoinOutcome {
    Joined(broadcast::Receiver<String>, bool),
    Locked,
    Kicked,
    LobbyFull,
    IncorrectPassword,
}

pub const MAX_PLAYERS: usize = 8;

#[derive(Clone)]
pub struct Lobby(Arc<Mutex<Inner>>);

// #[derive(Debug, Clone)]
pub struct Inner {
    pub id: types::LobbyId,
    pub name: String,
    pub public: bool,
    pub password: Option<String>,
    pub locked: bool,
    pub tx: broadcast::Sender<String>,
    pub last_empty: Option<Instant>,

    pub phase: types::Phase,
    pub players: Vec<types::Player>,
    pub options: Vec<types::Opt>,
    pub votes_submitted: Vec<String>,
    pub results: Option<Vec<types::VoteResult>>,
    pub max_vetoes: u32,
    pub votes: HashMap<String, Vec<String>>,
    pub host_id: Option<types::PlayerId>,
    pub kicked_ids: HashSet<String>,
}

impl Lobby {
    pub fn new(public: bool, password: Option<String>) -> Lobby {
        let id = types::LobbyId::rand();
        let name = generate_name();

        let (tx, _rx) = broadcast::channel::<String>(64);
        Self(Arc::new(Mutex::new(Inner {
            id,
            name,
            public,
            password,
            locked: false,
            tx,
            last_empty: Some(Instant::now()),
            phase: types::Phase::Lobby,
            players: Vec::new(),
            options: Vec::new(),
            votes_submitted: Vec::new(),
            results: None,
            max_vetoes: 1,
            votes: HashMap::new(),
            host_id: None,
            kicked_ids: HashSet::new(),
        })))
    }

    pub fn join(&mut self, info: &types::JoinInfo) -> JoinOutcome {
        self.0.lock().unwrap().join(info)
    }
}

impl Inner {
    pub fn join(&mut self, info: &types::JoinInfo) -> JoinOutcome {
        let types::JoinInfo {
            player_id,
            name,
            lobby_id: _,
            password,
        } = info;

        if let Some(outcome) = self.rejoin(player_id, name) {
            return outcome;
        }

        if self.locked {
            return JoinOutcome::Locked;
        }

        if self.players.len() >= MAX_PLAYERS {
            return JoinOutcome::LobbyFull;
        }

        // match (self.password, password) {
        //     (None, None) => (),
        //     (None, Some(_)) => (),
        //     (Some(pw), None) => {
        //         return JoinOutcome::IncorrectPassword
        //     }
        //     (Some(pw), Some(user_pw)) => {
        //         if &pw != user_pw {
        //             return JoinOutcome::IncorrectPassword;
        //         }
        //     }
        // }
        //     // Only verify password on newcomers.
        //     if matches!(outcome, JoinOutcome::New) {
        //         if let Some(ref lobby_pw) = lobby.password {
        //             let provided = password.as_deref().unwrap_or("");
        //             if provided != lobby_pw {
        //                 warn!(lobby_id, "player rejected: incorrect password");
        //                 drop(lobby);
        //                 let (mut sender, _) = socket.split();
        //                 let msg = r#"{"type":"toast","message":"Incorrect password"}"#;
        //                 let _ = sender.send(Message::Text(msg.into())).await;
        //                 return;
        //             }
        //         }
        //     }

        // if let Some(ref pw) = self.password {
        //     if matches!(password, Some(user_pw) if pw == user_pw) {
        //
        //     } else {
        //
        //     }
        //     // if &pw != user_pw {
        //     //     return JoinOutcome::IncorrectPassword;
        //     // }
        //
        //
        // }

        self.players.push(types::Player {
            id: player_id.clone(),
            name: String::from(name),
            connection_status: types::ConnectionStatus::Connected,
            ready: false,
        });

        let rx = self.tx.subscribe();

        let outcome = JoinOutcome::Joined(rx, false);

        // If there's no host, assign this player as host
        if self.host_id.is_none() {
            self.host_id = Some(player_id.clone());
        }

        outcome
    }

    /// Try to reconnect a previously-seen player.
    pub fn rejoin(&mut self, id: &types::PlayerId, name: &str) -> Option<JoinOutcome> {
        let Some(player) = self.players.iter_mut().find(|p| &p.id == id) else {
            // No existing player found, so must be a new one.
            return None;
        };

        player.name = String::from(name);

        let outcome = match player.connection_status {
            types::ConnectionStatus::Connected => {
                let rx = self.tx.subscribe();
                // Someone is already here?
                // It's possible that they just opened it up in a new tab...
                JoinOutcome::Joined(rx, true)
            }
            types::ConnectionStatus::Kicked => {
                // Return early here, as we're just getting rid of them..
                return Some(JoinOutcome::Kicked);
            }
            types::ConnectionStatus::Disconnected(_) => {
                let rx = self.tx.subscribe();
                // Reconnect the player to existing slot.
                player.connection_status = types::ConnectionStatus::Connected;
                JoinOutcome::Joined(rx, true)
            }
        };

        // If there's no host, assign this player as host
        if self.host_id.is_none() {
            self.host_id = Some(id.clone());
        }

        Some(outcome)
    }
}

impl Inner {
    // pub fn has_connected_players(&self) -> bool {
    //     self.players.iter().any(|p| p.is_connected())
    // }

    // pub fn set_max_vetoes(&mut self, count: u32) {
    //     self.max_vetoes = count.max(1).min(20);
    // }

    // pub fn remove_player(&mut self, id: &str) {
    //     if let Some(player) = self.players.iter_mut().find(|p| p.id == id) {
    //         player.connection_status = types::ConnectionStatus::Disconnected(20);
    //     }
    // }

    // pub fn kick_player(&mut self, target_id: &str) {
    //     self.players.retain(|p| p.id != target_id);
    //     self.kicked_ids.insert(target_id.to_string());
    //     // Clean up any votes, vetoes, and games from the kicked player
    //     self.votes.remove(target_id);
    //     self.votes_submitted.retain(|id| id != target_id);
    //     self.options.retain(|g| g.suggested_by != target_id);
    //     for game in self.options.iter_mut() {
    //         game.vetoed_by.retain(|id| id != target_id);
    //     }
    // }

    // pub fn is_kicked(&self, player_id: &str) -> bool {
    //     self.kicked_ids.contains(player_id)
    // }

    // pub fn get_host_id(&self) -> Option<&str> {
    //     self.host_id.as_deref()
    // }

    // pub fn has_eligible_games(&self) -> bool {
    //     self.options.iter().any(|g| g.vetoed_by.is_empty())
    // }

    // pub fn advance_phase(&mut self) {
    //     use types::Phase;
    //     // Don't advance from Vetoing to Voting if all games are vetoed
    //     if self.phase == Phase::Vetoing && !self.has_eligible_games() {
    //         return;
    //     }
    //
    //     self.phase = match self.phase {
    //         Phase::Lobby => Phase::Adding,
    //         Phase::Adding => Phase::Vetoing,
    //         Phase::Vetoing => Phase::Voting,
    //         Phase::Voting => {
    //             self.compute_results();
    //             Phase::Results
    //         }
    //         Phase::Results => Phase::Results,
    //     };
    //
    //     if self.phase == Phase::Vetoing || self.phase == Phase::Voting {
    //         // Reset ready flags when entering Vetoing or Voting
    //         for player in self.players.iter_mut() {
    //             player.ready = false;
    //         }
    //     }
    //     if self.phase == Phase::Voting {
    //         self.votes_submitted.clear();
    //     }
    // }

    // fn compute_results(&mut self) {
    //     let eligible_games: Vec<&types::Opt> = self
    //         .options
    //         .iter()
    //         .filter(|g| g.vetoed_by.is_empty())
    //         .collect();
    //
    //     let n = eligible_games.len();
    //     let eligible_ids: Vec<String> = eligible_games.iter().map(|g| g.id.clone()).collect();
    //
    //     let mut scores: HashMap<String, u32> =
    //         eligible_ids.iter().map(|id| (id.clone(), 0u32)).collect();
    //
    //     for ranking in self.votes.values() {
    //         // Only count positions for eligible games
    //         let filtered: Vec<&String> = ranking
    //             .iter()
    //             .filter(|gid| eligible_ids.contains(gid))
    //             .collect();
    //
    //         for (pos, game_id) in filtered.iter().enumerate() {
    //             let points = (n - pos) as u32;
    //             *scores.entry((*game_id).clone()).or_insert(0) += points;
    //         }
    //     }
    //
    //     let mut score_vec: Vec<(String, u32)> = scores.into_iter().collect();
    //     score_vec.sort_by(|a, b| b.1.cmp(&a.1));
    //
    //     let mut results: Vec<types::VoteResult> = Vec::new();
    //     let mut current_rank = 1;
    //     for (i, (game_id, score)) in score_vec.into_iter().enumerate() {
    //         // Same score as previous entry gets the same rank
    //         if i > 0 {
    //             if let Some(prev) = results.last() {
    //                 if score < prev.score {
    //                     current_rank = i + 1;
    //                 }
    //             }
    //         }
    //         let game_name = self
    //             .options
    //             .iter()
    //             .find(|g| g.id == game_id)
    //             .map(|g| g.name.clone())
    //             .unwrap_or_default();
    //         results.push(types::VoteResult {
    //             game_id,
    //             game_name,
    //             score,
    //             rank: current_rank,
    //         });
    //     }
    //
    //     self.results = Some(results);
    // }

    // pub fn set_ready(&mut self, player_id: &str, ready: bool) {
    //     if let Some(player) = self.players.iter_mut().find(|p| p.id == player_id) {
    //         player.ready = ready;
    //     }
    // }

    pub fn add_game(&mut self, player_id: &str, name: &str) -> bool {
        if self.phase != types::Phase::Adding {
            return false;
        }
        if name.trim().is_empty() {
            return false;
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.options.push(types::Opt {
            id,
            name: name.to_string(),
            suggested_by: player_id.to_string(),
            vetoed_by: None,
        });
        true
    }

    pub fn remove_game(&mut self, player_id: &str, game_id: &str) -> bool {
        if self.phase != types::Phase::Adding {
            return false;
        }
        if let Some(pos) = self
            .options
            .iter()
            .position(|g| g.id == game_id && g.suggested_by == player_id)
        {
            self.options.remove(pos);
            true
        } else {
            false
        }
    }

    // pub fn veto_game(&mut self, player_id: &str, game_id: &str) {
    //     let used = self
    //         .options
    //         .iter()
    //         .filter(|g| g.vetoed_by.is_some_and(|id| id == player_id))
    //         .count() as u32;
    //     if used >= self.max_vetoes {
    //         return;
    //     }
    //     if let Some(game) = self.options.iter_mut().find(|g| g.id == game_id) {
    //         if !game.vetoed_by.contains(&player_id.to_string()) {
    //             game.vetoed_by.push(player_id.to_string());
    //         }
    //     }
    // }
    //
    // pub fn unveto_game(&mut self, player_id: &str, game_id: &str) {
    //     if let Some(game) = self.options.iter_mut().find(|g| g.id == game_id) {
    //         game.vetoed_by.retain(|id| id != player_id);
    //     }
    // }

    // pub fn submit_vote(&mut self, player_id: &str, ranking: Vec<String>) -> bool {
    //     if self.phase != Phase::Voting {
    //         return false;
    //     }
    //     self.votes.insert(player_id.to_string(), ranking);
    //     if !self.votes_submitted.contains(&player_id.to_string()) {
    //         self.votes_submitted.push(player_id.to_string());
    //     }
    //     // Check if all connected players have voted
    //     let connected_players: Vec<&str> = self
    //         .players
    //         .iter()
    //         .filter(|p| p.is_connected())
    //         .map(|p| &p.id)
    //         .collect();
    //     connected_players
    //         .iter()
    //         .all(|id| self.votes_submitted.contains(&id.to_string()))
    // }

    pub fn reset(&mut self) {
        self.options.clear();
        self.votes.clear();
        self.votes_submitted.clear();
        self.results = None;
        self.kicked_ids.clear();
        self.phase = types::Phase::Lobby;
        for player in self.players.iter_mut() {
            player.ready = false;
        }
    }
}

fn generate_name() -> String {
    use rand::RngExt;
    const ADJECTIVES: &[&str] = &[
        "Fuzzy", "Spicy", "Grumpy", "Wobbly", "Sneaky", "Bouncy", "Fluffy", "Zesty", "Clumsy",
        "Sassy", "Wiggly", "Cheeky", "Dizzy", "Jolly", "Wacky", "Peppy", "Goofy", "Nifty", "Perky",
        "Zippy",
    ];
    const NOUNS: &[&str] = &[
        "Badger",
        "Narwhal",
        "Capybara",
        "Penguin",
        "Platypus",
        "Axolotl",
        "Quokka",
        "Wombat",
        "Meerkat",
        "Pangolin",
        "Tapir",
        "Manatee",
        "Binturong",
        "Fossa",
        "Numbat",
        "Echidna",
        "Kinkajou",
        "Potoo",
        "Blobfish",
        "Tardigrade",
    ];
    let mut rng = rand::rng();
    let adj = ADJECTIVES[rng.random_range(0..ADJECTIVES.len())];
    let noun = NOUNS[rng.random_range(0..NOUNS.len())];
    format!("{} {}", adj, noun)
}
