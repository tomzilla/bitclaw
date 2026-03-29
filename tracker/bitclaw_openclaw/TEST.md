# OpenClaw + Arcadia Tracker Integration Test

## Overview

This test demonstrates:
1. OpenClaw connecting to the arcadia tracker network via the plugin
2. A second client discovering the OpenClaw client
3. P2P message exchange (future: requires persistent connection)

## Quick Start

### Option A: Automated E2E Test

```bash
cd /Users/tomwu/bitagents/tracker/arcadia_openclaw
./test-e2e.sh
```

### Option B: Manual Step-by-Step

See `MANUAL_TEST_GUIDE.sh` for detailed manual testing steps.

## Test Architecture

```
┌─────────────────┐                          ┌─────────────────┐
│   OpenClaw      │                          │   Sender Client │
│   (Plugin)      │                          │   (CLI)         │
│   client_id:    │                          │   client_id:    │
│   xxxxx-xxxx    │                          │   yyyyy-yyyy    │
└────────┬────────┘                          └────────┬────────┘
         │                                            │
         │         ┌──────────────────┐               │
         └────────▶│  Tracker Server  │◀──────────────┘
                   │  (port 8000)     │
                   │                  │
                   │  - Hub Registry  │
                   │  - Agent Search  │
                   │  - P2P Discovery │
                   └──────────────────┘
```

## Files

| File | Description |
|------|-------------|
| `test-e2e.sh` | Automated end-to-end test script |
| `MANUAL_TEST_GUIDE.sh` | Step-by-step manual testing guide |
| `test-integration.sh` | Integration test setup (alternative) |

## Expected Output

### Step 1: Tracker Server Starts
```
[INFO] Tracker started (PID: 12345)
```

### Step 2: List Hubs
```json
{
  "success": true,
  "data": {
    "hubs": [
      {"hub_id": "...", "name": "test-hub", ...}
    ]
  }
}
```

### Step 3: OpenClaw Connects
```json
{
  "success": true,
  "data": {
    "hub_id": "...",
    "hub_name": "test-hub",
    "client_id": "abc-123",
    "local_address": "0.0.0.0:PORT",
    "upnp_enabled": false
  }
}
```

### Step 4: Sender Discovers OpenClaw
```json
{
  "success": true,
  "data": {
    "hub": "test-hub",
    "query": "openclaw",
    "agents": [
      {"agent_id": "...", "name": "openclaw-agent", ...}
    ]
  }
}
```

## Troubleshooting

### Tracker won't start
```bash
# Check PostgreSQL is running
psql -U postgres -h localhost -c "SELECT 1"

# Check port 8000 is free
lsof -i :8000
```

### CLI returns empty hubs
```bash
# The tracker needs hubs seeded - check the migration
# Hubs should be created in database migration
```

### OpenClaw plugin not found
```bash
# Ensure plugin is in OpenClaw extensions path
# or set OPENCLAW_PLUGIN_LOAD_PATHS
```

## Next Steps

After basic connectivity works:

1. **Implement persistent connection**: Modify `service.ts` to spawn a long-running arcadia-agent process

2. **Add message sending**: Implement `send_message` and `broadcast_message` actions

3. **Handle incoming messages**: Add event listener for P2P messages and forward to OpenClaw

4. **Multi-agent orchestration**: Test real workflows like:
   - Batch PR reviews
   - Parallel issue fixing
   - Capability-based agent discovery
