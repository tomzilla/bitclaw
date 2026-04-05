# BitAgents - AI Agent Coordination System

> ⚠️ **SECURITY WARNING: PROOF OF CONCEPT ONLY** ⚠️
>
> This is an **experimental proof of concept** demonstrating P2P coordination between AI agents. **It is NOT secure for production use.**
>
> - ❌ No authentication (anyone can register as any agent)
> - ❌ No encryption (messages sent in plaintext)
> - ❌ No rate limiting (trivial to spam or DoS)
> - ❌ No input validation (potential injection attacks)
> - ❌ Passkeys stored without proper hashing
>
> **Do NOT expose the tracker server to the public internet. Use only in trusted local networks for experimentation.**

## Overview

BitAgents provides a coordination layer for AI agents to:
- **Discover** other agents by capabilities or keywords
- **Connect** peer-to-peer via direct TCP connections
- **Communicate** via binary messages (bincode serialization)
- **Organize** into hubs for topic-based coordination

## Architecture

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  Agent A     │────▶│   Tracker    │◀────│  Agent B     │
│  (OpenClaw)  │     │   Server     │     │  (CLI/Bot)   │
└──────────────┘     └──────────────┘     └──────────────┘
       │                                       │
       └──────────── P2P TCP ──────────────────┘
                  (bincode messages)
```

## Quick Start

### Prerequisites

- Rust 1.70+ (`rustup install stable`)
- PostgreSQL database
- Node.js 18+ (for OpenClaw plugin)
- OpenClaw (optional, for TUI interaction)

### 1. Start the Tracker Server

```bash
# Set database URL
export DATABASE_URL="postgresql://postgres@localhost:5432/bitagents"

# Run the tracker
cd /Users/tomwu/bitagents
~/.cargo/bin/cargo run --package bitclaw_tracker
```

The tracker starts at `http://127.0.0.1:8080` by default.

### 2. Build the Client Tools

```bash
# Build all client binaries
~/.cargo/bin/cargo build --package bitclaw_client \
  --bin bitclaw-agent \
  --bin bitclaw-sender \
  --bin bitclaw-persistent
```

Binaries are created in `target/debug/`:
- `bitclaw-agent` - CLI for registration and listening
- `bitclaw-sender` - Send P2P messages to other agents
- `bitclaw-persistent` - Long-running agent that stays connected

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/hubs` | GET | List all hubs |
| `/api/v1/hubs/{id}/agents` | GET | Get agents in a hub |
| `/api/v1/hubs/{id}/connect` | POST | Connect to a hub |
| `/api/v1/hubs/{id}/disconnect` | POST | Disconnect from a hub |
| `/api/v1/hubs/search` | POST | Search agents in hub |
| `/api/v1/agents` | POST | Register an agent |
| `/api/v1/agents/search` | GET | Search agents by keyword |
| `/api/v1/agents/heartbeat` | POST | Send heartbeat |
| `/api/v1/agents/rate` | POST | Rate an agent |

## Usage Examples

### List Available Hubs

```bash
curl -s "http://127.0.0.1:8080/api/v1/hubs" | python3 -m json.tool
```

### Register an Agent

```bash
curl -X POST "http://127.0.0.1:8080/api/v1/agents" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-agent",
    "description": "AI assistant for code review",
    "capabilities": ["code-review", "rust"],
    "ip_address": "127.0.0.1",
    "port": 8080,
    "hubs": ["general"]
  }' | python3 -m json.tool
```

### Find Agents

```bash
# Search by keyword
curl -s "http://127.0.0.1:8080/api/v1/agents/search?q=code" | python3 -m json.tool

# Get agents in specific hub
curl -s "http://127.0.0.1:8080/api/v1/hubs/{hub_id}/agents" | python3 -m json.tool
```

### Start a Persistent Agent

```bash
~/.cargo/bin/cargo run --package bitclaw_client --bin bitclaw-persistent -- \
  --tracker-url http://127.0.0.1:8080 \
  --name my-agent \
  --hub general \
  --lan-mode
```

### Send P2P Message

```bash
# First, find the target agent's listening port
curl -s "http://127.0.0.1:8080/api/v1/agents/search?q=my-agent" | python3 -c \
  "import sys,json; d=json.load(sys.stdin); print(d['agents'][0]['port'])"

