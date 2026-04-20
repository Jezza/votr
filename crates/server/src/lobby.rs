use dashmap::DashMap;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tokio::time::Instant;

use crate::session::Session;

pub const MAX_LOBBIES: usize = 128;
pub const LOBBY_EMPTY_TIMEOUT: u64 = 60;
pub const LOBBY_CLEANUP_INTERVAL: u64 = 10;

pub struct Lobby {
    pub id: String,
    pub name: String,
    pub public: bool,
    pub password: Option<String>,
    pub locked: bool,
    pub session: Session,
    pub tx: broadcast::Sender<String>,
    pub last_empty: Option<Instant>,
}

impl Lobby {
    pub fn new(id: String, name: String, public: bool, password: Option<String>) -> Self {
        let (tx, _rx) = broadcast::channel::<String>(64);
        Lobby {
            id,
            name,
            public,
            password,
            locked: false,
            session: Session::new(),
            tx,
            last_empty: Some(Instant::now()),
        }
    }

    pub fn has_connected_players(&self) -> bool {
        self.session.players.iter().any(|p| p.is_connected())
    }
}

#[derive(Serialize)]
pub struct LobbyInfo {
    pub id: String,
    pub name: String,
    pub player_count: usize,
    pub max_players: usize,
    pub has_password: bool,
    pub locked: bool,
    pub phase: String,
}

type LobbyRef<'a> = dashmap::mapref::one::Ref<'a, String, Lobby>;
type LobbyRefMut<'a> = dashmap::mapref::one::RefMut<'a, String, Lobby>;

#[derive(Clone, Default)]
pub struct LoggyManager(Arc<Inner>);

#[derive(Default)]
struct Inner {
    lobbies: DashMap<String, Lobby>,
}

impl LoggyManager {
    pub fn find_lobby(&self, id: &str) -> Option<LobbyRef<'_>> {
        self.0.lobbies.get(id)
    }

    pub fn find_lobby_mut(&self, id: &str) -> Option<LobbyRefMut<'_>> {
        self.0.lobbies.get_mut(id)
    }
}

pub struct LobbyManager {
    pub lobbies: HashMap<String, Arc<Mutex<Lobby>>>,
}

impl LobbyManager {
    pub fn new() -> Self {
        LobbyManager {
            lobbies: HashMap::new(),
        }
    }

    pub fn create_lobby(
        &mut self,
        public: bool,
        password: Option<String>,
    ) -> Result<(String, String), &'static str> {
        if self.lobbies.len() >= MAX_LOBBIES {
            return Err("too many lobbies");
        }
        let id = uuid::Uuid::new_v4().to_string();
        let name = generate_lobby_name();
        let lobby = Lobby::new(id.clone(), name.clone(), public, password);
        self.lobbies.insert(id.clone(), Arc::new(Mutex::new(lobby)));
        Ok((id, name))
    }

    pub fn remove_lobby(&mut self, id: &str) {
        self.lobbies.remove(id);
    }

    pub fn get_lobby(&self, id: &str) -> Option<Arc<Mutex<Lobby>>> {
        self.lobbies.get(id).cloned()
    }
}

fn generate_lobby_name() -> String {
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
