import type {PhaseProps} from "../types";
import {DisconnectTimer} from "../components/DisconnectTimer";
import {LobbySettingsForm} from "../components/LobbySettingsForm";

export function LobbyPhase({state, myId, isHost, send}: PhaseProps) {
	const maxVetoes = state.max_vetoes;

	return (
		<>
			{isHost ? (
				<LobbySettingsForm state={state} send={send}/>
			) : (
				<section className="card">
					<p className="hint-text">
						{state.lobby_name}
						{state.lobby_locked && " · Locked"}
						{!state.lobby_public && " · Private"}
					</p>
					<p className="hint-text">
						Each player gets {maxVetoes} veto{maxVetoes !== 1 ? "s" : ""}
					</p>
				</section>
			)}

			<section className="card">
				<h2 className="section-title">Players</h2>
				<ul className="player-list">
					{state.players.map((player) => {
						const connected = player.connection_status.ty === "connected";
						const disconnectedAt = player.connection_status.ty === "disconnected"
							? player.connection_status.at
							: null;
						return (
							<li
								key={player.id}
								className={`player-item ${!connected ? "player-item--disconnected" : ""}`}
							>
								<span
									className={`connected-dot ${connected ? "connected-dot--on" : "connected-dot--off"}`}
									title={connected ? "Connected" : "Disconnected"}
								>
									●
								</span>
								<span className="player-name">
									{player.name}
									{player.id === myId && (
										<span className="you-label"> (you)</span>
									)}
									{disconnectedAt != null && (
										<DisconnectTimer at={disconnectedAt}/>
									)}
								</span>
								{player.id === state.host_id && (
									<span className="host-crown" title="Host">👑</span>
								)}
								{isHost && player.id !== myId && (
									<button
										className="btn btn-icon btn-danger btn-small"
										onClick={() => {
											if (window.confirm(`Kick ${player.name}?`)) {
												send({ty: "kick_player", target_id: player.id});
											}
										}}
										title={`Kick ${player.name}`}
									>
										✕
									</button>
								)}
							</li>
						);
					})}
				</ul>
			</section>

			{isHost && (
				<section className="card">
					<button
						className="btn btn-primary btn-full btn-large"
						onClick={() => send({ty: "advance_phase"})}
					>
						Start → Adding Options
					</button>
				</section>
			)}
		</>
	);
}