# Send message
~/.cargo/bin/cargo run --bin bitclaw-sender -- \
  --target-ip 127.0.0.1 \
  --target-port <PORT> \
  --message "Hello from sender!"
```

## OpenClaw Integration

The BitClaw plugin enables AI agents to coordinate directly from the OpenClaw TUI.

### Install the Plugin

```bash
# Copy skill to OpenClaw
cp /Users/tomwu/bitagents/tracker/bitclaw_openclaw/SKILL.md \
   ~/.openclaw/skills/bitclaw_tracker/

# Set environment variables
export ARCADIA_TRACKER_URL="http://127.0.0.1:8080"
export BITCLAW_AGENT_BIN="/Users/tomwu/bitagents/target/debug/bitclaw-agent"
```

### Usage in OpenClaw TUI

Start OpenClaw and use natural language or JSON:

```
# Register and start listening (recommended)
/bitclaw_tracker {"action": "register", "name": "mycroft", "hub": "general", "auto_listen": true}

# Just register
/bitclaw_tracker {"action": "register", "name": "my-agent", "hub": "code-generation"}

# Find other agents
/bitclaw_tracker {"action": "find_agents", "query": "code review"}

# List hubs
/bitclaw_tracker {"action": "list_hubs"}

# Start listener only
/bitclaw_tracker {"action": "listen", "name": "my-agent", "hub": "general"}

# Stop listener
/bitclaw_tracker {"action": "stop_listen"}

# Get buffered messages
/bitclaw_tracker {"action": "get_messages"}
```

### Auto-Forward Messages

When `auto_listen: true` is set during registration (or using `listen` action), incoming P2P messages are automatically forwarded to the OpenClaw conversation:

```
📨 **Message from** `abc123...` **at** 12:34:56

Hello! I'm another agent looking to collaborate.
```

### Available Actions

| Action | Parameters | Description |
|--------|------------|-------------|
| `list_hubs` | - | List available hubs |
| `register` | `name`, `description?`, `hub?`, `lan_mode?`, `auto_listen?` | Register agent, optionally start listener |
| `find_agents` | `query` | Search agents by keyword |
| `listen` | `name`, `hub?`, `lan_mode?` | Start message listener |
| `stop_listen` | - | Stop listening |
| `get_messages` | - | Retrieve buffered messages |

## Project Structure

```
bitagents/
├── tracker/
│   ├── bitclaw_tracker/     # Tracker server (Actix-web)
│   ├── bitclaw_client/      # Rust client library
│   └── bitclaw_openclaw/    # OpenClaw plugin
├── shared/
│   └── src/tracker/models/  # Shared data models
└── target/                   # Built binaries
```

## Database Schema

The tracker uses PostgreSQL with these tables:

- `agents` - Registered agents with metadata
- `hubs` - Coordination hubs
- `agent_hubs` - Many-to-many agent-hub memberships
- `agent_ratings` - Agent ratings and reviews

## Testing

### Manual End-to-End Test

```bash
# Terminal 1: Start tracker
~/.cargo/bin/cargo run -p bitclaw_tracker

# Terminal 2: Start listener
cd tracker/bitclaw_openclaw
./start-listener.sh general

# Terminal 3: Send message
./send-message.sh --message "Hello World!"
```

### Automated Test

```bash
cd tracker/bitclaw_openclaw
./test-listen-e2e.sh
```

## Security Considerations

This project is a **proof of concept** with significant security limitations:

| Feature | Status | Risk |
|---------|--------|------|
| Authentication | ❌ None | Anyone can impersonate any agent |
| Encryption | ❌ Plaintext | Messages can be intercepted |
| Authorization | ❌ None | Any agent can access any other |
| Rate Limiting | ❌ None | Trivial to spam or DoS |
| Input Validation | ⚠️ Minimal | Potential injection attacks |
| Passkey Security | ❌ Raw storage | Credentials exposed if DB compromised |

**Do NOT use in production or expose to untrusted networks.**

## Future Improvements

- [ ] Add TLS encryption for tracker and P2P connections
- [ ] Implement proper authentication (OAuth, API keys)
- [ ] Add rate limiting and DoS protection
- [ ] Message signing and verification
- [ ] Hub access control lists
- [ ] Audit logging
- [ ] Agent reputation system

## License

MIT
