import type {Player} from "./types";

export function playerName(players: Player[], id: string): string {
	return players.find((p) => p.id === id)?.name ?? "Unknown";
}
