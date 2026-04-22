use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[serde(transparent)]
pub struct PlayerId(pub uuid::Uuid);

impl std::fmt::Display for PlayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PlayerId(")?;
        self.0.fmt(f)?;
        f.write_str(")")
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[serde(transparent)]
pub struct LobbyId(pub uuid::Uuid);

impl LobbyId {
    pub fn rand() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl std::fmt::Display for LobbyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LobbyId(")?;
        self.0.fmt(f)?;
        f.write_str(")")
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[serde(transparent)]
pub struct OptId(uuid::Uuid);

impl OptId {
    pub fn rand() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl std::fmt::Display for OptId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OptId(")?;
        self.0.fmt(f)?;
        f.write_str(")")
    }
}

#[derive(Serialize)]
pub struct LobbyInfo {
    pub id: LobbyId,
    pub name: String,
    pub player_count: usize,
    pub max_players: usize,
    pub has_password: bool,
    pub locked: bool,
    pub phase: Phase,
}

#[derive(Deserialize)]
pub struct CreateLobbyRequest {
    pub public: Option<bool>,
    pub password: Option<String>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
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
    pub id: PlayerId,
    pub name: String,
    pub connection_status: ConnectionStatus,
    pub ready: bool,
}

impl Player {
    pub fn is_connected(&self) -> bool {
        matches!(self.connection_status, ConnectionStatus::Connected(_))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "ty", rename_all = "snake_case")]
pub enum ConnectionStatus {
    Connected(Connected),
    /// Seconds remaining before this player is removed (None if connected)
    Disconnected(Disconnected),
}

impl ConnectionStatus {
    pub fn connected() -> Self {
        Self::Connected(Connected {})
    }

    pub fn disconnected(timeout: u32) -> Self {
        Self::Disconnected(Disconnected { timeout })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Connected {}

#[derive(Debug, Clone, Serialize)]
pub struct Disconnected {
    timeout: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Opt {
    pub id: OptId,
    pub name: String,
    pub suggested_by: PlayerId,
    pub vetoed_by: Option<PlayerId>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoteResult {
    pub game_id: OptId,
    pub game_name: String,
    pub score: u32,
    pub rank: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "ty", rename_all = "snake_case")]
pub enum Outgoing {
    Welcome(Welcome),
    Error(Error),
    Kicked(Kicked),
    Toast(Toast),
    LobbyClosed(LobbyClosed),
    #[serde(rename = "state")]
    LobbyState(LobbyState),
}

impl From<Welcome> for Outgoing {
    fn from(value: Welcome) -> Self {
        Self::Welcome(value)
    }
}

impl From<Error> for Outgoing {
    fn from(value: Error) -> Self {
        Self::Error(value)
    }
}

impl From<Toast> for Outgoing {
    fn from(value: Toast) -> Self {
        Self::Toast(value)
    }
}

impl From<LobbyClosed> for Outgoing {
    fn from(value: LobbyClosed) -> Self {
        Self::LobbyClosed(value)
    }
}

impl From<Kicked> for Outgoing {
    fn from(value: Kicked) -> Self {
        Self::Kicked(value)
    }
}

impl From<LobbyState> for Outgoing {
    fn from(value: LobbyState) -> Self {
        Self::LobbyState(value)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "ty", rename_all = "snake_case")]
pub enum Incoming {
    SetName(SetName),
    AddGame(AddGame),
    RemoveGame(RemoveGame),
    VetoGame(VetoGame),
    UnvetoGame(UnvetoGame),
    SubmitVote(SubmitVote),
    SetReady(SetReady),
    AdvancePhase,
    ResetSession,
    SetMaxVetoes(SetMaxVetoes),
    KickPlayer(KickPlayer),
    SetLobbyPublic(SetLobbyPublic),
    SetLobbyPassword(SetLobbyPassword),
    SetLobbyLocked(SetLobbyLocked),
    CloseLobby,
}

#[derive(Debug, Clone, Serialize)]
pub struct LobbyState {
    pub phase: Phase,
    pub players: Vec<Player>,
    pub games: Vec<Opt>,
    pub results: Option<Vec<VoteResult>>,
    pub host_id: Option<PlayerId>,
    pub max_vetoes: u32,
    pub lobby_id: LobbyId,
    pub lobby_name: String,
    pub lobby_public: bool,
    pub lobby_locked: bool,
    pub lobby_has_password: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Welcome {
    pub lobby_id: String,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Kicked {}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct LobbyClosed {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToastLevel {
    Info,
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
}

impl Toast {
    pub fn new(value: impl Into<String>, level: ToastLevel) -> Self {
        Self {
            message: value.into(),
            level,
        }
    }

    pub fn info(value: impl Into<String>) -> Self {
        Self::new(value, ToastLevel::Info)
    }

    pub fn warn(value: impl Into<String>) -> Self {
        Self::new(value, ToastLevel::Warning)
    }

    pub fn error(value: impl Into<String>) -> Self {
        Self::new(value, ToastLevel::Error)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetName {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddGame {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoveGame {
    pub game_id: OptId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VetoGame {
    pub game_id: OptId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnvetoGame {
    pub game_id: OptId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitVote {
    pub ranking: Vec<OptId>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetReady {
    pub ready: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetMaxVetoes {
    pub count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KickPlayer {
    pub target_id: PlayerId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetLobbyPublic {
    pub public: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetLobbyPassword {
    pub password: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetLobbyLocked {
    pub locked: bool,
}

#[derive(Debug, Deserialize)]
pub struct JoinInfo {
    pub player_id: PlayerId,
    pub name: String,
    pub lobby_id: LobbyId,
    pub password: Option<String>,
}
