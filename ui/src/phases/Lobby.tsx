import type {PhaseProps} from "../types";
import {LobbySettingsForm} from "../components/LobbySettingsForm";
import {PlayerStatus} from "../components/PlayerStatus";

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
						{maxVetoes === 0
							? "No vetoes"
							: `${maxVetoes} veto${maxVetoes !== 1 ? "s" : ""}`}
					</p>
				</section>
			)}

			<PlayerStatus
				players={state.players}
				myId={myId}
				hostId={state.host_id}
				isHost={isHost}
				onKick={(id) => send({ty: "kick_player", target_id: id})}
				showReady={false}
				title="Players"
			/>

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
