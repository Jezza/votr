import { useCallback, useEffect, useState } from "react";
import type { LobbyInfo } from "./types";
import { GitHubLink } from "./GitHubLink";
import { apiBase } from "./api";

const PHASE_LABELS: Record<string, string> = {
  lobby: "Lobby",
  adding: "Adding",
  vetoing: "Vetoing",
  voting: "Voting",
  results: "Results",
};

interface LobbyBrowserProps {
  onJoin: (lobbyId: string, password?: string) => void;
}

export function LobbyBrowser({ onJoin }: LobbyBrowserProps) {
  const [lobbies, setLobbies] = useState<LobbyInfo[]>([]);
  const [creating, setCreating] = useState(false);
  const [createPublic, setCreatePublic] = useState(true);
  const [createPassword, setCreatePassword] = useState("");
  const [passwordPrompt, setPasswordPrompt] = useState<string | null>(null);
  const [passwordInput, setPasswordInput] = useState("");

  const fetchLobbies = useCallback(async () => {
    try {
      const res = await fetch(`${apiBase}/api/lobbies`);
      if (res.ok) {
        setLobbies(await res.json());
      }
    } catch {}
  }, []);

  useEffect(() => {
    fetchLobbies();
    const interval = setInterval(fetchLobbies, 5000);
    return () => clearInterval(interval);
  }, [fetchLobbies]);

  const handleCreate = async () => {
    try {
      const res = await fetch(`${apiBase}/api/lobbies`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          public: createPublic,
          password: createPassword || null,
        }),
      });
      if (res.ok) {
        const data = await res.json();
        onJoin(data.id, createPassword || undefined);
      }
    } catch {}
  };

  const handleJoin = (lobby: LobbyInfo) => {
    if (lobby.has_password) {
      setPasswordPrompt(lobby.id);
      setPasswordInput("");
    } else {
      onJoin(lobby.id);
    }
  };

  const handlePasswordSubmit = () => {
    if (passwordPrompt) {
      onJoin(passwordPrompt, passwordInput);
      setPasswordPrompt(null);
    }
  };

  return (
    <div className="app">
      <main className="content-area">
        <div className="content-inner">
          <div className="connecting-screen" style={{ justifyContent: "flex-start", paddingTop: "60px" }}>
            <div className="connecting-logo">Votr</div>
            <GitHubLink size={22} className="github-link--hero" />
          </div>

          {passwordPrompt && (
            <section className="card">
              <h2 className="section-title">Enter Password</h2>
              <div className="lobby-create-form">
                <input
                  className="lobby-password-input"
                  type="password"
                  value={passwordInput}
                  onChange={(e) => setPasswordInput(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && handlePasswordSubmit()}
                  placeholder="Lobby password"
                  autoFocus
                  maxLength={64}
                />
                <div className="lobby-create-actions">
                  <button className="btn btn-outline" onClick={() => setPasswordPrompt(null)}>
                    Cancel
                  </button>
                  <button className="btn btn-primary" onClick={handlePasswordSubmit}>
                    Join
                  </button>
                </div>
              </div>
            </section>
          )}

          <section className="card">
            <h2 className="section-title">
              Lobbies {lobbies.length > 0 && <span className="count-badge">{lobbies.length}</span>}
            </h2>
            {lobbies.length === 0 ? (
              <p className="hint-text">No public lobbies available. Create one!</p>
            ) : (
              <ul className="lobby-list">
                {lobbies.map((lobby) => (
                  <li key={lobby.id} className="lobby-item">
                    <div className="lobby-item-info">
                      <span className="lobby-item-name">
                        {lobby.name}
                        {lobby.has_password && <span className="lobby-lock-icon" title="Password protected"> 🔒</span>}
                        {lobby.locked && <span className="lobby-locked-badge">Locked</span>}
                      </span>
                      <span className="lobby-item-meta">
                        {lobby.player_count}/{lobby.max_players} players
                        <span className="phase-badge phase-badge--small">{PHASE_LABELS[lobby.phase] ?? lobby.phase}</span>
                      </span>
                    </div>
                    <button
                      className="btn btn-primary"
                      onClick={() => handleJoin(lobby)}
                      disabled={lobby.locked}
                    >
                      Join
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </section>

          {creating ? (
            <section className="card">
              <h2 className="section-title">Create Lobby</h2>
              <div className="lobby-create-form">
                <label className="lobby-toggle">
                  <span>Public</span>
                  <input
                    type="checkbox"
                    checked={createPublic}
                    onChange={(e) => setCreatePublic(e.target.checked)}
                  />
                </label>
                <input
                  className="lobby-password-input"
                  type="password"
                  value={createPassword}
                  onChange={(e) => setCreatePassword(e.target.value)}
                  placeholder="Password (optional)"
                  maxLength={64}
                />
                <div className="lobby-create-actions">
                  <button className="btn btn-outline" onClick={() => setCreating(false)}>
                    Cancel
                  </button>
                  <button className="btn btn-primary" onClick={handleCreate}>
                    Create
                  </button>
                </div>
              </div>
            </section>
          ) : (
            <section className="card">
              <button
                className="btn btn-primary btn-full btn-large"
                onClick={() => setCreating(true)}
              >
                Create Lobby
              </button>
            </section>
          )}
        </div>
      </main>
    </div>
  );
}
