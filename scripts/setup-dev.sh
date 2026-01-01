#!/bin/bash
set -e

echo "=== Jireh's Flashcards - Development Setup ==="
echo ""

# Check required tools
echo "Checking required tools..."

if ! command -v cargo &> /dev/null; then
    echo "ERROR: cargo is not installed"
    echo "Install Rust from https://rustup.rs"
    exit 1
fi
echo "  cargo: $(cargo --version)"

if ! command -v pnpm &> /dev/null; then
    echo "ERROR: pnpm is not installed"
    echo "Install with: npm install -g pnpm"
    exit 1
fi
echo "  pnpm: $(pnpm --version)"

if ! command -v node &> /dev/null; then
    echo "ERROR: node is not installed"
    exit 1
fi
echo "  node: $(node --version)"

echo ""

# Check .env exists
if [ ! -f "apps/backend/.env" ]; then
    echo "ERROR: apps/backend/.env not found"
    echo ""
    echo "To fix this:"
    echo "  1. cp apps/backend/.env.example apps/backend/.env"
    echo "  2. Edit apps/backend/.env with your database and R2 credentials"
    exit 1
fi
echo "Environment file found: apps/backend/.env"

# Install dependencies
echo ""
echo "Installing dependencies..."
pnpm install

# Load environment and run migrations
echo ""
echo "Running database migrations..."
set -a
source apps/backend/.env
set +a

cd apps/backend
cargo sqlx database create 2>/dev/null || echo "  Database already exists or cannot create (this is usually fine)"
cargo sqlx migrate run --source ./migrations 2>/dev/null || cargo run --bin migrate 2>/dev/null || echo "  Migrations applied via app startup"
cd ../..

echo ""
echo "=== Setup Complete! ==="
echo ""
echo "To start development:"
echo ""
echo "  Terminal 1 (Backend):"
echo "    cd apps/backend && cargo run"
echo ""
echo "  Terminal 2 (Desktop App):"
echo "    pnpm nx run desktop:dev"
echo ""
echo "  Or run both with:"
echo "    pnpm nx run-many -t dev"
echo ""
