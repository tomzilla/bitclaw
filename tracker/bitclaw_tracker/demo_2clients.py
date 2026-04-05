#!/usr/bin/env python3
"""
Demo: Two Persistent Clients with P2P Discovery

Client 1 (DataAnalyzer) registers with the tracker and stays online.
Client 2 (NLPAssistant) starts, discovers Client 1 via the tracker,
and connects to it directly via TCP for peer-to-peer communication.

Usage:
    python3 demo_2clients.py
"""

import json
import urllib.request
import urllib.error
import sys
import time
import uuid
import random
import threading
import socket
from typing import Dict, Optional, Any

BASE_URL = "http://localhost:8080/api/v1"

# Client profiles
CLIENT_1_PROFILE = {
    "name": "DataAnalyzer-" + str(uuid.uuid4())[:8],
    "description": "AI agent specialized in data analysis and statistical modeling using Python",
    "capabilities": ["data-analysis", "python", "statistics"],
    "hubs": ["data-analysis", "general"]
}

CLIENT_2_PROFILE = {
    "name": "NLPAssistant-" + str(uuid.uuid4())[:8],
    "description": "Natural language processing agent for text analysis and translation",
    "capabilities": ["nlp", "text-analysis", "translation"],
    "hubs": ["nlp", "general"]
}


def make_request(method: str, path: str, data: Optional[Dict] = None, timeout: int = 10) -> Dict:
    """Make HTTP request"""
    url = f"{BASE_URL}{path}"
    headers = {"Content-Type": "application/json"}

    if data:
        data = json.dumps(data).encode('utf-8')

    req = urllib.request.Request(url, data=data, headers=headers, method=method)

    try:
        with urllib.request.urlopen(req, timeout=timeout) as response:
            return {
                "status": response.status,
                "body": json.loads(response.read().decode('utf-8')) if response.status != 204 else None
            }
    except urllib.error.HTTPError as e:
        return {
            "status": e.code,
            "body": json.loads(e.read().decode('utf-8')) if e.code != 204 else None
        }
    except urllib.error.URLError as e:
        return {"error": f"Connection failed: {e.reason}"}
    except Exception as e:
        return {"error": str(e)}


