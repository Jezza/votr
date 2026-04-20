import type { PhaseProps } from "../types";

function playerName(players: PhaseProps["state"]["players"], id: string): string {
  return players.find((p) => p.id === id)?.name ?? "Unknown";
}

export function VetoingPhase({ state, myId, isHost, send, myPlayer, getCountdown }: PhaseProps) {
  const readyCount = state.players.filter((p) => p.ready).length;
  const totalCount = state.players.length;
  const amReady = myPlayer?.ready ?? false;
  const allVetoed = state.games.length > 0 && state.games.every((g) => g.vetoed_by.length > 0);

  const handleToggleReady = () => {
    send({ ty: "set_ready", ready: !amReady });
  };

  return (
    <>
      <section className="card">
        <h2 className="section-title">Veto Options</h2>
        <p className="hint-text">
          Veto any options you don't want. A single veto removes an option from
          voting.
        </p>
      </section>

      <section className="card">
        <ul className="game-list">
          {state.games.map((game) => {
            const iVetoed = myId !== null && game.vetoed_by.includes(myId);
            const vetoCount = game.vetoed_by.length;
            const hasVetoes = vetoCount > 0;

            return (
              <li
                key={game.id}
                className={`game-item ${hasVetoes ? "game-item--vetoed" : ""}`}
              >
                <div className="game-item-info">
                  <span className="game-item-name">{game.name}</span>
                  <span className="game-item-meta">
                    by {playerName(state.players, game.suggested_by)}
                  </span>
                  {hasVetoes && (
                    <span className="veto-count-badge">
                      vetoed by {game.vetoed_by.map((id) => playerName(state.players, id)).join(", ")}
                    </span>
                  )}
                </div>
                <button
                  className={`btn btn-icon ${iVetoed ? "btn-danger btn-active" : "btn-outline"}`}
                  onClick={() =>
                    send(
                      iVetoed
                        ? { ty: "unveto_game", game_id: game.id }
                        : { ty: "veto_game", game_id: game.id }
                    )
                  }
                  title={iVetoed ? "Remove veto" : "Veto this option"}
                  aria-label={iVetoed ? `Remove veto from ${game.name}` : `Veto ${game.name}`}

                >
                  {iVetoed ? "✓ Vetoed" : "Veto"}
                </button>
              </li>
            );
          })}
        </ul>
      </section>

      <section className="card">
        <button
          className={`btn btn-full btn-large ${amReady ? "btn-success btn-active" : "btn-outline"}`}
          onClick={handleToggleReady}
        >
          {amReady ? "✓ Done Vetoing" : "Done Vetoing"}
        </button>
      </section>

      <section className="card">
        <h2 className="section-title">Player Status</h2>
        <ul className="player-list">
          {state.players.map((player) => (
            <li key={player.id} className="player-item">
              <span
                className={`ready-dot ${player.ready ? "ready-dot--on" : "ready-dot--off"}`}
              >
                {player.ready ? "✓" : "○"}
              </span>
              <span className="player-name">
                {player.name}
                {player.id === myId && <span className="you-label"> (you)</span>}
                {player.disconnect_timeout != null && (
                  <span className="disconnect-timer"> ({getCountdown(player.id, player.disconnect_timeout)}s)</span>
                )}
              </span>
            </li>
          ))}
        </ul>
      </section>

      {allVetoed && (
        <section className="card">
          <p className="hint-text" style={{ color: "#c45050" }}>
            All options have been vetoed! Remove some vetoes or ask the host to reset the session.
          </p>
        </section>
      )}

      {isHost && (
        <section className="card">
          <button
            className="btn btn-primary btn-full btn-large"
            onClick={() => send({ ty: "advance_phase" })}
            disabled={allVetoed}
          >
            {allVetoed
              ? "Can't advance — all options vetoed"
              : `Advance to Voting (${readyCount}/${totalCount} ready)`}
          </button>
        </section>
      )}
    </>
  );
}
