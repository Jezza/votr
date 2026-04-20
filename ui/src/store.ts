import {createStore,} from 'zustand';
import {
	persist,
	createJSONStorage
} from 'zustand/middleware';
import React, {useContext} from "react";
import {shallow} from "zustand/vanilla/shallow";
import {useStoreWithEqualityFn} from "zustand/traditional";
import type {StateCreator} from "zustand/vanilla";
import {generateSeriousName} from "./names.ts";

export type AppApi = PlayerApi;

export type PlayerApi = {
	playerId: string,
	playerName: string;

	setPlayerName: (name: string) => void;
};

export type AppStore = ReturnType<typeof createAppStore>;

export const APP_CONTEXT: React.Context<AppStore> = React.createContext(null as unknown as AppStore);

function useAppStore(): AppStore {
	const state = useContext(APP_CONTEXT);
	if (!state) {
		throw new Error("State must be used from within a STATE_CONTEXT.");
	}
	return state;
}

export function useApp<StateSlice = AppApi>(
	selector: (state: AppApi) => StateSlice,
	equalityFn?: (a: StateSlice, b: StateSlice) => boolean,
) {
	const store = useAppStore();
	const eqFn = equalityFn ?? shallow;
	return useStoreWithEqualityFn(store, selector, eqFn);
}

// Returns a non-reactive reference to the store for reading fresh state
// outside of React's render cycle (e.g. in tree data provider callbacks).
export function useAppRef(): () => AppApi {
	const store = useAppStore();
	return store.getState;
}

export function createAppStore() {
	const store: StateCreator<AppApi> = (set, get, s) => {
		return ({
			playerId: crypto.randomUUID(),
			playerName: generateSeriousName(),
			setPlayerName: (name) => set({playerName: name}),
		});
	};

	return createStore(persist(store, {
		name: 'votr',
		version: 1,
		storage: createJSONStorage(() => localStorage),
	}))
}

