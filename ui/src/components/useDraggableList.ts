import React, {useRef, useState} from "react";

interface DragState {
	fromIndex: number;
	toIndex: number;
}

export function useDraggableList<T, E extends HTMLElement = HTMLElement>(
	order: T[],
	setOrder: (next: T[]) => void,
) {
	const [drag, setDrag] = useState<DragState | null>(null);
	const listRef = useRef<E | null>(null);

	const displayOrder = drag
		? (() => {
			const next = [...order];
			const [item] = next.splice(drag.fromIndex, 1);
			next.splice(drag.toIndex, 0, item!);
			return next;
		})()
		: order;

	const getHoverIndex = (clientY: number): number => {
		if (!listRef.current) return drag?.fromIndex ?? 0;
		const items = Array.from(listRef.current.children) as HTMLElement[];
		for (let i = 0; i < items.length; i++) {
			const rect = items[i]!.getBoundingClientRect();
			if (clientY < rect.top + rect.height / 2) return i;
		}
		return items.length - 1;
	};

	const onDragStart = (e: React.PointerEvent<HTMLElement>, index: number) => {
		e.preventDefault();
		e.currentTarget.setPointerCapture(e.pointerId);
		setDrag({fromIndex: index, toIndex: index});
	};

	const onPointerMove = (e: React.PointerEvent<E>) => {
		if (!drag) return;
		const hoverIndex = getHoverIndex(e.clientY);
		if (hoverIndex !== drag.toIndex) {
			setDrag((prev) => (prev ? {...prev, toIndex: hoverIndex} : null));
		}
	};

	const onPointerUp = () => {
		if (!drag) return;
		if (drag.fromIndex !== drag.toIndex) {
			const next = [...order];
			const [item] = next.splice(drag.fromIndex, 1);
			next.splice(drag.toIndex, 0, item!);
			setOrder(next);
		}
		setDrag(null);
	};

	const draggingItem = drag ? order[drag.fromIndex] ?? null : null;

	return {
		displayOrder,
		listRef,
		draggingItem,
		onDragStart,
		listHandlers: {
			onPointerMove,
			onPointerUp,
			onPointerCancel: onPointerUp,
		},
	};
}
