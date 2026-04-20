use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Lobby,
    Adding,
    Vetoing,
    Voting,
    Results,
}

#[derive(Debug, Clone, Serialize)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub connection_status: ConnectionStatus,
    pub ready: bool,
    // pub connected: bool,
    // pub disconnect_timeout: Option<u32>,
}

impl Player {
    pub fn is_connected(&self) -> bool {
        matches!(self.connection_status, ConnectionStatus::Connected)
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum ConnectionStatus {
    Connected,
    Kicked,
    /// Seconds remaining before this player is removed (None if connected)
    Disconnected(u32),
}

#[derive(Debug)]
pub enum JoinOutcome {
    New,
    Rejoined,
    Kicked,
    LobbyFull,
}

#[derive(Debug, Clone, Serialize)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub suggested_by: String,
    pub vetoed_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoteResult {
    pub game_id: String,
    pub game_name: String,
    pub score: u32,
    pub rank: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub phase: Phase,
    pub players: Vec<Player>,
    pub games: Vec<Game>,
    pub votes_submitted: Vec<String>,
    pub results: Option<Vec<VoteResult>>,
    pub max_vetoes: u32,
    #[serde(skip)]
    pub votes: HashMap<String, Vec<String>>,
    #[serde(skip)]
    pub host_id: Option<String>,
    #[serde(skip)]
    pub kicked_ids: HashSet<String>,
}

pub const MAX_PLAYERS: usize = 8;

impl Session {
    pub fn new() -> Session {
        Session {
            phase: Phase::Lobby,
            players: Vec::new(),
            games: Vec::new(),
            votes_submitted: Vec::new(),
            results: None,
            max_vetoes: 1,
            votes: HashMap::new(),
            host_id: None,
            kicked_ids: HashSet::new(),
        }
    }

    pub fn set_max_vetoes(&mut self, count: u32) {
        self.max_vetoes = count.max(1).min(20);
    }

    /// Try to reconnect a previously-seen player.
    pub fn rejoin(&mut self, id: &str, name: &str) -> Option<JoinOutcome> {
        let Some(player) = self.players.iter_mut().find(|p| p.id == id) else {
            // No existing player found, so must be a new one.
            return None;
        };

        player.name = String::from(name);

        let outcome = match player.connection_status {
            ConnectionStatus::Connected => {
                // Someone is already here with that?
                // It's possible that they just opened it up in a new tab...
                JoinOutcome::Rejoined
            }
            ConnectionStatus::Kicked => {
                // Return early here, as we're just getting rid of them..
                return Some(JoinOutcome::Kicked);
            }
            ConnectionStatus::Disconnected(_) => {
                // Reconnect the player to existing slot.
                player.connection_status = ConnectionStatus::Connected;
                JoinOutcome::Rejoined
            }
        };

        // If there's no host, assign this player as host
        if self.host_id.is_none() {
            self.host_id = Some(id.to_string());
        }

        Some(outcome)
    }

    pub fn add_player(&mut self, id: &str, name: &str) -> JoinOutcome {
        if let Some(outcome) = self.rejoin(id, name) {
            return outcome;
        }

        let outcome = if self.players.len() >= MAX_PLAYERS {
            // Return early here, as it's an error state.
            return JoinOutcome::LobbyFull;
        } else {
            self.players.push(Player {
                id: String::from(id),
                name: String::from(name),
                connection_status: ConnectionStatus::Connected,
                ready: false,
            });

            JoinOutcome::New
        };

        // If there's no host, assign this player as host
        if self.host_id.is_none() {
            self.host_id = Some(id.to_string());
        }

        outcome
    }

    pub fn remove_player(&mut self, id: &str) {
        if let Some(player) = self.players.iter_mut().find(|p| p.id == id) {
            player.connection_status = ConnectionStatus::Disconnected(20);
        }
    }

    pub fn kick_player(&mut self, target_id: &str) {
        self.players.retain(|p| p.id != target_id);
        self.kicked_ids.insert(target_id.to_string());
        // Clean up any votes, vetoes, and games from the kicked player
        self.votes.remove(target_id);
        self.votes_submitted.retain(|id| id != target_id);
        self.games.retain(|g| g.suggested_by != target_id);
        for game in self.games.iter_mut() {
            game.vetoed_by.retain(|id| id != target_id);
        }
    }

    pub fn is_kicked(&self, player_id: &str) -> bool {
        self.kicked_ids.contains(player_id)
    }

    pub fn get_host_id(&self) -> Option<&str> {
        self.host_id.as_deref()
    }

    pub fn has_eligible_games(&self) -> bool {
        self.games.iter().any(|g| g.vetoed_by.is_empty())
    }

    pub fn advance_phase(&mut self) {
        // Don't advance from Vetoing to Voting if all games are vetoed
        if self.phase == Phase::Vetoing && !self.has_eligible_games() {
            return;
        }

        self.phase = match self.phase {
            Phase::Lobby => Phase::Adding,
            Phase::Adding => Phase::Vetoing,
            Phase::Vetoing => Phase::Voting,
            Phase::Voting => {
                self.compute_results();
                Phase::Results
            }
            Phase::Results => Phase::Results,
        };

        if self.phase == Phase::Vetoing || self.phase == Phase::Voting {
            // Reset ready flags when entering Vetoing or Voting
            for player in self.players.iter_mut() {
                player.ready = false;
            }
        }
        if self.phase == Phase::Voting {
            self.votes_submitted.clear();
        }
    }

    fn compute_results(&mut self) {
        let eligible_games: Vec<&Game> = self
            .games
            .iter()
            .filter(|g| g.vetoed_by.is_empty())
            .collect();

        let n = eligible_games.len();
        let eligible_ids: Vec<String> = eligible_games.iter().map(|g| g.id.clone()).collect();

        let mut scores: HashMap<String, u32> =
            eligible_ids.iter().map(|id| (id.clone(), 0u32)).collect();

        for ranking in self.votes.values() {
            // Only count positions for eligible games
            let filtered: Vec<&String> = ranking
                .iter()
                .filter(|gid| eligible_ids.contains(gid))
                .collect();

            for (pos, game_id) in filtered.iter().enumerate() {
                let points = (n - pos) as u32;
                *scores.entry((*game_id).clone()).or_insert(0) += points;
            }
        }

        let mut score_vec: Vec<(String, u32)> = scores.into_iter().collect();
        score_vec.sort_by(|a, b| b.1.cmp(&a.1));

        let mut results: Vec<VoteResult> = Vec::new();
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
                .games
                .iter()
                .find(|g| g.id == game_id)
                .map(|g| g.name.clone())
                .unwrap_or_default();
            results.push(VoteResult {
                game_id,
                game_name,
                score,
                rank: current_rank,
            });
        }

        self.results = Some(results);
    }

    pub fn set_ready(&mut self, player_id: &str, ready: bool) {
        if let Some(player) = self.players.iter_mut().find(|p| p.id == player_id) {
            player.ready = ready;
        }
    }

    pub fn add_game(&mut self, player_id: &str, name: &str) -> bool {
        if self.phase != Phase::Adding {
            return false;
        }
        if name.trim().is_empty() {
            return false;
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.games.push(Game {
            id,
            name: name.to_string(),
            suggested_by: player_id.to_string(),
            vetoed_by: Vec::new(),
        });
        true
    }

    pub fn remove_game(&mut self, player_id: &str, game_id: &str) -> bool {
        if self.phase != Phase::Adding {
            return false;
        }
        if let Some(pos) = self
            .games
            .iter()
            .position(|g| g.id == game_id && g.suggested_by == player_id)
        {
            self.games.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn veto_game(&mut self, player_id: &str, game_id: &str) {
        let used = self
            .games
            .iter()
            .filter(|g| g.vetoed_by.contains(&player_id.to_string()))
            .count() as u32;
        if used >= self.max_vetoes {
            return;
        }
        if let Some(game) = self.games.iter_mut().find(|g| g.id == game_id) {
            if !game.vetoed_by.contains(&player_id.to_string()) {
                game.vetoed_by.push(player_id.to_string());
            }
        }
    }

    pub fn unveto_game(&mut self, player_id: &str, game_id: &str) {
        if let Some(game) = self.games.iter_mut().find(|g| g.id == game_id) {
            game.vetoed_by.retain(|id| id != player_id);
        }
    }

    pub fn submit_vote(&mut self, player_id: &str, ranking: Vec<String>) -> bool {
        if self.phase != Phase::Voting {
            return false;
        }
        self.votes.insert(player_id.to_string(), ranking);
        if !self.votes_submitted.contains(&player_id.to_string()) {
            self.votes_submitted.push(player_id.to_string());
        }
        // Check if all connected players have voted
        let connected_players: Vec<&str> = self
            .players
            .iter()
            .filter(|p| p.is_connected())
            .map(|p| p.id.as_str())
            .collect();
        connected_players
            .iter()
            .all(|id| self.votes_submitted.contains(&id.to_string()))
    }

    pub fn reset(&mut self) {
        self.games.clear();
        self.votes.clear();
        self.votes_submitted.clear();
        self.results = None;
        self.kicked_ids.clear();
        self.phase = Phase::Lobby;
        for player in self.players.iter_mut() {
            player.ready = false;
        }
    }
}
