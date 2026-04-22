import type {Player} from "../types";
import {DisconnectTimer} from "./DisconnectTimer";

interface Props {
	players: Player[];
	myId: string | null;
	title?: string;
}

export function PlayerStatus({players, myId, title = "Player Status"}: Props) {
	return (
		<section className="card">
			<h2 className="section-title">{title}</h2>
			<ul className="player-list">
				{players.map((player) => {
					const disconnectedAt = player.connection_status.ty === "disconnected"
						? player.connection_status.at
						: null;
					return (
						<li
							key={player.id}
							className={`player-item ${disconnectedAt != null ? "player-item--disconnected" : ""}`}
						>
							<span
								className={`ready-dot ${player.ready ? "ready-dot--on" : "ready-dot--off"}`}
							>
								{player.ready ? "✓" : "○"}
							</span>
							<span className="player-name">
								{player.name}
								{player.id === myId && <span className="you-label"> (you)</span>}
								{disconnectedAt != null && (
									<DisconnectTimer at={disconnectedAt}/>
								)}
							</span>
						</li>
					);
				})}
			</ul>
		</section>
	);
}
