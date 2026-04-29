import {useEffect, useState} from "react";

function formatElapsed(seconds: number): string {
	if (seconds < 5) return "just now";
	if (seconds < 60) return `${seconds}s ago`;

	const minutes = Math.floor(seconds / 60);
	const rs = seconds % 60;
	if (minutes < 60) {
		return rs === 0 ? `${minutes}m ago` : `${minutes}m ${rs}s ago`;
	}

	const hours = Math.floor(minutes / 60);
	const rm = minutes % 60;
	if (hours < 24) {
		return rm === 0 ? `${hours}h ago` : `${hours}h ${rm}m ago`;
	}

	const days = Math.floor(hours / 24);
	const rh = hours % 24;
	return rh === 0 ? `${days}d ago` : `${days}d ${rh}h ago`;
}

export function DisconnectTimer({at}: { at: number }) {
	const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));

	useEffect(() => {
		const id = setInterval(() => setNow(Math.floor(Date.now() / 1000)), 1000);
		return () => clearInterval(id);
	}, []);

	const elapsed = Math.max(0, now - at);
	return <span className="disconnect-timer" title="Disconnected"> ({formatElapsed(elapsed)})</span>;
}