class PersistentClient:
    """A persistent client that registers, sends heartbeats, and accepts P2P connections"""

    def __init__(self, name: str, profile: Dict, port: int):
        self.name = name
        self.profile = profile
        self.agent_id = ""
        self.passkey = ""
        self.is_registered = False
        self.port = port
        self.server_socket = None
        self.connected_peers = []
        self.running = False
        self.received_messages = []

    def register(self) -> bool:
        """Register with the tracker"""
        print(f"[{self.name}] Registering with tracker...")

        # Generate a random port for P2P connections
        p2p_port = random.randint(9000, 9999)

        agent_data = {
            "name": self.profile["name"],
            "description": self.profile["description"],
            "capabilities": self.profile["capabilities"],
            "hubs": self.profile["hubs"],
            "endpoint": f"http://localhost:{p2p_port}/api"
        }

        result = make_request("POST", "/agents", agent_data)

        if result.get("status") == 200 and "agent_id" in result.get("body", {}):
            self.agent_id = result["body"]["agent_id"]
            self.passkey = result["body"]["agent_passkey"]
            self.is_registered = True
            print(f"[{self.name}] Registered successfully!")
            print(f"           Agent ID: {self.agent_id}")
            print(f"           Passkey: {self.passkey[:20]}...")
            return True

        print(f"[{self.name}] Registration failed: {result}")
        return False

    def heartbeat(self) -> Dict:
        """Send heartbeat to tracker"""
        if not self.is_registered:
            return {"error": "Not registered"}

        data = {
            "agent_id": self.agent_id,
            "passkey": self.passkey,
            "status": "active"
        }
        return make_request("POST", "/agents/heartbeat", data)

    def search_agents(self, **params) -> list:
        """Search for other agents"""
        query = "&".join(f"{k}={v}" for k, v in params.items())
        path = f"/agents/search?{query}" if query else "/agents/search"
        result = make_request("GET", path)

        if result.get("status") == 200:
            return result.get("body", {}).get("agents", [])
        return []

    def get_hub_agents(self, hub_id: str) -> list:
        """Get agents in a specific hub"""
        result = make_request("GET", f"/hubs/{hub_id}/agents")
        if result.get("status") == 200:
            return result.get("body", {}).get("agents", [])
        return []

    def start_p2p_listener(self):
        """Start TCP listener for P2P connections"""
        self.running = True
        self.server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.server_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)

        try:
            self.server_socket.bind(('localhost', self.port))
            self.server_socket.listen(5)
            self.server_socket.settimeout(1.0)
            print(f"[{self.name}] P2P listener started on port {self.port}")

            while self.running:
                try:
                    conn, addr = self.server_socket.accept()
                    print(f"[{self.name}] New P2P connection from {addr}")
                    self.connected_peers.append((conn, addr))

                    # Handle connection in thread
                    threading.Thread(target=self.handle_peer, args=(conn, addr), daemon=True).start()
                except socket.timeout:
                    continue
                except Exception as e:
                    if self.running:
                        print(f"[{self.name}] P2P error: {e}")
                    break
        except Exception as e:
            print(f"[{self.name}] Failed to start P2P listener: {e}")
        finally:
            if self.server_socket:
                self.server_socket.close()

    def handle_peer(self, conn: socket.socket, addr):
        """Handle incoming P2P connection"""
        try:
            conn.settimeout(5.0)
            data = conn.recv(4096)
            if data:
                message = data.decode('utf-8')
                print(f"[{self.name}] Received from {addr}: {message}")
                self.received_messages.append({"from": addr, "message": message})

                # Send response
                response = f"Hello from {self.name}!".encode('utf-8')
                conn.send(response)
        except Exception as e:
            print(f"[{self.name}] Error handling peer {addr}: {e}")
        finally:
            conn.close()

    def connect_to_peer(self, peer_name: str, port: int, message: str) -> bool:
        """Connect to another client via TCP"""
        print(f"[{self.name}] Connecting to {peer_name} on localhost:{port}...")

        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.settimeout(5.0)
            sock.connect(('localhost', port))

            # Send message
            sock.send(message.encode('utf-8'))
            print(f"[{self.name}] Sent to {peer_name}: {message}")

            # Receive response
            response = sock.recv(4096)
            print(f"[{self.name}] Received from {peer_name}: {response.decode('utf-8')}")

            sock.close()
            return True
        except Exception as e:
            print(f"[{self.name}] Failed to connect to {peer_name}: {e}")
            return False

    def stop(self):
        """Stop the client"""
        self.running = False
        if self.server_socket:
            self.server_socket.close()
        for conn, _ in self.connected_peers:
            try:
                conn.close()
            except:
                pass
        print(f"[{self.name}] Stopped")


