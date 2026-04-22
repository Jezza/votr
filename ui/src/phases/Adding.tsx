import {useState} from "react";
import type {PhaseProps} from "../types";
import {PlayerStatus} from "../components/PlayerStatus";
import {playerName} from "../util";

export function AddingPhase({state, myId, isHost, send, myPlayer}: PhaseProps) {
	const [gameInput, setGameInput] = useState("");

	const readyCount = state.players.filter((p) => p.ready).length;
	const totalCount = state.players.length;
	const amReady = myPlayer?.ready ?? false;

	const handleAddGame = () => {
		const trimmed = gameInput.trim();
		if (!trimmed) return;
		send({ty: "add_game", name: trimmed});
		setGameInput("");
	};

	const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
		if (e.key === "Enter") handleAddGame();
	};

	const handleToggleReady = () => {
		send({ty: "set_ready", ready: !amReady});
	};

	return (
		<>
			<section className="card">
				<h2 className="section-title">Add Options</h2>
				<div className="input-row">
					<input
						className="text-input"
						type="text"
						placeholder="Option name…"
						value={gameInput}
						onChange={(e) => setGameInput(e.target.value)}
						onKeyDown={handleKeyDown}
						maxLength={80}
					/>
					<button
						className="btn btn-primary"
						onClick={handleAddGame}
						disabled={!gameInput.trim()}
					>
						Add
					</button>
				</div>
			</section>

			<section className="card">
				<h2 className="section-title">
					Options{" "}
					{state.games.length > 0 && (
						<span className="count-badge">{state.games.length}</span>
					)}
				</h2>
				{state.games.length === 0 ? (
					<p className="empty-hint">No options added yet. Be the first!</p>
				) : (
					<ul className="game-list">
						{state.games.map((game) => (
							<li key={game.id} className="game-item">
								<div className="game-item-info">
									<span className="game-item-name">{game.name}</span>
									<span className="game-item-meta">
										by {playerName(state.players, game.suggested_by)}
									</span>
								</div>
								{game.suggested_by === myId && (
									<button
										className="btn btn-danger btn-icon"
										onClick={() => send({ty: "remove_game", game_id: game.id})}
										title="Remove option"
										aria-label={`Remove ${game.name}`}
									>
										×
									</button>
								)}
							</li>
						))}
					</ul>
				)}
			</section>

			<section className="card">
				<button
					className={`btn btn-full btn-large ${amReady ? "btn-success btn-active" : "btn-outline"}`}
					onClick={handleToggleReady}
				>
					{amReady ? "✓ Ready" : "Not Ready"}
				</button>
			</section>

			<PlayerStatus players={state.players} myId={myId}/>

			{isHost && (
				<section className="card">
					<button
						className="btn btn-primary btn-full btn-large"
						onClick={() => send({ty: "advance_phase"})}
					>
						Advance to Vetoing ({readyCount}/{totalCount} ready)
					</button>
				</section>
			)}
		</>
	);
}
