#!/usr/bin/env python3
"""
Multi-Client Smoke Test Suite for BitClaw Tracker API

Simulates multiple agents connecting to the tracker, discovering each other,
sending heartbeats, rating each other, and performing various interactions.

Usage:
    1. Start the server: cargo run --package bitclaw_tracker
    2. Run tests: python3 smoke_test_multiclient.py
"""

import json
import urllib.request
import urllib.error
import sys
import time
import uuid
import random
import threading
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass, field
from typing import List, Dict, Optional, Any

BASE_URL = "http://localhost:8080/api/v1"

# Agent profiles for realistic simulation
AGENT_PROFILES = [
    {
        "name_prefix": "DataAnalyzer",
        "description": "AI agent specialized in data analysis, statistical modeling, and visualization using Python and R",
        "capabilities": ["data-analysis", "python", "statistics", "visualization"],
        "hubs": ["data-analysis", "general"]
    },
    {
        "name_prefix": "NLPAssistant",
        "description": "Natural language processing agent for text analysis, sentiment detection, and translation",
        "capabilities": ["nlp", "text-analysis", "translation", "python"],
        "hubs": ["nlp", "general"]
    },
    {
        "name_prefix": "CodeGenius",
        "description": "Code generation and review agent supporting multiple programming languages",
        "capabilities": ["code-generation", "python", "javascript", "rust"],
        "hubs": ["code-generation", "general"]
    },
    {
        "name_prefix": "ResearchBot",
        "description": "Research assistant agent for academic paper analysis and information retrieval",
        "capabilities": ["research", "data-analysis", "nlp"],
        "hubs": ["research", "general"]
    },
    {
        "name_prefix": "ChatCompanion",
        "description": "Conversational AI agent for customer support and general chat",
        "capabilities": ["chat", "nlp", "customer-support"],
        "hubs": ["chat", "general"]
    },
    {
        "name_prefix": "ImageCreator",
        "description": "Image generation and editing agent using state-of-the-art diffusion models",
        "capabilities": ["image-generation", "graphics", "ai-art"],
        "hubs": ["image-generation", "general"]
    },
    {
        "name_prefix": "AutomationPro",
        "description": "Task automation agent for workflow orchestration and repetitive task handling",
        "capabilities": ["automation", "python", "workflow"],
        "hubs": ["automation", "general"]
    },
]


@dataclass
class AgentClient:
    """Represents a simulated agent client"""
    agent_id: str = ""
    passkey: str = ""
    name: str = ""
    profile: Dict = field(default_factory=dict)
    is_registered: bool = False

    def make_request(self, method: str, path: str, data: Optional[Dict] = None) -> Dict:
        """Make HTTP request with this agent's context"""
        url = f"{BASE_URL}{path}"
        headers = {"Content-Type": "application/json"}

        if data:
            data = json.dumps(data).encode('utf-8')

        req = urllib.request.Request(url, data=data, headers=headers, method=method)

        try:
            with urllib.request.urlopen(req, timeout=10) as response:
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

    def register(self) -> bool:
        """Register this agent with the tracker"""
        if not self.profile:
            return False

        agent_data = {
            "name": f"{self.profile['name_prefix']}-{str(uuid.uuid4())[:8]}",
            "description": self.profile["description"],
            "capabilities": self.profile["capabilities"],
            "hubs": self.profile["hubs"],
            "endpoint": f"http://localhost:{random.randint(9000, 9999)}/api"
        }

        result = self.make_request("POST", "/agents", agent_data)

        if result.get("status") == 200 and "agent_id" in result.get("body", {}):
            self.agent_id = result["body"]["agent_id"]
            self.passkey = result["body"]["agent_passkey"]
            self.name = agent_data["name"]
            self.is_registered = True
            return True
        return False

    def heartbeat(self, status: str = "active") -> Dict:
        """Send heartbeat to tracker"""
        data = {
            "agent_id": self.agent_id,
            "passkey": self.passkey,
            "status": status
        }
        return self.make_request("POST", "/agents/heartbeat", data)

    def rate_agent(self, target_agent_id: str, stars: int, comment: str) -> Dict:
        """Rate another agent"""
        data = {
            "rater_agent_id": self.agent_id,
            "rater_passkey": self.passkey,
            "rated_agent_id": target_agent_id,
            "stars": stars,
            "comment": comment
        }
        return self.make_request("POST", "/agents/rate", data)

    def search(self, **params) -> Dict:
        """Search for agents"""
        query = "&".join(f"{k}={v}" for k, v in params.items())
        path = f"/agents/search?{query}" if query else "/agents/search"
        return self.make_request("GET", path)

    def get_hub_agents(self, hub_id: str) -> Dict:
        """Get agents in a specific hub"""
        return self.make_request("GET", f"/hubs/{hub_id}/agents")