def main():
    print("=" * 70)
    print("DEMO: Two Persistent Clients with P2P Discovery")
    print("=" * 70)
    print()

    # Check server is running
    print("Checking tracker server...")
    result = make_request("GET", "/hubs")
    if "error" in result:
        print(f"ERROR: Tracker not running: {result['error']}")
        sys.exit(1)
    print("Tracker is running!\n")

    # =========================================================================
    # CLIENT 1: DataAnalyzer - Registers and starts P2P listener
    # =========================================================================
    print("=" * 70)
    print("CLIENT 1: DataAnalyzer")
    print("=" * 70)

    client1 = PersistentClient("DataAnalyzer", CLIENT_1_PROFILE, port=9501)

    # Register client 1
    if not client1.register():
        print("Failed to register Client 1")
        sys.exit(1)

    # Start P2P listener in background
    listener_thread = threading.Thread(target=client1.start_p2p_listener, daemon=True)
    listener_thread.start()
    time.sleep(0.5)  # Give listener time to start

    # Send first heartbeat
    hb_result = client1.heartbeat()
    if hb_result.get("status") == 200:
        print(f"[DataAnalyzer] Heartbeat sent, interval: {hb_result['body'].get('interval', 'N/A')}s")
    print()

    # =========================================================================
    # CLIENT 2: NLPAssistant - Starts and discovers Client 1
    # =========================================================================
    print("=" * 70)
    print("CLIENT 2: NLPAssistant (Discovery Phase)")
    print("=" * 70)

    client2 = PersistentClient("NLPAssistant", CLIENT_2_PROFILE, port=9502)

    # Register client 2
    if not client2.register():
        print("Failed to register Client 2")
        sys.exit(1)

    # Start P2P listener
    listener_thread2 = threading.Thread(target=client2.start_p2p_listener, daemon=True)
    listener_thread2.start()
    time.sleep(0.5)

    # Send heartbeat
    hb_result2 = client2.heartbeat()
    print()

    # =========================================================================
    # DISCOVERY: Client 2 searches for Client 1
    # =========================================================================
    print("-" * 70)
    print("DISCOVERY PHASE")
    print("-" * 70)

    # Search by capability
    print(f"\n[{client2.name}] Searching for agents with 'data-analysis' capability...")
    found_agents = client2.search_agents(capability="data-analysis")

    if found_agents:
        print(f"[{client2.name}] Found {len(found_agents)} agent(s):")
        for agent in found_agents:
            print(f"           - {agent['name']} (ID: {agent['agent_id'][:8]}...)")
            print(f"             Description: {agent['description'][:60]}...")
    else:
        print(f"[{client2.name}] No agents found with 'data-analysis' capability")

    # Also search by hub
    print(f"\n[{client2.name}] Getting agents from 'general' hub...")
    # Need to get hub ID first
    hubs_result = make_request("GET", "/hubs")
    general_hub = None
    for hub in hubs_result.get("body", {}).get("hubs", []):
        if hub["name"] == "general":
            general_hub = hub["hub_id"]
            break

    if general_hub:
        hub_agents = client2.get_hub_agents(general_hub)
        if hub_agents:
            print(f"[{client2.name}] Found {len(hub_agents)} agent(s) in general hub:")
            for agent in hub_agents:
                print(f"           - {agent['name']}")

    print()

    # =========================================================================
    # P2P CONNECTION: Client 2 connects to Client 1
    # =========================================================================
    print("-" * 70)
    print("P2P CONNECTION PHASE")
    print("-" * 70)
    print()

    # Client 2 connects directly to Client 1
    print(f"[{client2.name}] Initiating P2P connection to {client1.name}...")
    success = client2.connect_to_peer(
        client1.name,
        client1.port,
        f"Hello {client1.name}! I discovered you via the tracker. Let's collaborate!"
    )

    if success:
        print(f"\n[SUCCESS] P2P connection established!")
        print(f"          {client2.name} <-> {client1.name}")
    else:
        print(f"\n[FAILED] P2P connection failed")

    print()

    # =========================================================================
    # CONTINUOUS OPERATION: Both clients send heartbeats
    # =========================================================================
    print("-" * 70)
    print("CONTINUOUS OPERATION")
    print("-" * 70)

    heartbeat_count = 0
    try:
        while heartbeat_count < 5:  # Send 5 heartbeats then exit
            time.sleep(3)
            heartbeat_count += 1

            # Client 1 heartbeat
            hb1 = client1.heartbeat()
            status1 = "OK" if hb1.get("status") == 200 else "FAIL"

            # Client 2 heartbeat
            hb2 = client2.heartbeat()
            status2 = "OK" if hb2.get("status") == 200 else "FAIL"

            print(f"[heartbeat #{heartbeat_count}] Client1: {status1}, Client2: {status2}")

            # Check discovered agents in heartbeat response
            if hb2.get("status") == 200 and "discovered_agents" in hb2.get("body", {}):
                discovered = hb2["body"]["discovered_agents"]
                if discovered:
                    print(f"                 Client2 discovered {len(discovered)} agent(s)")
    except KeyboardInterrupt:
        print("\nInterrupted by user")

    print()

    # Cleanup
    print("=" * 70)
    print("SHUTDOWN")
    print("=" * 70)

    client1.stop()
    client2.stop()

    print()
    print("Demo completed!")
    print()
    print("Summary:")
    print(f"  - {client1.name}: Registered, sent {heartbeat_count} heartbeats")
    print(f"  - {client2.name}: Discovered {client1.name} via tracker, connected via P2P")
    print()


if __name__ == "__main__":
    main()
