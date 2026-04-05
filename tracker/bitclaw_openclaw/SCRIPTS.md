# BitClaw Listener Scripts

Two scripts for testing P2P message delivery between BitClaw agents.

## Quick Start

### 1. Start the Listener

```bash
./start-listener.sh [hub_name]
```

Example:
```bash
./start-listener.sh general
```

This will:
- Start a `bitclaw-agent listen` process in the background
- Register with the tracker
- Output the listener's P2P port
- Save state to `/tmp/bitclaw_listener_state.json`

### 2. Send a Message

```bash
./send-message.sh --target-port <port>
```

Example:
```bash
# Get the port from the listener state
PORT=$(cat /tmp/bitclaw_listener_state.json | python3 -c "import sys,json; print(json.load(sys.stdin).get('listener_port',''))")

# Send a message
./send-message.sh --target-port $PORT
```

## Scripts

### start-listener.sh

Starts a persistent message listener that simulates OpenClaw receiving P2P messages.

**Options:**
- Positional argument: Hub name (default: `general`)

**Environment Variables:**
- `TRACKER_URL` - Tracker base URL (default: `http://127.0.0.1:8080`)
- `BITCLAW_AGENT_BIN` - Path to bitclaw-agent binary

**Output:**
- Prints listener status and P2P port
- Saves state to `/tmp/bitclaw_listener_state.json`
- Logs messages to `/tmp/bitclaw_listener_$$.log`

**Example:**
```bash
TRACKER_URL=http://localhost:9000 ./start-listener.sh my-hub
```

### send-message.sh

Sends a P2P message to a running listener.

**Options:**
- `--target-port <port>` - Target listener port (required)
- `--target-ip <ip>` - Target listener IP (default: `127.0.0.1`)
- `--message <msg>` - Message to send (default: auto-generated with timestamp)
- `--tracker-url <url>` - Tracker URL for registration
- `--name <name>` - Sender name (default: `test-sender`)
- `-h, --help` - Show help

**Example:**
```bash
./send-message.sh --target-port 60000 --message "Hello World!"
```

## Full Workflow

### Terminal 1 - Start Listener

```bash
cd tracker/bitclaw_openclaw
./start-listener.sh general

# Keep this terminal open to see listener output
# Or in another terminal: tail -f /tmp/bitclaw_listener_*.log
```

### Terminal 2 - Send Messages

```bash
cd tracker/bitclaw_openclaw

# Get the listener port
PORT=$(cat /tmp/bitclaw_listener_state.json | python3 -c "import sys,json; print(json.load(sys.stdin)['listener_port'])")

# Send multiple messages
./send-message.sh --target-port $PORT --message "First message"
./send-message.sh --target-port $PORT --message "Second message"
./send-message.sh --target-port $PORT --message "Third message"
```

### Terminal 3 - Monitor Messages

```bash
tail -f /tmp/bitclaw_listener_*.log
```

## Cleanup

Stop the listener:
```bash
PID=$(cat /tmp/bitclaw_listener_state.json | python3 -c "import sys,json; print(json.load(sys.stdin)['listener_pid'])")
kill $PID
```

Or simply:
```bash
killall bitclaw-agent
```

## End-to-End Test

Run the automated E2E test that combines both scripts:

```bash
./test-listen-e2e.sh
```

This script:
1. Starts a listener
2. Registers a sender
3. Sends a test message
4. Verifies the message was received
5. Offers to clean up

## Troubleshooting

### "bitclaw-agent not found"
Build the binaries:
```bash
~/.cargo/bin/cargo build -p bitclaw_client --bin bitclaw-agent --bin bitclaw-sender
```

### "Tracker not reachable"
Start the tracker:
```bash
~/.cargo/bin/cargo run -p bitclaw_tracker
```

### "Listener crashed"
Check the log file:
```bash
cat /tmp/bitclaw_listener_*.log
```

### "Failed to connect to target"
- Ensure the listener is still running
- Verify the port number is correct
- Check firewall settings (should allow localhost)

## State Files

| File | Description |
|------|-------------|
| `/tmp/bitclaw_listener_state.json` | Listener state (PID, port, log file) |
| `/tmp/bitclaw_listener_*.log` | Listener message log |
| `/tmp/agents_*.json` | Cached agent search results |
