use crate::{lobby, types};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio::time::Instant;
use tracing::{info, warn};
use vector_map::VecMap;

#[derive(Debug)]
pub enum JoinOutcome {
    Joined(broadcast::Receiver<types::Outgoing>, bool),
    Locked,
    Kicked,
    LobbyFull,
    IncorrectPassword,
}

pub const MAX_PLAYERS: usize = 8;

pub struct Lobby {
    id: types::LobbyId,
    tx: broadcast::Sender<types::Outgoing>,

    name: String,
    public: bool,
    password: Option<String>,
    locked: bool,

    last_empty: Option<Instant>,
    max_vetoes: u32,

    players: VecMap<types::PlayerId, types::Player>,

    phase: types::Phase,
    options: Vec<types::Opt>,
    votes: HashMap<types::PlayerId, Vec<types::OptId>>,
    results: Option<Vec<types::VoteResult>>,
    kicked_ids: HashSet<types::PlayerId>,

    host_id: Option<types::PlayerId>,
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
            locked: Default::default(),
            tx,
            last_empty: Some(Instant::now()),
            phase: types::Phase::Lobby,
            players: Default::default(),
            options: Default::default(),
            results: Default::default(),
            max_vetoes: 1,
            votes: Default::default(),
            host_id: Default::default(),
            kicked_ids: Default::default(),
        }
    }

    pub fn id(&self) -> types::LobbyId {
        self.id
    }

    pub fn players(&self) -> &VecMap<types::PlayerId, types::Player> {
        &self.players
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn public(&self) -> bool {
        self.public
    }

    pub fn as_info(&self) -> types::LobbyInfo {
        let count = self.connected_players().count();

        let Self {
            id,
            tx: _,
            name,
            public: _,
            password,
            locked,
            last_empty: _,
            max_vetoes: _,
            players: _,
            phase,
            options: _,
            votes: _,
            results: _,
            kicked_ids: _,
            host_id: _,
        } = self;

        types::LobbyInfo {
            id: *id,
            name: String::from(name),
            player_count: count,
            max_players: MAX_PLAYERS,
            has_password: password.is_some(),
            locked: *locked,
            phase: *phase,
        }
    }

    fn eligible_options(&self) -> impl Iterator<Item = &types::Opt> {
        self.options.iter().filter(|g| g.vetoed_by.is_none())
    }

    fn connected_players(&self) -> impl Iterator<Item = (&types::PlayerId, &types::Player)> {
        self.players.iter().filter(|(_id, p)| p.is_connected())
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

    pub fn is_host(&self, player_id: types::PlayerId) -> bool {
        self.host_id
            .as_ref()
            .map_or(false, |host| *host == player_id)
    }

    pub fn reassign_host_from(&mut self, player_id: types::PlayerId) {
        if self.is_host(player_id) {
            // Pick someone else, if there is anyone else.
            let new_host = self
                .connected_players()
                .skip_while(|(id, p)| **id == player_id)
                .map(|(id, _)| *id)
                .next();
            self.host_id = new_host;
        }
    }
}

impl Lobby {
    pub fn send_state(&self) {
        let Self {
            tx: _,
            last_empty: _,
            votes: _,
            kicked_ids: _,
            id,
            name,
            public,
            password,
            locked,
            max_vetoes,
            players,
            phase,
            options,
            results,
            host_id,
        } = self;

        let players = players.values().cloned().collect::<Vec<_>>();

        let state = types::LobbyState {
            phase: *phase,
            players,
            games: options.clone(),
            results: results.clone(),
            host_id: *host_id,
            max_vetoes: *max_vetoes,
            lobby_id: *id,
            lobby_name: name.clone(),
            lobby_public: *public,
            lobby_locked: *locked,
            lobby_has_password: password.is_some(),
        };

        let _ = self.tx.send(types::Outgoing::LobbyState(state)).ok();
    }

    pub fn join(&mut self, info: &types::JoinInfo) -> JoinOutcome {
        let types::JoinInfo {
            player_id,
            name,
            lobby_id: _,
            password,
        } = info;

        let player_id = *player_id;

        if let Some(outcome) = self.rejoin(player_id, name) {
            return outcome;
        }

        if self.locked {
            return JoinOutcome::Locked;
        }

        if self.players.len() >= MAX_PLAYERS {
            return JoinOutcome::LobbyFull;
        }

        match (self.password.as_deref(), password.as_deref()) {
            (None, None) => (),
            (None, Some(_)) | (Some(_), None) => {
                warn!(%player_id, "player rejected: incorrect password");
                return JoinOutcome::IncorrectPassword;
            }
            (Some(pw), Some(user_pw)) => {
                if pw != user_pw {
                    warn!(%player_id, "player rejected: incorrect password");
                    return JoinOutcome::IncorrectPassword;
                }
            }
        }

        self.players.insert(
            player_id,
            types::Player {
                id: player_id,
                name: String::from(name),
                connection_status: types::ConnectionStatus::connected(),
                ready: false,
            },
        );

        let rx = self.tx.subscribe();

        let outcome = JoinOutcome::Joined(rx, false);

        // If there's no host, assign this player as host
        if self.host_id.is_none() {
            self.host_id = Some(player_id);
        }

        self.send_state();

        outcome
    }

    /// Try to reconnect a previously-seen player.
    pub fn rejoin(&mut self, id: types::PlayerId, name: &str) -> Option<JoinOutcome> {
        if self.kicked_ids.contains(&id) {
            // Return early here, as we're just getting rid of them..
            return Some(JoinOutcome::Kicked);
        }

        // If we can't find it, then it's not an existing player...
        let player = self.players.get_mut(&id)?;

        player.name = String::from(name);
        player.connection_status = types::ConnectionStatus::connected();

        let rx = self.tx.subscribe();

        // If there's no host, assign this player as host
        if self.host_id.is_none() {
            self.host_id = Some(id.clone());
        }

        self.send_state();

        Some(JoinOutcome::Joined(rx, true))
    }

    pub fn set_name(&mut self, player_id: types::PlayerId, mut name: String) -> bool {
        crate::trim_in_place(&mut name);
        if name.is_empty() {
            return false;
        }

        if let Some(player) = self.players.get_mut(&player_id) {
            player.name = name;
            true
        } else {
            false
        }
    }

    pub fn add_game(&mut self, player_id: types::PlayerId, mut name: String) -> bool {
        if self.phase != types::Phase::Adding {
            return false;
        }
        crate::trim_in_place(&mut name);
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

    pub fn remove_game(&mut self, player_id: types::PlayerId, choice_id: types::OptId) -> bool {
        if self.phase != types::Phase::Adding {
            return false;
        }
        let opt = self
            .options
            .iter()
            .position(|opt| opt.id == choice_id && opt.suggested_by == player_id);

        if let Some(pos) = opt {
            self.options.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn veto_game(&mut self, player_id: types::PlayerId, game_id: &types::OptId) -> bool {
        if self.phase != types::Phase::Vetoing {
            return false;
        }

        let used = self
            .options
            .iter()
            .filter(|g| g.vetoed_by.as_ref().is_some_and(|id| *id == player_id))
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

    pub fn unveto_game(&mut self, player_id: types::PlayerId, game_id: &types::OptId) -> bool {
        if self.phase != types::Phase::Vetoing {
            return false;
        }

        self.options
            .iter_mut()
            .find(|g| &g.id == game_id)
            .map_or(false, |opt| {
                opt.vetoed_by.take_if(|opt| *opt == player_id).is_some()
            })
    }

    pub fn submit_vote(&mut self, player_id: types::PlayerId, ranking: Vec<types::OptId>) -> bool {
        if self.phase != types::Phase::Voting {
            return false;
        }
        self.votes.insert(player_id.clone(), ranking);

        self.set_ready(player_id, true);

        self.connected_players()
            .all(|(id, _p)| self.votes.contains_key(&id))
    }

    pub fn set_ready(&mut self, player_id: types::PlayerId, ready: bool) -> bool {
        let Some(player) = self.players.get_mut(&player_id) else {
            return false;
        };

        player.ready = ready;

        true
    }

    pub fn check_advance(&mut self) {
        if self.phase.auto_advance() && self.connected_players().all(|(_id, p)| p.ready) {
            self.advance_phase_impl();
        }
    }

    pub fn advance_phase(&mut self, player_id: types::PlayerId) {
        if !self.is_host(player_id) {
            return;
        }
        self.advance_phase_impl();
    }

    fn advance_phase_impl(&mut self) {
        use types::Phase;

        self.phase = match self.phase {
            Phase::Lobby => Phase::Adding,
            Phase::Adding => {
                if self.eligible_options().next().is_none() {
                    return;
                }
                if self.max_vetoes == 0 {
                    Phase::Voting
                } else {
                    Phase::Vetoing
                }
            }
            Phase::Vetoing => {
                if self.eligible_options().next().is_none() {
                    return;
                }
                Phase::Voting
            }
            Phase::Voting => {
                self.compute_results();

                Phase::Results
            }
            Phase::Results => Phase::Results,
        };

        self.unready_players()
    }

    fn unready_players(&mut self) {
        for (_, player) in self.players.iter_mut() {
            player.ready = false;
        }
    }

    pub fn reset(&mut self, player_id: types::PlayerId) {
        if !self.is_host(player_id) {
            return;
        }

        self.options.clear();
        self.votes.clear();
        self.results = None;
        self.kicked_ids.clear();
        self.phase = types::Phase::Lobby;

        self.unready_players();
    }

    pub fn set_max_vetoes(&mut self, player_id: types::PlayerId, count: u32) {
        if self.is_host(player_id) {
            self.max_vetoes = count.min(32);
        }
    }

    pub fn kick_player(&mut self, player_id: types::PlayerId, target_id: types::PlayerId) {
        if !self.is_host(player_id) {
            return;
        }

        self.remove_player(target_id);

        self.kicked_ids.insert(target_id.clone());
    }

    pub fn set_lobby_public(&mut self, player_id: types::PlayerId, public: bool) {
        if self.is_host(player_id) {
            self.public = public;
        }
    }

    pub fn set_lobby_password(&mut self, player_id: types::PlayerId, password: Option<String>) {
        if self.is_host(player_id) {
            self.password = password;
        }
    }

    pub fn set_lobby_locked(&mut self, player_id: types::PlayerId, locked: bool) {
        if self.is_host(player_id) {
            self.locked = locked;
        }
    }

    pub fn close(&mut self, player_id: types::PlayerId) -> bool {
        if self.is_host(player_id) {
            let _ = self
                .tx
                .send(types::Outgoing::LobbyClosed(types::LobbyClosed {}))
                .ok();

            true
        } else {
            false
        }
    }

    /// Typically from the server shutting down, but this _purges_ the player.
    pub fn remove_player(&mut self, player_id: types::PlayerId) {
        self.players.remove(&player_id);
        self.votes.remove(&player_id);
        self.options.retain(|g| g.suggested_by != player_id);
        for opt in self.options.iter_mut() {
            opt.vetoed_by.take_if(|id| *id == player_id);
        }

        let has_players = self.players.iter().any(|(_id, p)| p.is_connected());
        if !has_players {
            self.last_empty = Some(Instant::now());
        }

        self.reassign_host_from(player_id);

        self.send_state();
    }

    /// This removes the active nature of a player.
    pub fn disconnect_player(&mut self, player_id: types::PlayerId) {
        if let Some(player) = self.players.get_mut(&player_id) {
            player.connection_status = types::ConnectionStatus::disconnected();
        }

        let has_players = self.players.values().any(|p| p.is_connected());
        if !has_players {
            self.last_empty = Some(Instant::now());
        }

        self.send_state();
    }

    pub fn timeout_player(&mut self, player_id: types::PlayerId) {
        let connected = self
            .players
            .get(&player_id)
            .map(|p| p.is_connected())
            .unwrap_or_default();

        if !connected {
            info!("Player {} timed out, removing host role", player_id);

            self.reassign_host_from(player_id);

            self.send_state();
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
