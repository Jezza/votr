import {useEffect, useRef, useState} from "react";
import type {Game, PhaseProps} from "../types";
import {PlayerStatus} from "../components/PlayerStatus";
import {playerName} from "../util";

function eligibleGames(games: Game[]): Game[] {
	return games.filter((g) => g.vetoed_by === null);
}

interface DragState {
	fromIndex: number;
	toIndex: number;
}

export function VotingPhase({state, myId, isHost, send, myPlayer}: PhaseProps) {
	const eligible = eligibleGames(state.games);
	const eligibleKey = eligible.map((g) => g.id).join(",");
	const [ranking, setRanking] = useState<string[]>(() => eligible.map((g) => g.id));
	const [abstaining, setAbstaining] = useState<string[]>([]);
	const [submitted, setSubmitted] = useState(false);
	const [drag, setDrag] = useState<DragState | null>(null);
	const listRef = useRef<HTMLOListElement>(null);

	useEffect(() => {
		setRanking(eligible.map((g) => g.id));
		setAbstaining([]);
		setSubmitted(false);
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [eligibleKey]);

	const alreadySubmitted = myPlayer?.ready ?? false;
	const isSubmitted = submitted || alreadySubmitted;

	const gameById = (id: string): Game | undefined => state.games.find((g) => g.id === id);

	const displayRanking = drag
		? (() => {
			const next = [...ranking];
			const [item] = next.splice(drag.fromIndex, 1);
			next.splice(drag.toIndex, 0, item!);
			return next;
		})()
		: ranking;

	const getHoverIndex = (clientY: number): number => {
		if (!listRef.current) return drag?.fromIndex ?? 0;
		const items = Array.from(listRef.current.children) as HTMLElement[];
		for (let i = 0; i < items.length; i++) {
			const rect = items[i]!.getBoundingClientRect();
			if (clientY < rect.top + rect.height / 2) return i;
		}
		return items.length - 1;
	};

	const handleDragStart = (e: React.PointerEvent<HTMLSpanElement>, index: number) => {
		e.preventDefault();
		e.currentTarget.setPointerCapture(e.pointerId);
		setDrag({fromIndex: index, toIndex: index});
	};

	const handlePointerMove = (e: React.PointerEvent<HTMLOListElement>) => {
		if (!drag) return;
		const hoverIndex = getHoverIndex(e.clientY);
		if (hoverIndex !== drag.toIndex) {
			setDrag((prev) => (prev ? {...prev, toIndex: hoverIndex} : null));
		}
	};

	const handlePointerUp = () => {
		if (!drag) return;
		if (drag.fromIndex !== drag.toIndex) {
			setRanking((prev) => {
				const next = [...prev];
				const [item] = next.splice(drag.fromIndex, 1);
				next.splice(drag.toIndex, 0, item!);
				return next;
			});
		}
		setDrag(null);
	};

	const moveToAbstaining = (gameId: string) => {
		setRanking((prev) => prev.filter((id) => id !== gameId));
		setAbstaining((prev) => [...prev, gameId]);
	};

	const moveToRanking = (gameId: string) => {
		setAbstaining((prev) => prev.filter((id) => id !== gameId));
		setRanking((prev) => [...prev, gameId]);
	};

	const handleSubmit = () => {
		send({ty: "submit_vote", ranking});
		setSubmitted(true);
	};

	const submittedCount = state.players.filter((p) => p.ready).length;
	const totalCount = state.players.length;

	const draggingOriginalId = drag ? ranking[drag.fromIndex] : null;

	return (
		<>
			{isSubmitted ? (
				<section className="card">
					<div className="submitted-banner">
						<span className="submitted-icon">✓</span>
						<span className="submitted-text">Vote submitted!</span>
					</div>
					<p className="hint-text center-text">
						Waiting for others… ({submittedCount}/{totalCount} submitted)
					</p>
				</section>
			) : (
				<section className="card">
					<h2 className="section-title">Submit Your Vote</h2>
					<p className="hint-text">
						Drag ⠿ to reorder. Remove options you don't want to rank.
					</p>
				</section>
			)}

			<section className="card">
				<h2 className="section-title">
					Your Ranking{" "}
					{ranking.length > 0 && <span className="count-badge">{ranking.length}</span>}
				</h2>
				{displayRanking.length === 0 ? (
					<p className="empty-hint">No options in your ranking. Add options from below.</p>
				) : (
					<ol
						className="ranking-list"
						ref={listRef}
						onPointerMove={handlePointerMove}
						onPointerUp={handlePointerUp}
						onPointerCancel={handlePointerUp}
						style={{touchAction: "none"}}
					>
						{displayRanking.map((gameId, index) => {
							const game = gameById(gameId);
							if (!game) return null;
							const isDragging = gameId === draggingOriginalId && drag !== null;
							return (
								<li
									key={gameId}
									className={`ranking-item ${isDragging ? "ranking-item--dragging" : ""}`}
								>
									<span
										className={`drag-handle ${isSubmitted ? "drag-handle--disabled" : ""}`}
										onPointerDown={isSubmitted ? undefined : (e) => handleDragStart(e, index)}
										title="Drag to reorder"
										aria-label="Drag handle"
									>
										⠿
									</span>
									<span className="ranking-position">{index + 1}</span>
									<div className="ranking-item-info">
										<span className="game-item-name">{game.name}</span>
										<span className="game-item-meta">
											by {playerName(state.players, game.suggested_by)}
										</span>
									</div>
									<button
										className="btn btn-icon btn-outline"
										onClick={() => moveToAbstaining(gameId)}
										disabled={isSubmitted}
										aria-label={`Remove ${game.name} from ranking`}
										title="Remove from ranking"
									>
										×
									</button>
								</li>
							);
						})}
					</ol>
				)}
			</section>

			{abstaining.length > 0 && (
				<section className="card">
					<h2 className="section-title">Abstaining</h2>
					<ul className="game-list">
						{abstaining.map((gameId) => {
							const game = gameById(gameId);
							if (!game) return null;
							return (
								<li key={gameId} className="game-item game-item--dim">
									<div className="game-item-info">
										<span className="game-item-name">{game.name}</span>
										<span className="game-item-meta">
											by {playerName(state.players, game.suggested_by)}
										</span>
									</div>
									<button
										className="btn btn-ghost"
										onClick={() => moveToRanking(gameId)}
										disabled={isSubmitted}
										aria-label={`Add ${game.name} to ranking`}
									>
										Add
									</button>
								</li>
							);
						})}
					</ul>
				</section>
			)}

			{!isSubmitted && (
				<section className="card">
					<button
						className="btn btn-primary btn-full btn-large"
						onClick={handleSubmit}
						disabled={ranking.length === 0}
					>
						Submit Vote ({ranking.length} option{ranking.length !== 1 ? "s" : ""} ranked)
					</button>
				</section>
			)}

			<PlayerStatus players={state.players} myId={myId} title="Submissions"/>

			{isHost && (
				<section className="card">
					<button
						className="btn btn-primary btn-full btn-large"
						onClick={() => send({ty: "advance_phase"})}
					>
						Advance to Results ({submittedCount}/{totalCount} submitted)
					</button>
				</section>
			)}
		</>
	);
}
