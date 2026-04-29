import type {PhaseProps} from "../types";
import {playerName} from "../util";

export function ResultsPhase({state, isHost, send}: PhaseProps) {
	const vetoedGames = state.games.filter((g) => g.vetoed_by !== null);

	if (!state.results || state.results.length === 0) {
		return (
			<section className="card">
				<h2 className="section-title">Results</h2>
				<p className="hint-text">Calculating results…</p>
			</section>
		);
	}

	const sorted = [...state.results].sort((a, b) => a.rank - b.rank);
	const totalVoters = state.players.length;
	const maxScore = sorted[0]?.score ?? 0;

	const scoreGroups = new Map<number, typeof sorted>();
	for (const entry of sorted) {
		const group = scoreGroups.get(entry.score) ?? [];
		group.push(entry);
		scoreGroups.set(entry.score, group);
	}
	const drawScores = new Set(
		[...scoreGroups.entries()].filter(([, group]) => group.length > 1).map(([score]) => score)
	);

	return (
		<>
			<section className="card">
				<h2 className="section-title results-title">Results</h2>
				<p className="hint-text">
					{totalVoters} {totalVoters === 1 ? "player" : "players"} voted across{" "}
					{sorted.length} {sorted.length === 1 ? "option" : "options"}
				</p>
				<ol className="results-list">
					{sorted.map((entry) => {
						const isFirst = entry.rank === 1;
						const isDraw = drawScores.has(entry.score);
						const barWidth = maxScore > 0 ? (entry.score / maxScore) * 100 : 0;

						return (
							<li
								key={entry.game_id}
								className={`results-item ${isFirst ? "results-item--first" : ""} ${isDraw ? "results-item--draw" : ""}`}
							>
								<div className="results-item-header">
									<span className="results-rank">
										{isFirst ? "🏆" : `#${entry.rank}`}
									</span>
									<span className="results-game-name">
										{entry.game_name}
										{isDraw && <span className="draw-badge">DRAW</span>}
									</span>
									<span className="results-score">{entry.score} pts</span>
								</div>
								<div className="results-bar-track">
									<div
										className={`results-bar-fill ${isFirst ? "results-bar-fill--first" : ""}`}
										style={{width: `${barWidth}%`}}
									/>
								</div>
							</li>
						);
					})}
				</ol>
			</section>

			{vetoedGames.length > 0 && (
				<section className="card">
					<h2 className="section-title">Vetoed Options</h2>
					<ul className="game-list">
						{vetoedGames.map((game) => (
							<li key={game.id} className="game-item game-item--vetoed game-item--dim">
								<div className="game-item-info">
									<span className="game-item-name game-item-name--crossed">
										{game.name}
									</span>
									{game.vetoed_by !== null && (
										<span className="game-item-meta">
											vetoed by {playerName(state.players, game.vetoed_by)}
										</span>
									)}
								</div>
							</li>
						))}
					</ul>
				</section>
			)}

			{isHost && (
				<section className="card">
					<button
						className="btn btn-primary btn-full btn-large"
						onClick={() => send({ty: "reset_session"})}
					>
						Play Again
					</button>
				</section>
			)}
		</>
	);
}
