export interface Player {
  id: string;
  name: string;
  connected: boolean;
  ready: boolean;
  disconnect_timeout: number | null;
}

export interface Game {
  id: string;
  name: string;
  suggested_by: string;
  vetoed_by: string[];
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
  votes_submitted: string[];
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
export type ServerMessage =
  | { type: "welcome"; player_id: string; lobby_id: string }
  | ({ type: "state" } & ServerState)
  | { type: "error"; message: string }
  | { type: "kicked" }
  | { type: "toast"; message: string; level: "info" | "error" | "warning" }
  | { type: "lobby_closed" };

// Client → Server messages
export type ClientMessage =
  | { type: "set_name"; name: string }
  | { type: "add_game"; name: string }
  | { type: "remove_game"; game_id: string }
  | { type: "veto_game"; game_id: string }
  | { type: "unveto_game"; game_id: string }
  | { type: "submit_vote"; ranking: string[] }
  | { type: "set_ready"; ready: boolean }
  | { type: "advance_phase" }
  | { type: "reset_session" }
  | { type: "set_max_vetoes"; count: number }
  | { type: "kick_player"; target_id: string }
  | { type: "set_lobby_public"; public: boolean }
  | { type: "set_lobby_password"; password: string | null }
  | { type: "set_lobby_locked"; locked: boolean }
  | { type: "close_lobby" };

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
  send: (msg: ClientMessage) => void;
  myPlayer: Player | undefined;
  getCountdown: (playerId: string, timeout: number) => number;
}
