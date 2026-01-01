#!/bin/bash
set -e

echo "=== Jireh's Flashcards - Test Runner ==="
echo ""

# Parse arguments
BACKEND_ONLY=false
FRONTEND_ONLY=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --backend)
            BACKEND_ONLY=true
            shift
            ;;
        --frontend)
            FRONTEND_ONLY=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: ./scripts/run-tests.sh [--backend] [--frontend]"
            exit 1
            ;;
    esac
done

# Run backend tests
if [ "$FRONTEND_ONLY" = false ]; then
    echo "=== Backend Tests ==="
    echo ""

    # Check test env exists
    if [ ! -f "apps/backend/.env.test" ]; then
        echo "ERROR: apps/backend/.env.test not found"
        echo ""
        echo "To fix this:"
        echo "  1. Create test database: CREATE DATABASE flashcards_test;"
        echo "  2. Create test R2 bucket: flashcards-test"
        echo "  3. cp apps/backend/.env.test.example apps/backend/.env.test"
        echo "  4. Edit apps/backend/.env.test with test credentials"
        exit 1
    fi

    # Load test environment
    set -a
    source apps/backend/.env.test
    set +a

    echo "Running backend tests with test environment..."
    echo "  DATABASE: ${DATABASE_URL%%@*}@..."
    echo "  S3_BUCKET: $S3_BUCKET"
    echo ""

    cargo test -p jirehs-flashcards-backend

    echo ""
    echo "Backend tests passed!"
    echo ""
fi

# Run frontend tests
if [ "$BACKEND_ONLY" = false ]; then
    echo "=== Frontend Tests ==="
    echo ""

    pnpm nx run frontend:test

    echo ""
    echo "Frontend tests passed!"
    echo ""
fi

echo "=== All Tests Passed! ==="
