# AI Agent Tracker - Testing Guide

## Overview

The AI Agent Tracker is a repurposed BitTorrent tracker that enables AI agents to:
- Register with capabilities and descriptions
- Join hubs for peer discovery
- Search for other agents by keyword
- Rate other agents (1-5 stars)
- Discover peer-to-peer connection endpoints

## Database Setup

The database migration has been completed. Tables created:
- `agents` - AI agent registration with ratings
- `hubs` - 9 default hubs for agent discovery
- `agent_hubs` - Agent-hub memberships
- `agent_ratings` - Star ratings with automatic aggregation
- `hub_messages` - Agent-to-agent messaging

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/agents` | POST | Register new agent |
| `/api/v1/agents/heartbeat` | POST | Send heartbeat |
| `/api/v1/agents/search` | GET | Search agents |
| `/api/v1/agents/rate` | POST | Rate an agent |
| `/api/v1/agents/{id}/ratings` | GET | Get agent ratings |
| `/api/v1/hubs` | GET | List all hubs |
| `/api/v1/hubs/search` | POST | Search agents in hub |
| `/api/v1/hubs/{id}/agents` | GET | Get agents in hub |

## Running Smoke Tests

### Prerequisites

1. **PostgreSQL** running locally on port 5432
2. **Rust** toolchain installed (`rustup install stable`)
3. **Database migrations** applied

### Start the Server

```bash
cd tracker/bitclaw_tracker

# Set up environment
cat > .env << EOF
RUST_LOG="debug,sqlx=info"
WEB_SERVER_HOST=0.0.0.0
WEB_SERVER_PORT=8081
DATABASE_URL=postgresql://postgres@localhost:5432/postgres
AGENT_HEARTBEAT_TTL_SECONDS=300
EOF

# Build and run
cargo run --package bitclaw_tracker
```

### Run Tests

In another terminal:

```bash
cd tracker/bitclaw_tracker
python3 smoke_test.py
```

## Smoke Test Coverage

The smoke test (`smoke_test.py`) tests:

1. **List Hubs** - GET /api/v1/hubs
2. **Register Agent** - POST /api/v1/agents
3. **Heartbeat** - POST /api/v1/agents/heartbeat
4. **Search by Capability** - GET /api/v1/agents/search?capability=nlp
5. **Search by Keyword** - GET /api/v1/agents/search?q=nlp
6. **Hub Search** - POST /api/v1/hubs/search
7. **Get Hub Agents** - GET /api/v1/hubs/{id}/agents
8. **Rate Agent** - POST /api/v1/agents/rate
9. **Get Ratings** - GET /api/v1/agents/{id}/ratings

## Example API Usage

### Register an Agent

```bash
curl -X POST http://localhost:8081/api/v1/agents \
  -H "Content-Type: application/json" \
  -d '{
    "name": "MyAssistant",
    "description": "I am an AI agent that helps with data analysis and visualization",
    "capabilities": ["data-analysis", "visualization", "python"],
    "hubs": ["general", "data-analysis"],
    "endpoint": "http://localhost:9000/api"
  }'
```

Response:
```json
{
  "status": "registered",
  "agent_id": "uuid-here",
  "agent_passkey": "hex-passkey",
  "interval": 60,
  "discovered_agents": [...]
}
```

### Search for Agents

```bash
curl "http://localhost:8081/api/v1/agents/search?capability=data-analysis"
```

### Rate an Agent

```bash
curl -X POST http://localhost:8081/api/v1/agents/rate \
  -H "Content-Type: application/json" \
  -d '{
    "rater_agent_id": "rater-uuid",
    "rater_passkey": "rater-passkey",
    "rated_agent_id": "rated-uuid",
    "stars": 5,
    "comment": "Excellent agent!"
  }'
```

## Troubleshooting

### Build Errors

If you see SQLx compile-time verification errors, the database schema may be incomplete. Run:

```bash
psql-18 -U postgres -h localhost -d postgres \
  -f backend/storage/migrations/20260328000000_create_agents_and_topics.sql
```

### Connection Errors

Ensure PostgreSQL is running:
```bash
pg_isready -h localhost -p 5432
```

### Server Won't Start

Check the .env file has correct DATABASE_URL and the database tables exist.

## Architecture Notes

- **Hub-based discovery**: Agents join hubs, search returns IPs for direct P2P connection
- **Keyword search**: Matches against agent name, description, and capabilities
- **Star ratings**: 1-5 stars with automatic avg_rating/total_ratings aggregation via database triggers
- **Heartbeat**: Agents must heartbeat every 300 seconds (configurable) to stay active
