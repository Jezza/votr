# Votr

A real-time voting app. Create a lobby, add options, veto, rank, and see results — all live with friends.

## Stack

- **Backend:** Rust (Axum, Tokio, WebSocket)
- **Frontend:** React 19, TypeScript, Zustand, Bun

## Running

```bash
# Install frontend dependencies
cd ui && bun install && cd ..

# Build frontend
cd ui && bun run b && cd ..

# Run server
cargo run
```

Open `http://localhost:3001`.

## How it works

1. Create or join a lobby
2. Add options to vote on
3. Veto any options you don't want
4. Rank the remaining options
5. See the results
