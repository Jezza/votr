use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio::time::Instant;

use crate::types;

#[derive(Debug)]
pub enum JoinOutcome {
    Joined(broadcast::Receiver<types::Outgoing>, bool),
    Locked,
    Kicked,
    LobbyFull,
    IncorrectPassword,
}

pub const MAX_PLAYERS: usize = 8;

// #[derive(Clone)]
// pub struct Lobby(Arc<Mutex<Inner>>);

// #[derive(Debug, Clone)]
pub struct Lobby {
    pub id: types::LobbyId,
    pub tx: broadcast::Sender<types::Outgoing>,

    pub name: String,
    pub public: bool,
    pub password: Option<String>,
    pub locked: bool,

    pub last_empty: Option<Instant>,
    pub max_vetoes: u32,

    pub players: Vec<types::Player>,

    pub phase: types::Phase,
    pub options: Vec<types::Opt>,
    pub votes: HashMap<types::PlayerId, Vec<types::OptId>>,
    pub results: Option<Vec<types::VoteResult>>,
    pub kicked_ids: HashSet<types::PlayerId>,

    pub host_id: Option<types::PlayerId>,
}

impl Lobby {
    pub fn new(public: bool, password: Option<String>) -> Lobby {
        let id = types::LobbyId::rand();
        let name = generate_name();

        let (tx, _rx) = broadcast::channel::<types::Outgoing>(64);
        Self {
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
            results: None,
            max_vetoes: 1,
            votes: HashMap::new(),
            host_id: None,
            kicked_ids: HashSet::new(),
        }
    }

    fn eligible_options(&self) -> impl Iterator<Item = &types::Opt> {
        self.options.iter().filter(|g| g.vetoed_by.is_none())
    }

    fn compute_results(&mut self) {
        let eligible_games: Vec<&types::Opt> = self.eligible_options().collect();

        let n = eligible_games.len();
        let eligible_ids: Vec<types::OptId> = eligible_games.iter().map(|g| g.id.clone()).collect();

        let mut scores: HashMap<types::OptId, u32> =
            eligible_ids.iter().map(|id| (id.clone(), 0u32)).collect();

        for ranking in self.votes.values() {
            let filtered: Vec<&types::OptId> = ranking
                .iter()
                .filter(|gid| eligible_ids.contains(gid))
                .collect();

            for (pos, id) in filtered.iter().enumerate() {
                let points = (n - pos) as u32;
                *scores.entry((*id).clone()).or_insert(0) += points;
            }
        }

        let mut score_vec: Vec<(types::OptId, u32)> = scores.into_iter().collect();
        score_vec.sort_by(|a, b| b.1.cmp(&a.1));

        let mut results: Vec<types::VoteResult> = Vec::new();
        let mut current_rank = 1;
        for (i, (game_id, score)) in score_vec.into_iter().enumerate() {
            // Same score as previous entry gets the same rank
            if i > 0 {
                if let Some(prev) = results.last() {
                    if score < prev.score {
                        current_rank = i + 1;
                    }
                }
            }
            let game_name = self
                .options
                .iter()
                .find(|g| g.id == game_id)
                .map(|g| g.name.clone())
                .unwrap_or_default();
            results.push(types::VoteResult {
                game_id,
                game_name,
                score,
                rank: current_rank,
            });
        }

        self.results = Some(results);
    }

    pub fn is_host(&self, player_id: &types::PlayerId) -> bool {
        self.host_id
            .as_ref()
            .map_or(false, |host| host == player_id)
    }
}

impl Lobby {
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

    pub fn set_name(&mut self, player_id: &types::PlayerId, mut name: String) -> bool {
        trim_in_place(&mut name);
        if name.is_empty() {
            return false;
        }

        let found = self.players.iter_mut().find(|p| &p.id == player_id);
        if let Some(player) = found {
            player.name = name;
            true
        } else {
            false
        }
    }

    pub fn add_game(&mut self, player_id: &types::PlayerId, mut name: String) -> bool {
        if self.phase != types::Phase::Adding {
            return false;
        }
        trim_in_place(&mut name);
        if name.is_empty() {
            return false;
        }

        self.options.push(types::Opt {
            id: types::OptId::rand(),
            name: name.to_string(),
            suggested_by: player_id.clone(),
            vetoed_by: None,
        });

        true
    }

