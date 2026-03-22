# 文字 (Moji)

A real-time multiplayer kanji and vocabulary game built entirely in Rust. Players get a kanji character (or vocab word) and race to type valid Japanese words containing it. Inspired by Bomb Party on [jklm.fun](https://jklm.fun).

**Live at [moji.fly.dev](https://moji.fly.dev)**

## What It Does

You join a lobby, the game shows you a kanji like **日**, and you type a word that uses it — like **日本** (にほん). If the word exists in the dictionary and contains the right kanji, you score. First to target score (or last man standing) wins.

There are three game modes:
- **Deathmatch** — Race to a target score. Fastest fingers win.
- **Duel** — Turn-based with lives. Miss a word and you lose a life. Last one standing wins.
- **Zen** — No pressure, no scores. Just practice at your own pace.

It also supports a **Vocab** content mode where instead of finding words with a kanji, you're given a word and need to type its correct hiragana reading.

Difficulty is configurable across all five JLPT levels (N5–N1), and the lobby leader can mix and match levels, set time limits, and toggle weighted kanji distribution based on frequency.

## Tech Stack

This is a Rust monorepo with four crates:

| Crate | What it does |
|-------|-------------|
| `backend` | Axum HTTP server + WebSocket handler, PostgreSQL via SQLx, JWT auth |
| `frontend` | Leptos SPA compiled to WebAssembly, runs entirely in the browser |
| `shared` | Types, messages, and server function definitions shared across both sides |
| `macros` | Procedural macros for reducing boilerplate |

Some other notable pieces:
- **WebSockets** for all real-time game communication (typing indicators, score updates, prompt changes)
- **JWT authentication** with support for both registered accounts and anonymous guest sessions
- **Argon2** password hashing
- **Rate limiting** via `tower-governor` to prevent WebSocket spam
- **Profanity filtering** on usernames via `rustrict`
- **SQLx compile-time checked queries** with offline mode for Docker builds
- **Telemetry tracking** — the backend silently tracks games played, words guessed, and online presence for analytics

## Project Structure

```
moji/
├── backend/
│   ├── src/
│   │   ├── main.rs          # Server entrypoint, routing, middleware
│   │   ├── api.rs           # HTTP + WebSocket handlers
│   │   ├── lobby.rs         # Game state machine (scoring, turns, prompts)
│   │   ├── state.rs         # Shared application state
│   │   ├── models/          # SQLx database models (users, sessions, stats)
│   │   ├── data.rs          # CSV data loading (kanji lists, word lists)
│   │   └── error.rs         # Error types
│   └── migrations/          # PostgreSQL schema migrations
├── frontend/
│   ├── src/
│   │   ├── main.rs          # App shell, routing, theme init
│   │   ├── components/      # Leptos components (home, lobby, game, etc.)
│   │   ├── context.rs       # Reactive contexts (auth, game state)
│   │   └── persistence.rs   # LocalStorage session management
│   ├── index.html           # Trunk entrypoint
│   └── input.css            # Tailwind v4 styles
├── shared/                  # Cross-crate types and server functions
├── macros/                  # Proc macros
├── data/                    # JLPT kanji + vocabulary CSVs (N1–N5)
├── Dockerfile               # Multi-stage build (cargo-chef + Trunk + Bun)
```

## Running Locally

### Prerequisites
- Rust (stable)
- PostgreSQL running locally
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- [Trunk](https://trunkrs.dev): `cargo install trunk`
- [Bun](https://bun.sh) (for Tailwind CSS processing)
- SQLx CLI (optional, for migrations): `cargo install sqlx-cli`

### Setup

1. **Clone and enter the repo:**
   ```bash
   git clone https://github.com/CldStlkr/moji.git
   cd moji
   ```

2. **Create a PostgreSQL database and set the connection URL:**
   ```bash
   createdb moji
   export DATABASE_URL=postgres://localhost/moji
   ```

3. **Run migrations:**
   ```bash
   sqlx migrate run --source backend/migrations
   ```

4. **Start the backend** (from the repo root):
   ```bash
   cargo run --bin moji-server
   ```

5. **In another terminal, build and serve the frontend:**
   ```bash
   cd frontend
   bunx tailwindcss -i ./input.css -o ./styles.css
   trunk serve
   ```

6. **Open `http://localhost:8080`** — create a guest account and start a lobby.

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | *(required)* | PostgreSQL connection string |
| `JWT_SECRET` | `INSECURE_DEFAULT_SECRET` | Secret for signing auth tokens |
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `8080` | Server port |
| `PRODUCTION` | *(unset)* | Set to `1` for production mode (stricter CORS, different asset paths) |
| `FRONTEND_URL` | `https://moji.fly.dev` | Allowed origin in production CORS |

## Deployment

The project deploys to [Fly.io](https://fly.io) with a single command:

```bash
fly deploy
```

The Dockerfile uses a multi-stage build:
1. **cargo-chef** for dependency caching
2. **Backend build** with `SQLX_OFFLINE=true` (requires running `cargo sqlx prepare --workspace` locally first)
3. **Frontend build** with Trunk + Bun (Tailwind v4)
4. **Final image** based on `debian:bookworm-slim` (~minimal footprint)

If you modify any SQLx queries, you need to regenerate the offline cache before deploying:
```bash
cargo sqlx prepare --workspace
git add .sqlx/ && git commit -m "update sqlx cache"
```

## Data Sources

The kanji and vocabulary data comes from:
- [Jōyō Kanji List](https://www.bunka.go.jp/kokugo_nihongo/sisaku/joho/joho/kijun/naikaku/kanji/) — the official list of kanji for everyday use
- [JMdict](https://www.edrdg.org/jmdict/j_jmdict.html) — comprehensive Japanese-English dictionary project

All data is loaded from CSV files in the `data/` directory at server startup.

## License

MIT
