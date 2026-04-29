import React, {
	useCallback,
	useEffect,
	useRef,
	useState
} from "react";
import * as types from './types';
import {LobbyPhase} from "./phases/Lobby";
import {AddingPhase} from "./phases/Adding";
import {VetoingPhase} from "./phases/Vetoing";
import {VotingPhase} from "./phases/Voting";
import {ResultsPhase} from "./phases/Results";
import {LobbyBrowser} from "./LobbyBrowser";
import {Toast} from "./Toast";
import {GitHubLink} from "./GitHubLink";
import {NameChip} from "./components/NameChip";
import {apiBase} from "./api";
import {
	APP_CONTEXT,
	type AppStore,
	createAppStore,
	useApp
} from "./store";
import {generateSeriousName} from "./names";

const RECONNECT_DELAY = 2000;

const PHASE_LABELS: Record<string, string> = {
	lobby: "Lobby",
	adding: "Adding",
	vetoing: "Vetoing",
	voting: "Voting",
	results: "Results",
};

function getLobbyIdFromUrl(): string | null {
	const params = new URLSearchParams(window.location.search);
	return params.get("lobby");
}

const AppInner = React.memo(() => {
	const wsRef = useRef<WebSocket | null>(null);
	// const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
	// const suppressReconnect = useRef(false);

	const [lobbyId, setLobbyId] = useState<string | null>(getLobbyIdFromUrl);
	const [lobbyPassword, setLobbyPassword] = useState<string | undefined>();
	const [state, setState] = useState<types.ServerState | null>(null);
	const [connected, setConnected] = useState(false);
	const [kicked, setKicked] = useState(false);
	const [toast, setToast] = useState<{ message: string; level: "info" | "error" | "warning" } | null>(null);
	const hasReceivedState = useRef(false);

	const {playerId, playerName, setPlayerName} = useApp(store => ({
		playerId: store.playerId,
		playerName: store.playerName,
		setPlayerName: store.setPlayerName,
	}));

	const send = useCallback((msg: types.Outgoing) => {
		const ws = wsRef.current;
		if (ws && ws.readyState === WebSocket.OPEN) {
			ws.send(JSON.stringify(msg));
		}
	}, []);

	const disconnect = useCallback(() => {
		// suppressReconnect.current = true;
		// if (reconnectTimer.current) {
		// 	clearTimeout(reconnectTimer.current);
		// 	reconnectTimer.current = null;
		// }
		if (wsRef.current) {
			wsRef.current.onclose = null;
			wsRef.current.onerror = null;
			wsRef.current.onmessage = null;
			wsRef.current.onopen = null;
			wsRef.current.close();
			wsRef.current = null;
		}
	}, []);

	const navigateToLobby = useCallback((id: string, password?: string) => {
		disconnect();
		hasReceivedState.current = false;
		setKicked(false);
		setState(null);
		setLobbyId(id);
		setLobbyPassword(password);
		window.history.pushState(null, "", `${apiBase}/?lobby=${id}`);
	}, [disconnect]);

	const navigateToBrowser = useCallback((toastMsg?: string) => {
		disconnect();
		hasReceivedState.current = false;
		setLobbyId(null);
		setLobbyPassword(undefined);
		setState(null);
		// setMyId(null);
		setKicked(false);
		window.history.pushState(null, "", `${apiBase}/`);
		if (toastMsg) {
			setToast({message: toastMsg, level: "info"});
		}
	}, [disconnect]);

	const connect = useCallback(() => {
		if (!lobbyId) { //  || suppressReconnect.current
			return;
		}

		if (wsRef.current) {
			disconnect();
			// wsRef.current.onclose = null;
			// wsRef.current.onerror = null;
			// wsRef.current.onmessage = null;
			// wsRef.current.onopen = null;
			// wsRef.current.close();
		}

		const name = playerName.trim() || generateSeriousName();
		if (name !== playerName) setPlayerName(name);

		const wsProto = window.location.protocol === "https:" ? "wss:" : "ws:";
		const params = new URLSearchParams();
		params.set("lobby_id", lobbyId);
		params.set("player_id", playerId);
		params.set("name", name);
		if (lobbyPassword) params.set("password", lobbyPassword);
		const wsUrl = `${wsProto}//${window.location.host}${apiBase}/ws?${params.toString()}`;

		const ws = new WebSocket(wsUrl);
		wsRef.current = ws;
		let opened = false;

		ws.onopen = () => {
			opened = true;
			setConnected(true);
		};

		ws.onclose = () => {
			setConnected(false);
			if (!opened) {
				// Server rejected the upgrade (e.g. lobby missing) — bail out instead of looping reconnects.
				navigateToBrowser("Could not join lobby");
			}
		};

		ws.onerror = () => {
			ws.close();
		};

		ws.onmessage = (event: MessageEvent) => {
			let incoming: types.Incoming;
			try {
				const msg: any = JSON.parse(event.data);

				if (typeof msg === "object" && 'ty' in msg) {
					incoming = msg as types.Incoming;
				} else {
					console.log("Unknown message type: ", msg);
					return;
				}
			} catch (e) {
				console.log("Unable to parse data, skipping...", e);
				return;
			}

			console.log(incoming);

			switch (incoming.ty) {
				case "welcome": {
					setKicked(false);
					break;
				}
				case "state": {
					const {ty: _ty, ...state} = incoming;
					const newState = state as types.ServerState;

					hasReceivedState.current = true;
					setState(newState);
					break;
				}
				case "error": {
					console.error("Server error:", incoming.message);
					break;
				}
				case "kicked": {
					setKicked(true);
					break;
				}
				case "lobby_closed": {
					navigateToBrowser("Lobby was closed");
					break;
				}
				case "toast": {
					setToast({message: incoming.message, level: incoming.level});
					// If we haven't received state yet, the lobby rejected us
					// if (!hasReceivedState.current) {
					// 	suppressReconnect.current = true;
					// 	navigateToBrowser(msg.message);
					// }
					break;
				}
			}
		};
	}, [lobbyId, lobbyPassword, navigateToBrowser]);

	// Generate silly name on first visit
	useEffect(() => {
		if (!playerName) {
			setPlayerName(generateSeriousName());
		}
	}, [playerName, setPlayerName]);

	// Connect when lobbyId changes
	useEffect(() => {
		if (lobbyId) {
			// suppressReconnect.current = false;
			connect();
		}
		return () => {
			// if (reconnectTimer.current) {
			// 	clearTimeout(reconnectTimer.current);
			// }
			if (wsRef.current) {
				wsRef.current.onclose = null;
				wsRef.current.close();
			}
		};
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [lobbyId]);

	// Handle browser back/forward
	useEffect(() => {
		const handlePopState = () => {
			const id = getLobbyIdFromUrl();
			if (id !== lobbyId) {
				if (id) {
					navigateToLobby(id);
				} else {
					navigateToBrowser();
				}
			}
		};
		window.addEventListener("popstate", handlePopState);
		return () => window.removeEventListener("popstate", handlePopState);
	}, [lobbyId, navigateToLobby, navigateToBrowser]);

	const myPlayer: types.Player | undefined = state?.players.find((p) => p.id === playerId);
	const isHost = playerId !== null && state?.host_id === playerId;

	const commitName = (name: string) => {
		setPlayerName(name);
		send({ty: "set_name", name});
	};

	// No lobby selected — show browser
	if (!lobbyId) {
		return (
			<>
				{toast && (
					<Toast message={toast.message} level={toast.level} onDismiss={() => setToast(null)}/>
				)}
				<LobbyBrowser onJoin={navigateToLobby}/>
			</>
		);
	}

	const renderPhase = () => {
		if (kicked) {
			return (
				<div className="connecting-screen">
					<div className="connecting-logo">Votr</div>
					<div className="connecting-status">You have been kicked from the session.</div>
				</div>
			);
		}

		if (!state) {
			return (
				<div className="connecting-screen">
					<div className="connecting-logo">Votr</div>
					<div className="connecting-status">
						{connected ? "Connected — waiting for state…" : "Reconnecting…"}
					</div>
					<div className="connecting-dots">
						<span className="dot-pulse"/>
						<span className="dot-pulse"/>
						<span className="dot-pulse"/>
					</div>
				</div>
			);
		}

		const props = {state, myId: playerId, isHost, send, myPlayer};

		switch (state.phase) {
			case "lobby":
				return <LobbyPhase {...props} />;
			case "adding":
				return <AddingPhase {...props} />;
			case "vetoing":
				return <VetoingPhase {...props} />;
			case "voting":
				return <VotingPhase {...props} />;
			case "results":
				return <ResultsPhase {...props} />;
			default:
				return <div className="content-area"><p>Unknown phase.</p></div>;
		}
	};

	return (
		<div className="app">
			{toast && (
				<Toast message={toast.message} level={toast.level} onDismiss={() => setToast(null)}/>
			)}
			{state && (
				<header className="top-bar">
					<div className="top-bar-left">
						<button
							type="button"
							className="top-bar-title"
							onClick={() => navigateToBrowser()}
							title="Back to lobby browser"
						>
							Votr
						</button>
						<span className="lobby-name-badge">{state.lobby_name}</span>
						<GitHubLink/>
					</div>
					<div className="top-bar-right">
						<span className="phase-badge">{PHASE_LABELS[state.phase] ?? state.phase}</span>
						<NameChip name={playerName} onCommit={commitName}/>
						<button
							className="btn btn-outline btn-small"
							onClick={() => navigateToBrowser()}
							title="Leave lobby"
						>
							Leave
						</button>
					</div>
				</header>
			)}
			<main className="content-area">
				<div className="content-inner">
					{renderPhase()}
				</div>
			</main>
		</div>
	);
});

export const App = React.memo(() => {
	const ref = React.useRef<AppStore | null>(null);
	if (!ref.current) {
		ref.current = createAppStore();
	}

	return (
		<APP_CONTEXT value={ref.current}>
			<AppInner/>
		</APP_CONTEXT>
	);
});

