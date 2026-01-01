# Jireh's Flashcards

A spaced repetition flashcard application with desktop sync and cloud backup.

## Tech Stack

| Layer | Technology |
|-------|------------|
| Frontend | React 18, TypeScript, Vite, Zustand, TanStack Query |
| Desktop | Tauri 2, SQLite |
| Backend | Rust, Axum, PostgreSQL, S3/R2 |
| Core | Rust library with SM-2 and FSRS algorithms |

## Project Structure

```
apps/
  frontend/     # React web app
  desktop/      # Tauri desktop wrapper
  backend/      # Rust API server
libs/
  flashcard-core/   # Spaced repetition algorithms
  shared-types/     # TypeScript type definitions
```

## Prerequisites

- Node.js >= 20.0.0
- pnpm 9.x (`npm install -g pnpm`)
- Rust toolchain (for backend/desktop work)
- PostgreSQL (for backend)

## Quick Start

```bash
# Install dependencies
pnpm install

# Set up environment files
cp apps/backend/.env.example apps/backend/.env
cp apps/frontend/.env.example apps/frontend/.env

# Start all dev servers
pnpm dev
```

Frontend runs at `http://localhost:5173`

## Environment Variables

### Backend (`apps/backend/.env`)

```env
DATABASE_URL=postgres://user:password@localhost:5432/flashcards
S3_BUCKET=flashcards-md
S3_REGION=auto
S3_ENDPOINT=https://xxx.r2.cloudflarestorage.com
S3_ACCESS_KEY=your_access_key
S3_SECRET_KEY=your_secret_key
HOST=0.0.0.0
PORT=3000
```

### Frontend (`apps/frontend/.env`)

```env
VITE_API_URL=http://localhost:3000
```

## Commands

```bash
# Development
pnpm dev                # Run all apps
pnpm frontend:dev       # Frontend only (port 5173)
pnpm desktop:dev        # Desktop app only

# Testing
pnpm test               # All tests
pnpm frontend:test      # Frontend tests
pnpm frontend:test:coverage  # With coverage

# Build
pnpm build              # Build all
pnpm frontend:build     # Frontend production build
pnpm desktop:build      # Desktop executable
```

### Backend (Rust)

```bash
cd apps/backend
cargo run               # Dev server
cargo test              # Run tests
cargo build --release   # Production build
```

## Architecture

```
Desktop (Tauri)              Cloud (Rust API)
+-----------------+          +------------------+
| Markdown Files  |          | PostgreSQL       |
| SQLite (local)  |  <--->   | S3/R2 Storage    |
| File Watcher    |  sync    | Review History   |
+-----------------+          +------------------+
```

- Local markdown files are the source of truth for card content
- SQLite stores learning state offline
- Cloud syncs cards, reviews, and settings when online

## Key Directories

| Path | Description |
|------|-------------|
| `apps/frontend/src/components/` | React components |
| `apps/frontend/src/stores/` | Zustand state stores |
| `apps/frontend/src/pages/` | Route pages |
| `apps/backend/src/routes/` | API endpoints |
| `apps/backend/src/services/` | Business logic |
| `apps/backend/migrations/` | Database migrations |
| `libs/flashcard-core/src/` | Core algorithms |

## Testing

Frontend uses Vitest with 80% coverage threshold:

```bash
pnpm frontend:test           # Run tests
pnpm frontend:test:ui        # Interactive UI
pnpm frontend:test:coverage  # Coverage report
```

Test utilities in `apps/frontend/src/test/`:
- `setup.ts` - Test environment config
- `factories.ts` - Test data factories
- `mocks/` - API and Tauri mocks

## Database

Migrations run automatically on backend startup. Schema located in `apps/backend/migrations/001_initial_schema.sql`.

Tables: `devices`, `cards`, `card_states`, `reviews`, `deck_settings`

## Documentation

- `spec.md` - Technical specification
- `progress.md` - Implementation status
