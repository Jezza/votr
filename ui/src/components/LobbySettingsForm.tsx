import type {Outgoing, ServerState} from "../types";

interface Props {
	state: ServerState;
	send: (msg: Outgoing) => void;
}

export function LobbySettingsForm({state, send}: Props) {
	const maxVetoes = state.max_vetoes;

	return (
		<section className="card">
			<h2 className="section-title">Lobby Settings</h2>
			<div className="lobby-settings">
				<div className="veto-stepper">
					<span className="veto-label">Vetoes per player</span>
					<button
						className="btn btn-outline btn-icon"
						onClick={() => send({ty: "set_max_vetoes", count: maxVetoes - 1})}
						disabled={maxVetoes <= 0}
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
	);
}
