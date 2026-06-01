# EVIF Development Commands

# Default: start all services
dev:
    @echo "Starting EVIF services..."
    @cd evif-web && npm run dev &
    @cargo run --package evif-rest &
    @sleep 5
    @echo ""
    @echo "🎉 EVIF Services:"
    @echo "  Frontend: http://localhost:3000"
    @echo "  Backend:  http://localhost:8081"

# Start frontend only
frontend:
    cd evif-web && npm run dev

# Start backend only  
backend:
    cargo run --package evif-rest

# Stop all services
stop:
    pkill -f "evif-rest" || true
    lsof -ti:3000 | xargs kill 2>/dev/null || true
    @echo "Services stopped"

# Type check
check:
    cd evif-web && npx tsc --noEmit
    cargo check --workspace

# Build production
build:
    cd evif-web && npm run build
    cargo build --release

# Run tests
test:
    cd evif-web && npm run test
    cargo test --workspace
