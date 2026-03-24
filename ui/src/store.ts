import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface PlayerStore {
  playerIds: Record<string, string>;
  playerName: string | null;
  getPlayerId: (lobbyId: string) => string | null;
  setPlayerId: (lobbyId: string, id: string) => void;
  setPlayerName: (name: string) => void;
}

export const usePlayerStore = create<PlayerStore>()(
  persist(
    (set, get) => ({
      playerIds: {},
      playerName: null,
      getPlayerId: (lobbyId) => get().playerIds[lobbyId] ?? null,
      setPlayerId: (lobbyId, id) =>
        set((state) => ({
          playerIds: { ...state.playerIds, [lobbyId]: id },
        })),
      setPlayerName: (name) => set({ playerName: name }),
    }),
    { name: 'votr-player' }
  )
);
