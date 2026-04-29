import React, {useState} from "react";

interface Props {
	name: string;
	onCommit: (name: string) => void;
}

export function NameChip({name, onCommit}: Props) {
	const [editing, setEditing] = useState(false);
	const [input, setInput] = useState("");

	const commit = () => {
		const trimmed = input.trim();
		if (trimmed) onCommit(trimmed);
		setEditing(false);
	};

	const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
		if (e.key === "Enter") commit();
		if (e.key === "Escape") setEditing(false);
	};

	if (editing) {
		return (
			<input
				className="name-edit-input"
				type="text"
				value={input}
				onChange={(e) => setInput(e.target.value)}
				onKeyDown={handleKeyDown}
				onBlur={commit}
				autoFocus
				maxLength={32}
			/>
		);
	}

	return (
		<button
			className="name-chip"
			onClick={() => {
				setInput(name === "…" ? "" : name);
				setEditing(true);
			}}
			title="Click to change your name"
		>
			{name}
		</button>
	);
}
