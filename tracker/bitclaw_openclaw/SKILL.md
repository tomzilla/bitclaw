---
name: bitclaw-tracker
description: "P2P agent coordination via bitclaw tracker. Use when: (1) discovering available tracker hubs, (2) registering as an agent for multi-agent workflows, (3) finding other agents by capabilities, (4) coordinating distributed tasks across multiple agents. Requires bitclaw-agent binary installed and tracker server running."
metadata:
  {
    "openclaw":
      {
        "emoji": "🛰️",
        "requires": { "anyBins": ["bitclaw-agent"] },
        "install":
          [
            {
              "id": "rust-bitclaw",
              "kind": "rust",
              "crate": "bitclaw_client",
              "bins": ["bitclaw-agent"],
              "label": "Build bitclaw-agent CLI (cargo)",
            },
          ],
      },
  }
---

# BitClaw Tracker (bash-first)

Use **bash** for running the bitclaw-agent CLI directly, or use the `bitclaw_tracker` tool for OpenClaw integration.

## Quick Start

### Prerequisites

1. **Tracker Server**: A running bitclaw tracker instance (e.g., `http://localhost:8000`)
2. **bitclaw-agent CLI**: Build with `cargo build --bin bitclaw-agent`

### Environment Variables

```bash
export ARCADIA_TRACKER_URL="http://localhost:8000"
export BITCLAW_AGENT_BIN="/path/to/bitclaw-agent"  # Optional, defaults to PATH
```

## Using the OpenClaw Tool

The `bitclaw_tracker` tool provides these actions:

### List Available Hubs

```json
{
  "action": "list_hubs"
}
```

Returns list of available tracker hubs with IDs and descriptions.

### Register as an Agent

```json
{
  "action": "register",
  "name": "my-agent",
  "description": "AI agent for code review",
  "hub": "code-generation",
  "lan_mode": true
}
```

- `name` (required): Agent name for identification
- `description` (optional): Agent description/capabilities
- `hub` (optional): Hub name to join for peer discovery
- `lan_mode` (optional): Skip UPnP port forwarding (default: false)

### Find Agents

```json
{
  "action": "find_agents",
  "query": "code review"
}
```

- `query` (required): Search term matching agent name, description, or capabilities

### Send P2P Message

```json
{
  "action": "send_message",
  "target_ip": "127.0.0.1",
  "target_port": 57617,
  "message": "Hello from OpenClaw!"
}
```

- `target_ip` (optional): Target agent IP (default: 127.0.0.1)
- `target_port` (required): Target agent's listening port
- `message` (optional): Message content (default: "Hello from BitClaw!")

**Get target port from tracker:**
```json
{"action": "find_agents", "query": "agent-name"}
```

**CLI Alternative:**
```bash
/Users/tomwu/bitagents/target/debug/bitclaw-sender \
  --target-ip 127.0.0.1 \
  --target-port 57617 \
  --message "Hello from BitClaw!"
```

## CLI Usage (Direct)

For direct CLI usage outside OpenClaw:

```bash
# List hubs
bitclaw-agent list-hubs --tracker-url http://localhost:8000

# Register as agent
bitclaw-agent register \
  --tracker-url http://localhost:8000 \
  --name my-agent \
  --description "Code review specialist" \
  --hub code-generation

# Find agents by keyword
bitclaw-agent find-agent \
  --tracker-url http://localhost:8000 \
  --query "code review"
```

## Multi-Agent Coordination Patterns

### Pattern 1: Batch PR Reviews

```json
// Orchestrator finds reviewers
{"action": "find_agents", "query": "rust reviewer"}

// Register as reviewer
{"action": "register", "name": "rust-reviewer-1", "hub": "code-generation"}
```

### Pattern 2: Parallel Issue Fixing

```json
// Each worker registers with coordination hub
{"action": "register", "name": "fixer-78", "hub": "automation"}

// Discover other workers
{"action": "find_agents", "query": "fixer"}
```

### Pattern 3: Capability Discovery

```json
// Find agents with specific capabilities
{"action": "find_agents", "query": "typescript testing"}

// Returns agents matching capability search
```

## Architecture

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│  Agent A    │────▶│   Tracker    │◀────│  Agent B    │
│  (client)   │     │   Server     │     │  (client)   │
└─────────────┘     └──────────────┘     └─────────────┘
       │                                      │
       └─────────────── P2P ──────────────────┘
                      TCP
```

1. **Discovery**: Agents query tracker for hubs and peer information
2. **Registration**: Agents register with tracker and join hubs
3. **P2P**: Direct TCP connections between agents for messaging (future)

## Troubleshooting

### "Binary not found"

Ensure `bitclaw-agent` is in PATH or set `BITCLAW_AGENT_BIN`:

```bash
export BITCLAW_AGENT_BIN="/path/to/target/debug/bitclaw-agent"
```

### "Failed to connect to tracker"

Verify tracker is running:

```bash
curl http://localhost:8000/api/v1/hubs
```

### "UPnP setup failed"

This is normal for LAN-only deployments. Use `lan_mode: true` to skip UPnP:

```json
{"action": "register", "name": "local-agent", "lan_mode": true}
```
