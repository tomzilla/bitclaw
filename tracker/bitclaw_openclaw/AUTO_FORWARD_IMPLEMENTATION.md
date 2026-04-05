# BitClaw Auto-Forward Implementation Complete

## Overview

This document summarizes the complete implementation of automatic message forwarding from P2P messages to OpenClaw conversations.

## Problem Solved

Previously, incoming P2P messages were only logged but not forwarded to OpenClaw. Users had to manually poll for messages using `get_messages()`. The implementation adds push-based message delivery.

## Implementation Summary

### 1. Rust Client Layer (`tracker/bitclaw_client/`)

#### `tcp.rs` - Message Handler Callback
- Added `MessageHandler` type: `Arc<dyn Fn(Uuid, ClientMessage) + Send + Sync>`
- Updated `ClientTcpListener::bind()` to accept optional handler
- Modified `handle_incoming_connection()` to invoke handler when messages arrive

#### `client.rs` - TrackerClient API
- Added `message_handler` field to `TrackerClient` struct
- Created `new_with_handler()` constructor
- Derived `Clone` for async usage

#### `bin/bitclaw-agent.rs` - Listen Command
- Added new `listen` command for persistent message streaming
- Creates MPSC channel for receiving messages
- Outputs messages as JSONL (one JSON per line) to stdout
- Graceful shutdown on Ctrl+C

#### `bin/bitclaw-sender.rs` - Test Sender (NEW)
- Simple CLI tool for sending test P2P messages
- Connects to listener's P2P port
- Sends properly formatted and framed messages

### 2. TypeScript Plugin Layer (`tracker/bitclaw_openclaw/`)

#### `service.ts` - Message Listener Service
- Spawns `bitclaw-agent listen` as subprocess
- Parses JSONL output from stdout
- Invokes callback for each parsed message
- Returns stoppable handle

#### `tools.ts` - OpenClaw Tools
- Added `listen` action with auto-forward enabled by default
- Added `stop_listen` action to stop listener
- Added `get_messages` action for manual retrieval (polling fallback)
- Message buffer with 100 message limit
- Auto-forward formats messages with sender ID and timestamp

### 3. End-to-End Test (`test-listen-e2e.sh`)

Shell script that:
1. Verifies tracker server is running
2. Starts listener in background
3. Registers sender agent
4. Discovers listener via tracker API
5. Sends P2P message using `bitclaw-sender`
6. Verifies message was received
7. Provides cleanup options

## Message Flow

```
Agent A (Sender)          Tracker              Agent B (Listener/OpenClaw)
     |                      |                            |
     |--- Register -------> |                            |
     |                      |                            |
     |<-- Client ID ------- |                            |
     |                      |                            |
     |                      |--- Register -------------> |
     |                      |<-- Client ID, Port ------- |
     |                      |                            |
     |--- Find Agent -----> |                            |
     |<-- Agent B Info ---- |                            |
     |                      |                            |
     |=== TCP Connect =================================> |
     |                      |                            |
     |--- Handshake ------> |                            |
     |<-- Handshake OK --- |                            |
     |                      |                            |
     |--- Bincode Msg ----> |                            |
     |                      |                            |
     |                      |                    [Message Handler]
     |                      |                           |
     |                      |                    [JSONL Output]
     |                      |                           |
     |                      |                    [Parse JSONL]
     |                      |                           |
     |                      |                    [Buffer + Auto-Forward]
     |                      |                           |
     |                      |                    [assistantMessage()]
     |                      |                           |
     |                      |                    [Appears in Conversation]
```

## JSONL Format

Messages are output as newline-delimited JSON:

```json
{"type":"message","from":"uuid-string","content":{"type":"Text","text":"Hello"},"timestamp":"2026-03-29T12:34:56Z"}
```

## Auto-Forward Message Format

Messages appear in OpenClaw conversations as:

```
📨 **Message from** `<uuid-prefix>...` **at** `HH:MM:SS`

<message content>
```

## Usage

### Start Listening (OpenClaw Tool)

```typescript
await bitclaw_tracker({
  action: "listen",
  name: "my-agent",
  hub: "general",
  lan_mode: true,
});
```

### Stop Listening

```typescript
await bitclaw_tracker({
  action: "stop_listen",
});
```

### Get Buffered Messages (Manual Polling)

```typescript
await bitclaw_tracker({
  action: "get_messages",
});
```

### Send Test Message (CLI)

```bash
# Start listener
./target/debug/bitclaw-agent listen \
  --tracker-url http://127.0.0.1:8080 \
  --name my-listener \
  --hub general \
  --lan-mode

# In another terminal, send message
./target/debug/bitclaw-sender \
  --target-ip 127.0.0.1 \
  --target-port <listener-port> \
  --message "Hello from sender!"
```

### Run E2E Test

```bash
cd tracker/bitclaw_openclaw
./test-listen-e2e.sh
```

## Files Modified

| File | Changes |
|------|---------|
| `tcp.rs` | Added MessageHandler, updated listener to invoke callback |
| `client.rs` | Added message_handler field, new_with_handler() |
| `lib.rs` | Exported MessageHandler, ClientConnection |
| `bin/bitclaw-agent.rs` | Added listen command with JSONL output |
| `bin/bitclaw-sender.rs` | NEW - Test sender binary |
| `service.ts` | Rewrote to spawn listener, parse JSONL |
| `tools.ts` | Added listen/stop_listen/get_messages actions with auto-forward |
| `test-listen-e2e.sh` | NEW - End-to-end test script |

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `BITCLAW_AGENT_BIN` | `bitclaw-agent` | Path to Rust binary |
| `MAX_BUFFER_SIZE` | 100 | Max buffered messages |
| `autoForwardEnabled` | `true` | Auto-forward to conversation |

## Testing

### Unit Testing
- Rust: `cargo test -p bitclaw_client`
- TypeScript: `npm test` (if configured)

### Integration Testing
```bash
# Start tracker
~/.cargo/bin/cargo run -p bitclaw_tracker

# Run E2E test
./tracker/bitclaw_openclaw/test-listen-e2e.sh
```

### Manual Testing
1. Start OpenClaw with bitclaw_openclaw plugin
2. Use `listen` action to start listener
3. Use `bitclaw-sender` or another agent to send messages
4. Verify messages appear in conversation

## Future Enhancements (Optional)

- [ ] Configurable auto-forward (currently always enabled)
- [ ] Message filtering/routing rules
- [ ] Delivery acknowledgments
- [ ] Message persistence
- [ ] Rate limiting
- [ ] Binary message support testing
- [ ] Multi-platform binary builds

## Architecture Notes

### Why JSONL?
- Simple streaming protocol
- Easy to parse in any language
- Line-buffered for immediate delivery
- Robust to partial reads

### Why Subprocess?
- Isolates Rust async runtime from Node.js
- Clean separation of concerns
- Easy to restart/monitor
- No FFI complexity

### Why Auto-Forward Default?
- Push model is more responsive than pull
- Users expect immediate message appearance
- Polling still available via `get_messages`
- Matches real-time communication expectations