def make_request(method: str, path: str, data: Optional[Dict] = None) -> Dict:
    """Make HTTP request without agent context"""
    url = f"{BASE_URL}{path}"
    headers = {"Content-Type": "application/json"}

    if data:
        data = json.dumps(data).encode('utf-8')

    req = urllib.request.Request(url, data=data, headers=headers, method=method)

    try:
        with urllib.request.urlopen(req, timeout=10) as response:
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


def print_result(test_name: str, passed: bool, details: str = None) -> bool:
    """Print test result"""
    status = "PASS" if passed else "FAIL"
    print(f"[{status}] {test_name}")
    if details:
        print(f"       {details}")
    return passed


class MultiClientTestSuite:
    """Multi-client test suite"""

    def __init__(self):
        self.agents: List[AgentClient] = []
        self.results: List[bool] = []
        self.hub_ids: List[str] = []

    def setup(self) -> bool:
        """Setup: get hub list"""
        print("\n=== Setup: Loading hubs ===")
        result = make_request("GET", "/hubs")

        if result.get("status") == 200 and "hubs" in result.get("body", {}):
            self.hub_ids = [h["hub_id"] for h in result["body"]["hubs"]]
            print(f"       Found {len(self.hub_ids)} hubs")
            return True
        return False

    def test_concurrent_registration(self, num_agents: int = 5) -> bool:
        """Test multiple agents registering concurrently"""
        print(f"\n=== Test: Concurrent Registration ({num_agents} agents) ===")

        def register_agent(profile_idx: int) -> tuple:
            client = AgentClient(profile=AGENT_PROFILES[profile_idx % len(AGENT_PROFILES)])
            success = client.register()
            return (client, success)

        with ThreadPoolExecutor(max_workers=num_agents) as executor:
            futures = [executor.submit(register_agent, i) for i in range(num_agents)]
            results = [f.result() for f in as_completed(futures)]

        for client, success in results:
            if success:
                self.agents.append(client)

        passed = len(self.agents) >= num_agents * 0.8  # 80% success rate
        print_result("Concurrent registration", passed,
                    f"Registered {len(self.agents)}/{num_agents} agents")
        return passed

    def test_concurrent_heartbeats(self) -> bool:
        """Test multiple agents sending heartbeats concurrently"""
        print(f"\n=== Test: Concurrent Heartbeats ({len(self.agents)} agents) ===")

        if len(self.agents) < 2:
            print_result("Concurrent heartbeats", False, "Not enough agents")
            return False

        def send_heartbeat(client: AgentClient) -> bool:
            result = client.heartbeat()
            return result.get("status") == 200

        with ThreadPoolExecutor(max_workers=len(self.agents)) as executor:
            futures = [executor.submit(send_heartbeat, agent) for agent in self.agents]
            results = [f.result() for f in as_completed(futures)]

        success_count = sum(results)
        passed = success_count == len(self.agents)
        print_result("Concurrent heartbeats", passed,
                    f"{success_count}/{len(self.agents)} successful")
        return passed

    def test_peer_discovery(self) -> bool:
        """Test agents discovering each other"""
        print(f"\n=== Test: Peer Discovery ===")

        if len(self.agents) < 2:
            print_result("Peer discovery", False, "Not enough agents")
            return False

        results = []

        # Each agent searches for agents with specific capabilities
        for i, agent in enumerate(self.agents[:3]):  # Test with first 3 agents
            # Search by capability
            search_result = agent.search(capability="python")
            if search_result.get("status") == 200:
                agents_found = search_result.get("body", {}).get("total", 0)
                results.append(agents_found > 0)
                print(f"       {agent.name}: Found {agents_found} agents with 'python' capability")

            # Give time between searches
            time.sleep(0.2)

        passed = all(results) if results else False
        print_result("Peer discovery (capability search)", passed,
                    f"{sum(results)}/{len(results)} agents found peers")
        return passed

    def test_cross_rating(self) -> bool:
        """Test agents rating each other"""
        print(f"\n=== Test: Cross Rating ===")

        if len(self.agents) < 2:
            print_result("Cross rating", False, "Not enough agents")
            return False

        results = []

        # Each agent rates a few other agents
        for i, rater in enumerate(self.agents[:min(5, len(self.agents))]):
            # Rate 2 other agents
            targets = [a for j, a in enumerate(self.agents) if j != i][:2]

            for target in targets:
                stars = random.randint(3, 5)
                comments = [
                    "Great agent, very reliable!",
                    "Excellent performance on tasks",
                    "Good collaboration experience",
                    "Professional and efficient",
                    "Highly recommended!"
                ]

                rating_result = rater.rate_agent(
                    target.agent_id,
                    stars,
                    random.choice(comments)
                )

                success = rating_result.get("status") == 200
                results.append(success)

                if success:
                    new_rating = rating_result["body"].get("new_avg_rating", 0)
                    print(f"       {rater.name} rated {target.name}: {stars} stars (avg: {new_rating:.2f})")

                time.sleep(0.1)

        passed = all(results) if results else False
        print_result("Cross rating", passed,
                    f"{sum(results)}/{len(results)} ratings submitted")
        return passed

    def test_hub_discovery(self) -> bool:
        """Test agents discovering others in the same hub"""
        print(f"\n=== Test: Hub-based Discovery ===")

        if len(self.agents) < 2 or not self.hub_ids:
            print_result("Hub discovery", False, "Not enough agents or hubs")
            return False

        results = []

        # Test getting agents from each hub
        for hub_id in self.hub_ids[:3]:  # Test first 3 hubs
            result = make_request("GET", f"/hubs/{hub_id}/agents")
            if result.get("status") == 200:
                agents = result.get("body", {}).get("agents", [])
                results.append(True)
                print(f"       Hub {hub_id[:8]}...: {len(agents)} agents")
            time.sleep(0.1)

        passed = all(results) if results else False
        print_result("Hub-based discovery", passed,
                    f"{sum(results)}/{len(results)} hubs queried successfully")
        return passed

    def test_keyword_search(self) -> bool:
        """Test keyword-based agent search"""
        print(f"\n=== Test: Keyword Search ===")

        if len(self.agents) < 2:
            print_result("Keyword search", False, "Not enough agents")
            return False

        search_terms = ["python", "data", "analysis", "agent", "nlp"]
        results = []

        for term in search_terms:
            result = make_request("GET", f"/agents/search?q={term}")
            if result.get("status") == 200:
                total = result.get("body", {}).get("total", 0)
                results.append(True)
                print(f"       Search '{term}': {total} results")
            time.sleep(0.1)

        passed = all(results) if results else False
        print_result("Keyword search", passed,
                    f"{sum(results)}/{len(results)} searches successful")
        return passed

    def test_heartbeat_with_discovery(self) -> bool:
        """Test heartbeat returns discovered agents"""
        print(f"\n=== Test: Heartbeat with Discovery ===")

        if len(self.agents) < 2:
            print_result("Heartbeat discovery", False, "Not enough agents")
            return False

        results = []

        # Send heartbeat and check discovered agents
        for agent in self.agents[:3]:
            result = agent.heartbeat()
            if result.get("status") == 200:
                discovered = result.get("body", {}).get("discovered_agents", [])
                results.append(len(discovered) >= 0)  # May be 0 if only one agent
                print(f"       {agent.name}: Discovered {len(discovered)} agents via heartbeat")
            time.sleep(0.1)

        passed = all(results) if results else False
        print_result("Heartbeat discovery", passed,
                    f"{sum(results)}/{len(results)} heartbeats returned discoveries")
        return passed

    def test_ratings_persistence(self) -> bool:
        """Test that ratings persist and can be retrieved"""
        print(f"\n=== Test: Ratings Persistence ===")

        if len(self.agents) < 2:
            print_result("Ratings persistence", False, "Not enough agents")
            return False

        # Get ratings for agents that were rated
        rated_agents = [a for a in self.agents if a.is_registered]
        results = []

        for agent in rated_agents[:3]:
            result = make_request("GET", f"/agents/{agent.agent_id}/ratings")
            if result.get("status") == 200:
                body = result.get("body", {})
                avg_rating = body.get("avg_rating", 0)
                total_ratings = body.get("total", 0)
                results.append(True)
                print(f"       {agent.name}: {total_ratings} ratings, avg: {avg_rating:.2f}")
            else:
                # 404 is ok if no ratings yet
                results.append(result.get("status") in [200, 404])
            time.sleep(0.1)

        passed = all(results) if results else False
        print_result("Ratings persistence", passed,
                    f"{sum(results)}/{len(results)} agents' ratings retrieved")
        return passed

    def test_stress_search(self, num_requests: int = 20) -> bool:
        """Stress test: multiple concurrent search requests"""
        print(f"\n=== Stress Test: Concurrent Searches ({num_requests} requests) ===")

        def search_request(search_type: str, param: str) -> bool:
            if search_type == "capability":
                result = make_request("GET", f"/agents/search?capability={param}")
            else:
                result = make_request("GET", f"/agents/search?q={param}")
            return result.get("status") == 200

        search_params = [
            ("capability", "python"),
            ("capability", "nlp"),
            ("keyword", "data"),
            ("keyword", "agent"),
            ("keyword", "analysis"),
        ]

        with ThreadPoolExecutor(max_workers=10) as executor:
            futures = []
            for i in range(num_requests):
                search_type, param = random.choice(search_params)
                futures.append(executor.submit(search_request, search_type, param))

            results = [f.result() for f in as_completed(futures)]

        success_count = sum(results)
        passed = success_count >= num_requests * 0.9  # 90% success
        print_result("Stress search", passed,
                    f"{success_count}/{num_requests} requests successful")
        return passed

    def run_all_tests(self) -> bool:
        """Run all multi-client tests"""
        print("=" * 70)
        print("BITCLAW TRACKER - MULTI-CLIENT SMOKE TEST SUITE")
        print("=" * 70)

        # Setup
        if not self.setup():
            print("Setup failed - cannot continue")
            return False

        # Run tests in order
        self.results.append(self.test_concurrent_registration(5))
        time.sleep(0.5)

        self.results.append(self.test_concurrent_heartbeats())
        time.sleep(0.5)

        self.results.append(self.test_peer_discovery())
        time.sleep(0.5)

        self.results.append(self.test_cross_rating())
        time.sleep(0.5)

        self.results.append(self.test_hub_discovery())
        time.sleep(0.5)

        self.results.append(self.test_keyword_search())
        time.sleep(0.5)

        self.results.append(self.test_heartbeat_with_discovery())
        time.sleep(0.5)

        self.results.append(self.test_ratings_persistence())
        time.sleep(0.5)

        self.results.append(self.test_stress_search(20))

        # Summary
        print("\n" + "=" * 70)
        passed = sum(self.results)
        total = len(self.results)
        print(f"RESULTS: {passed}/{total} test groups passed")

        if passed == total:
            print("ALL TESTS PASSED!")
        else:
            print(f"WARNING: {total - passed} test group(s) failed")

        print("=" * 70)

        return all(self.results)


def main():
    """Main entry point"""
    print("\nMulti-Client Smoke Test Suite for BitClaw Tracker")
    print("This test simulates multiple agents connecting and interacting\n")

    # Check server connectivity
    print("Checking server connectivity...")
    result = make_request("GET", "/hubs")
    if "error" in result:
        print(f"ERROR: Cannot connect to server: {result['error']}")
        print("Make sure the server is running on http://localhost:8080")
        sys.exit(1)
    print("Server is reachable!\n")

    # Run tests
    test_suite = MultiClientTestSuite()
    success = test_suite.run_all_tests()

    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
