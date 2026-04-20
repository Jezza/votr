import type {PhaseProps} from "../types";

export function LobbyPhase({state, myId, isHost, send, getCountdown}: PhaseProps) {
	const maxVetoes = state.max_vetoes;

	return (
		<>
			{isHost && state.phase === "lobby" ? (
				<section className="card">
					<h2 className="section-title">Settings</h2>
					<div className="veto-stepper">
						<span className="veto-label">Vetoes per player</span>
						<button
							className="btn btn-outline btn-icon"
							onClick={() => send({ty: "set_max_vetoes", count: maxVetoes - 1})}
							disabled={maxVetoes <= 1}
							aria-label="Decrease veto count"
						>
							−
						</button>
						<span className="veto-count">{maxVetoes}</span>
						<button
							className="btn btn-outline btn-icon"
							onClick={() => send({ty: "set_max_vetoes", count: maxVetoes + 1})}
							disabled={maxVetoes >= 10}
							aria-label="Increase veto count"
						>
							+
						</button>
					</div>
				</section>
			) : (
				<section className="card">
					<p className="hint-text">
						Each player gets {maxVetoes} veto{maxVetoes !== 1 ? "s" : ""}
					</p>
				</section>
			)}

			{isHost && (
				<section className="card">
					<h2 className="section-title">Lobby Settings</h2>
					<div className="lobby-settings">
						<label className="lobby-toggle">
							<span>Public</span>
							<input
								type="checkbox"
								checked={state.lobby_public}
								onChange={(e) => send({ty: "set_lobby_public", public: e.target.checked})}
							/>
						</label>
						<label className="lobby-toggle">
							<span>Locked</span>
							<input
								type="checkbox"
								checked={state.lobby_locked}
								onChange={(e) => send({ty: "set_lobby_locked", locked: e.target.checked})}
							/>
						</label>
						<div className="lobby-password-row">
							<input
								className="lobby-password-input"
								type="text"
								placeholder={state.lobby_has_password ? "Change password" : "Set password"}
								maxLength={64}
								onKeyDown={(e) => {
									if (e.key === "Enter") {
										const value = (e.target as HTMLInputElement).value;
										send({ty: "set_lobby_password", password: value || null});
										(e.target as HTMLInputElement).value = "";
									}
								}}
							/>
							{state.lobby_has_password && (
								<button
									className="btn btn-outline btn-small"
									onClick={() => send({ty: "set_lobby_password", password: null})}
								>
									Remove
								</button>
							)}
						</div>
						<button
							className="btn btn-danger btn-full"
							onClick={() => {
								if (window.confirm("Close this lobby? All players will be disconnected.")) {
									send({ty: "close_lobby"});
								}
							}}
						>
							Close Lobby
						</button>
					</div>
				</section>
			)}

			{!isHost && (
				<section className="card">
					<p className="hint-text">
						{state.lobby_name}
						{state.lobby_locked && " · Locked"}
						{!state.lobby_public && " · Private"}
					</p>
				</section>
			)}

			<section className="card">
				<h2 className="section-title">Players</h2>
				<ul className="player-list">
					{state.players.map((player) => (
						<li key={player.id} className="player-item">
              <span
				  className={`connected-dot ${player.connected ? "connected-dot--on" : "connected-dot--off"}`}
	              title={player.connected ? "Connected" : "Disconnected"}
			  >
                ●
              </span>
							<span className="player-name">
                {player.name}
								{player.id === myId && (
									<span className="you-label"> (you)</span>
								)}
								{player.disconnect_timeout != null && (
									<span className="disconnect-timer"> ({getCountdown(player.id, player.disconnect_timeout)}s)</span>
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
					))}
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
