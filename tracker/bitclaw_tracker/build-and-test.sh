#!/bin/bash
# AI Agent Tracker - Build and Test Script
# This script sets up the database, builds the server, and runs smoke tests

set -e

echo "======================================"
echo "AI Agent Tracker - Build and Test"
echo "======================================"

# Configuration
DB_USER="${DB_USER:-postgres}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-postgres}"
SERVER_PORT="${SERVER_PORT:-8081}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Step 1: Check if PostgreSQL is running
log_info "Checking PostgreSQL connection..."
if ! psql-18 -U "$DB_USER" -h "$DB_HOST" -d "$DB_NAME" -c "SELECT 1" > /dev/null 2>&1; then
    log_error "Cannot connect to PostgreSQL. Please ensure it's running."
    exit 1
fi
log_info "PostgreSQL is running."

# Step 2: Run database migrations
log_info "Running database migrations..."
MIGRATION_FILE="$(pwd)/backend/storage/migrations/20260328000000_create_agents_and_topics.sql"
if [ -f "$MIGRATION_FILE" ]; then
    psql-18 -U "$DB_USER" -h "$DB_HOST" -d "$DB_NAME" -f "$MIGRATION_FILE" > /dev/null 2>&1 || true
    log_info "Database migrations completed."
else
    log_warn "Migration file not found: $MIGRATION_FILE"
fi

# Step 3: Create .env file if not exists
ENV_FILE="$(pwd)/tracker/arcadia_tracker/.env"
if [ ! -f "$ENV_FILE" ]; then
    log_info "Creating .env file..."
    cat > "$ENV_FILE" << EOF
RUST_LOG="debug,sqlx=info"
WEB_SERVER_HOST=0.0.0.0
WEB_SERVER_PORT=$SERVER_PORT
API_KEY=change_me
ALLOWED_TORRENT_CLIENTS=BI3500,BI3600
NUMWANT_DEFAULT=15
NUMWANT_MAX=15
ANNOUNCE_MIN=1800
ANNOUNCE_MIN_ENFORCED=1740
ANNOUNCE_MAX=3600
MAX_PEERS_PER_TORRENT_PER_USER=3
FLUSH_INTERVAL_MILLISECONDS=3000
PEER_EXPIRY_INTERVAL=1800
ACTIVE_PEER_TTL=7200
INACTIVE_PEER_TTL=1814400
AGENT_HEARTBEAT_TTL_SECONDS=300
ARCADIA_API_BASE_URL=http://localhost:8080
DATABASE_URL=postgresql://$DB_USER@$DB_HOST:$DB_PORT/$DB_NAME
EOF
    log_info ".env file created."
fi

# Step 4: Build the tracker
log_info "Building arcadia_tracker..."
export PATH="$HOME/.cargo/bin:$PATH"
export DATABASE_URL="postgresql://$DB_USER@$DB_HOST:$DB_PORT/$DB_NAME"

# Note: Build may fail due to SQLx offline mode requiring all tables
# If you encounter build errors, you may need to:
# 1. Create missing tables for the torrent tracker
# 2. Or set SQLX_OFFLINE=true and use cached queries

cd "$(pwd)/tracker/arcadia_tracker"

# Try to build
if cargo build --release 2>&1; then
    log_info "Build successful!"
else
    log_error "Build failed. See errors above."
    log_warn "The build may fail due to missing torrent tracker tables."
    log_warn "To fix this, you need to create the full schema or use Docker."
    exit 1
fi

# Step 5: Start the server in background
log_info "Starting server on port $SERVER_PORT..."
./target/release/arcadia_tracker &
SERVER_PID=$!
sleep 3

# Check if server is running
if ! kill -0 $SERVER_PID 2>/dev/null; then
    log_error "Server failed to start."
    exit 1
fi
log_info "Server started with PID $SERVER_PID"

# Step 6: Run smoke tests
log_info "Running smoke tests..."
cd "$(pwd)/tracker/arcadia_tracker"
python3 smoke_test.py
TEST_RESULT=$?

# Step 7: Cleanup
log_info "Stopping server..."
kill $SERVER_PID 2>/dev/null || true

if [ $TEST_RESULT -eq 0 ]; then
    log_info "All tests passed!"
else
    log_error "Some tests failed."
fi

exit $TEST_RESULT
