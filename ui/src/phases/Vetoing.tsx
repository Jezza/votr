import type {PhaseProps} from "../types";
import {PlayerStatus} from "../components/PlayerStatus";
import {playerName} from "../util";

export function VetoingPhase({state, myId, isHost, send, myPlayer}: PhaseProps) {
	const readyCount = state.players.filter((p) => p.ready).length;
	const totalCount = state.players.length;
	const amReady = myPlayer?.ready ?? false;
	const allVetoed = state.games.length > 0 && state.games.every((g) => g.vetoed_by !== null);

	const myVetoCount = myId === null
		? 0
		: state.games.filter((g) => g.vetoed_by === myId).length;
	const vetoesExhausted = myVetoCount >= state.max_vetoes;

	const handleToggleReady = () => {
		send({ty: "set_ready", ready: !amReady});
	};

	return (
		<>
			<section className="card">
				<h2 className="section-title">Veto Options</h2>
				<p className="hint-text">
					A single veto removes an option from voting.
					You have {Math.max(0, state.max_vetoes - myVetoCount)} of {state.max_vetoes} veto{state.max_vetoes !== 1 ? "es" : ""} remaining.
				</p>
			</section>

			<section className="card">
				<ul className="game-list">
					{state.games.map((game) => {
						const iVetoed = myId !== null && game.vetoed_by === myId;
						const hasVeto = game.vetoed_by !== null;
						const disableVeto = !iVetoed && (hasVeto || vetoesExhausted);

						return (
							<li
								key={game.id}
								className={`game-item ${hasVeto ? "game-item--vetoed" : ""}`}
							>
								<div className="game-item-info">
									<span className="game-item-name">{game.name}</span>
									<span className="game-item-meta">
										by {playerName(state.players, game.suggested_by)}
									</span>
									{hasVeto && game.vetoed_by !== null && (
										<span className="veto-count-badge">
											vetoed by {playerName(state.players, game.vetoed_by)}
										</span>
									)}
								</div>
								<button
									className={`btn btn-icon ${iVetoed ? "btn-danger btn-active" : "btn-outline"}`}
									onClick={() =>
										send(
											iVetoed
												? {ty: "unveto_game", game_id: game.id}
												: {ty: "veto_game", game_id: game.id}
										)
									}
									disabled={disableVeto}
									title={
										iVetoed
											? "Remove veto"
											: hasVeto
												? "Already vetoed by another player"
												: vetoesExhausted
													? "No vetoes remaining"
													: "Veto this option"
									}
									aria-label={iVetoed ? `Remove veto from ${game.name}` : `Veto ${game.name}`}
								>
									{iVetoed ? "✓ Vetoed" : "Veto"}
								</button>
							</li>
						);
					})}
				</ul>
			</section>

			<section className="card">
				<button
					className={`btn btn-full btn-large ${amReady ? "btn-success btn-active" : "btn-outline"}`}
					onClick={handleToggleReady}
				>
					{amReady ? "✓ Done Vetoing" : "Done Vetoing"}
				</button>
			</section>

			<PlayerStatus
				players={state.players}
				myId={myId}
				hostId={state.host_id}
				isHost={isHost}
				onKick={(id) => send({ty: "kick_player", target_id: id})}
			/>

			{allVetoed && (
				<section className="card">
					<p className="hint-text" style={{color: "#c45050"}}>
						All options have been vetoed! Remove some vetoes or ask the host to reset the session.
					</p>
				</section>
			)}

			{isHost && (
				<section className="card">
					<button
						className="btn btn-primary btn-full btn-large"
						onClick={() => {
							if (readyCount < totalCount && !window.confirm(
								`Only ${readyCount}/${totalCount} players are ready. Advance to Voting anyway?`
							)) return;
							send({ty: "advance_phase"});
						}}
						disabled={allVetoed}
					>
						{allVetoed
							? "Can't advance — all options vetoed"
							: `Advance to Voting (${readyCount}/${totalCount} ready)`}
					</button>
				</section>
			)}
		</>
	);
}
