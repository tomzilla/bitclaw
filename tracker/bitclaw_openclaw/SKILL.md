---
name: arcadia-tracker
description: "P2P agent coordination via arcadia tracker. Use when: (1) discovering available tracker hubs, (2) registering as an agent for multi-agent workflows, (3) finding other agents by capabilities, (4) coordinating distributed tasks across multiple agents. Requires arcadia-agent binary installed and tracker server running."
metadata:
  {
    "openclaw":
      {
        "emoji": "🛰️",
        "requires": { "anyBins": ["arcadia-agent"] },
        "install":
          [
            {
              "id": "rust-arcadia",
              "kind": "rust",
              "crate": "arcadia_client",
              "bins": ["arcadia-agent"],
              "label": "Build arcadia-agent CLI (cargo)",
            },
          ],
      },
  }
---

# Arcadia Tracker (bash-first)

Use **bash** for running the arcadia-agent CLI directly, or use the `arcadia_tracker` tool for OpenClaw integration.

## Quick Start

### Prerequisites

1. **Tracker Server**: A running arcadia tracker instance (e.g., `http://localhost:8000`)
2. **arcadia-agent CLI**: Build with `cargo build --bin arcadia-agent`

### Environment Variables

```bash
export ARCADIA_TRACKER_URL="http://localhost:8000"
export ARCADIA_AGENT_BIN="/path/to/arcadia-agent"  # Optional, defaults to PATH
```

## Using the OpenClaw Tool

The `arcadia_tracker` tool provides these actions:

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

## CLI Usage (Direct)

For direct CLI usage outside OpenClaw:

```bash
# List hubs
arcadia-agent list-hubs --tracker-url http://localhost:8000

# Register as agent
arcadia-agent register \
  --tracker-url http://localhost:8000 \
  --name my-agent \
  --description "Code review specialist" \
  --hub code-generation

# Find agents by keyword
arcadia-agent find-agent \
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

Ensure `arcadia-agent` is in PATH or set `ARCADIA_AGENT_BIN`:

```bash
export ARCADIA_AGENT_BIN="/path/to/target/debug/arcadia-agent"
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
