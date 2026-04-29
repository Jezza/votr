export type ConnectionStatus =
	| { ty: "connected" }
	| { ty: "disconnected", at: number };

export interface Player {
	id: string;
	name: string;
	connection_status: ConnectionStatus;
	ready: boolean;
}

export interface Game {
	id: string;
	name: string;
	suggested_by: string;
	vetoed_by: string | null;
}

export interface ResultEntry {
	game_id: string;
	game_name: string;
	score: number;
	rank: number;
}

export type Phase = "lobby" | "adding" | "vetoing" | "voting" | "results";

export interface ServerState {
	phase: Phase;
	players: Player[];
	games: Game[];
	results: ResultEntry[] | null;
	host_id: string | null;
	max_vetoes: number;
	lobby_id: string;
	lobby_name: string;
	lobby_public: boolean;
	lobby_locked: boolean;
	lobby_has_password: boolean;
}

// Server → Client messages
export type Incoming =
	| { ty: "welcome" } & Welcome
	| ({ ty: "state" } & ServerState)
	| { ty: "error" } & Error
	| { ty: "kicked" }
	| { ty: "toast"} & Toast
	| { ty: "lobby_closed" };

// Client → Server messages
export type Outgoing =
	| { ty: "set_name" } & SetName
	| { ty: "add_game" } & AddGame
	| { ty: "remove_game" } & RemoveGame
	| { ty: "veto_game" } & VetoGame
	| { ty: "unveto_game" } & UnvetoGame
	| { ty: "submit_vote" } & SubmitVote
	| { ty: "set_ready" } & SetReady
	| { ty: "advance_phase" }
	| { ty: "reset_session" }
	| { ty: "set_max_vetoes" } & SetMaxVetoes
	| { ty: "kick_player" } & KickPlayer
	| { ty: "set_lobby_public" } & SetLobbyPublic
	| { ty: "set_lobby_password" } & SetLobbyPassword
	| { ty: "set_lobby_locked" } & SetLobbyLocked
	| { ty: "close_lobby" };

// @formatter:off
export type Welcome          = { lobby_id: string }
export type Error            = { message: string }
export type Toast            = { message: string, level: "info" | "error" | "warning" }

export type SetName          = { name: string }
export type AddGame          = { name: string }
export type RemoveGame       = { game_id: string }
export type VetoGame         = { game_id: string }
export type UnvetoGame       = { game_id: string }
export type SubmitVote       = { ranking: string[] }
export type SetReady         = { ready: boolean }
export type SetMaxVetoes     = { count: number }
export type KickPlayer       = { target_id: string }
export type SetLobbyPublic   = { public: boolean }
export type SetLobbyPassword = { password: string | null }
export type SetLobbyLocked   = { locked: boolean }
// @formatter:on

export interface LobbyInfo {
	id: string;
	name: string;
	player_count: number;
	max_players: number;
	has_password: boolean;
	locked: boolean;
	phase: string;
}

export interface PhaseProps {
	state: ServerState;
	myId: string | null;
	isHost: boolean;
	send: (msg: Outgoing) => void;
	myPlayer: Player | undefined;
}