    pub fn remove_game(&mut self, player_id: &types::PlayerId, choice_id: &types::OptId) -> bool {
        if self.phase != types::Phase::Adding {
            return false;
        }
        let opt = self
            .options
            .iter()
            .position(|opt| &opt.id == choice_id && &opt.suggested_by == player_id);

        if let Some(pos) = opt {
            self.options.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn veto_game(&mut self, player_id: &types::PlayerId, game_id: &types::OptId) -> bool {
        if self.phase != types::Phase::Vetoing {
            return false;
        }

        let used = self
            .options
            .iter()
            .filter(|g| g.vetoed_by.as_ref().is_some_and(|id| id == player_id))
            .count() as u32;

        if used >= self.max_vetoes {
            return false;
        }
        if let Some(game) = self.options.iter_mut().find(|g| &g.id == game_id) {
            game.vetoed_by.get_or_insert_with(|| player_id.clone());
            true
        } else {
            false
        }
    }

    pub fn unveto_game(&mut self, player_id: &types::PlayerId, game_id: &types::OptId) -> bool {
        if self.phase != types::Phase::Vetoing {
            return false;
        }

        self.options
            .iter_mut()
            .find(|g| &g.id == game_id)
            .map_or(false, |opt| {
                opt.vetoed_by.take_if(|opt| opt == player_id).is_some()
            })
    }

    pub fn submit_vote(&mut self, player_id: &types::PlayerId, ranking: Vec<types::OptId>) -> bool {
        if self.phase != types::Phase::Voting {
            return false;
        }
        self.votes.insert(player_id.clone(), ranking);

        self.players
            .iter()
            // .filter(|p| p.is_connected())
            .all(|player| self.votes.contains_key(&player.id))
    }

    pub fn set_ready(&mut self, player_id: &types::PlayerId, ready: bool) -> bool {
        let player = self.players.iter_mut().find(|p| &p.id == player_id);

        if let Some(player) = player {
            player.ready = ready;
            true
        } else {
            false
        }
    }

    pub fn advance_phase(&mut self) {
        use types::Phase;

        self.phase = match self.phase {
            Phase::Lobby => Phase::Adding,
            Phase::Adding => {
                if self.eligible_options().next().is_none() {
                    return;
                }
                Phase::Vetoing
            }
            Phase::Vetoing => Phase::Voting,
            Phase::Voting => {
                self.compute_results();

                Phase::Results
            }
            Phase::Results => Phase::Results,
        };

        self.unready_players()
    }

    fn unready_players(&mut self) {
        for player in self.players.iter_mut() {
            player.ready = false;
        }
    }

    pub fn reset(&mut self) {
        self.options.clear();
        self.votes.clear();
        self.results = None;
        self.kicked_ids.clear();
        self.phase = types::Phase::Lobby;

        self.unready_players();
    }

    pub fn set_max_vetoes(&mut self, count: u32) {
        self.max_vetoes = count.max(1).min(32);
    }

    pub fn kick_player(&mut self, target_id: &types::PlayerId) {
        self.players.retain(|p| &p.id != target_id);
        self.kicked_ids.insert(target_id.clone());
        self.votes.remove(target_id);
        self.options.retain(|g| &g.suggested_by != target_id);
        for game in self.options.iter_mut() {
            game.vetoed_by.take_if(|id| id == target_id);
        }
    }

    pub fn set_lobby_public(&mut self, player_id: &types::PlayerId, public: bool) {
        if self.is_host(player_id) {
            self.public = public;
        }
    }

    pub fn set_lobby_password(&mut self, player_id: &types::PlayerId, password: Option<String>) {
        if self.is_host(player_id) {
            self.password = password;
        }
    }

    pub fn set_lobby_locked(&mut self, player_id: &types::PlayerId, locked: bool) {
        if self.is_host(player_id) {
            self.locked = locked;
        }
    }

    pub fn close(&mut self, player_id: &types::PlayerId) -> bool {
        if self.is_host(player_id) {
            self.tx
                .send(types::Outgoing::LobbyClosed(types::LobbyClosed {}))
                .is_ok()
        } else {
            false
        }
    }
}

// impl Inner {
//     // pub fn has_connected_players(&self) -> bool {
//     //     self.players.iter().any(|p| p.is_connected())
//     // }
//     // pub fn is_kicked(&self, player_id: &str) -> bool {
//     //     self.kicked_ids.contains(player_id)
// }

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

fn trim_in_place(s: &mut String) {
    let trimmed = s.trim();
    let start = trimmed.as_ptr() as usize - s.as_ptr() as usize;
    let len = trimmed.len();
    s.truncate(start + len);
    s.drain(..start);
}
