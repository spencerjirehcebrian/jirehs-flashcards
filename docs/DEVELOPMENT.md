# Development Setup Guide

This guide covers setting up your local development and testing environment for Jireh's Flashcards.

## Prerequisites

- **Rust** (latest stable) - https://rustup.rs
- **Node.js** >= 20 - https://nodejs.org
- **pnpm** >= 9 - `npm install -g pnpm`
- **PostgreSQL** server access
- **Cloudflare R2** bucket access

## Quick Start

### 1. Clone and Install Dependencies

```bash
git clone <repo-url>
cd jirehs-flashcards
pnpm install
```

### 2. Configure Environment

Copy the example environment file and fill in your credentials:

```bash
cp apps/backend/.env.example apps/backend/.env
```

Edit `apps/backend/.env`:

```bash
# Database - your PostgreSQL connection string
DATABASE_URL=postgres://user:password@host:port/flashcards

# S3/R2 Storage
S3_BUCKET=your-bucket-name
S3_REGION=auto
S3_ENDPOINT=https://YOUR_ACCOUNT_ID.r2.cloudflarestorage.com
S3_ACCESS_KEY=your_access_key
S3_SECRET_KEY=your_secret_key

# Server
HOST=0.0.0.0
PORT=3000
```

### 3. Run Setup Script

```bash
./scripts/setup-dev.sh
```

This will:
- Verify required tools are installed
- Install all dependencies
- Run database migrations

### 4. Start Development

**Terminal 1 - Backend:**
```bash
cd apps/backend && cargo run
```

**Terminal 2 - Desktop App:**
```bash
pnpm nx run desktop:dev
```

Or run both with:
```bash
pnpm nx run-many -t dev
```

## Running Tests

Tests require a separate database and R2 bucket to avoid conflicts with development data.

### 1. Create Test Database

On your PostgreSQL server:

```sql
CREATE DATABASE flashcards_test;
```

### 2. Create Test R2 Bucket

In Cloudflare Dashboard:
1. Go to R2
2. Create bucket named `flashcards-test`
3. Use the same API token as your dev bucket

### 3. Configure Test Environment

```bash
cp apps/backend/.env.test.example apps/backend/.env.test
```

Edit `apps/backend/.env.test` with your test database URL and bucket name.

### 4. Run Tests

Run all tests:
```bash
./scripts/run-tests.sh
```

Run backend tests only:
```bash
./scripts/run-tests.sh --backend
```

Run frontend tests only:
```bash
./scripts/run-tests.sh --frontend
```

Or manually:
```bash
# Backend tests
source apps/backend/.env.test && cargo test -p jirehs-flashcards-backend

# Frontend tests (no cloud services needed)
pnpm nx run frontend:test
```

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `DATABASE_URL` | Yes | PostgreSQL connection string |
| `S3_ENDPOINT` | Yes | R2/S3 endpoint URL |
| `S3_BUCKET` | Yes | R2/S3 bucket name |
| `S3_REGION` | Yes | Region (`auto` for R2) |
| `S3_ACCESS_KEY` | Yes | R2/S3 access key |
| `S3_SECRET_KEY` | Yes | R2/S3 secret key |
| `HOST` | No | Server bind address (default: `0.0.0.0`) |
| `PORT` | No | Server port (default: `3000`) |
| `RUST_LOG` | No | Log level: `debug`, `info`, `warn`, `error` |

## Project Commands

| Command | Description |
|---------|-------------|
| `pnpm install` | Install all dependencies |
| `pnpm nx run desktop:dev` | Start Tauri desktop app (includes frontend) |
| `pnpm nx run frontend:dev` | Start frontend dev server only |
| `pnpm nx run frontend:test` | Run frontend tests |
| `pnpm nx run frontend:build` | Build frontend for production |
| `cargo run -p jirehs-flashcards-backend` | Start backend server |
| `cargo test -p jirehs-flashcards-backend` | Run backend tests |
| `cargo build -p jirehs-flashcards-backend --release` | Build backend for production |
| `./scripts/setup-dev.sh` | Initial development setup |
| `./scripts/run-tests.sh` | Run all tests |

## Project Structure

```
jirehs-flashcards/
├── apps/
│   ├── backend/          # Rust backend (Axum + PostgreSQL)
│   │   ├── src/
│   │   ├── migrations/   # SQLx migrations
│   │   ├── tests/        # Integration tests
│   │   └── .env          # Environment config (gitignored)
│   │
│   ├── frontend/         # React frontend (Vite + TanStack Query)
│   │   └── src/
│   │
│   └── desktop/          # Tauri desktop app
│       └── src-tauri/    # Rust Tauri code
│
├── libs/
│   ├── flashcard-core/   # Shared Rust library (parser, algorithms)
│   └── shared-types/     # Shared TypeScript types
│
├── scripts/              # Development scripts
├── docs/                 # Documentation
└── spec.md               # Technical specification
```

## Troubleshooting

### Database Connection Failed

- Verify `DATABASE_URL` is correct
- Check PostgreSQL server is running and accessible
- Ensure the database exists

### R2/S3 Connection Failed

- Verify all `S3_*` environment variables are set
- Check your R2 API token has read/write permissions
- Ensure the bucket exists

### Tauri Build Issues

- Ensure Rust toolchain is up to date: `rustup update`
- Check Tauri prerequisites: https://tauri.app/v2/guides/prerequisites

### Frontend Dev Server Port Conflict

- The frontend dev server uses port 5173
- If in use, modify `apps/frontend/vite.config.ts`

### Tests Fail with Database Errors

- Ensure test database exists and is separate from dev
- Check `apps/backend/.env.test` has correct credentials
- Tests use device-based cleanup; old test data should not interfere
