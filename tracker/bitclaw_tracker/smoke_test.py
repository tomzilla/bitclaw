#!/usr/bin/env python3
"""
Smoke tests for BitClaw Tracker API
Tests all agent-related endpoints

Usage:
    1. Start the server: cargo run --package bitclaw_tracker
    2. Run tests: python3 smoke_test.py
"""

import json
import urllib.request
import urllib.error
import sys
import time
import uuid

BASE_URL = "http://localhost:8080/api/v1"

def make_request(method, path, data=None, headers=None):
    """Make HTTP request and return response"""
    url = f"{BASE_URL}{path}"
    if headers is None:
        headers = {"Content-Type": "application/json"}

    if data:
        data = json.dumps(data).encode('utf-8')

    req = urllib.request.Request(url, data=data, headers=headers, method=method)

    try:
        with urllib.request.urlopen(req) as response:
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
        return {"error": f"Connection failed: {e.reason}. Is the server running?"}
    except Exception as e:
        return {"error": str(e)}

def print_result(test_name, passed, details=None):
    """Print test result"""
    status = "PASS" if passed else "FAIL"
    print(f"[{status}] {test_name}")
    if details:
        print(f"       {details}")
    return passed

def test_list_hubs():
    """Test GET /api/v1/hubs - List all hubs"""
    print("\n=== Testing GET /api/v1/hubs ===")
    result = make_request("GET", "/hubs")

    if "error" in result:
        print_result("List hubs", False, f"Request failed: {result['error']}")
        return False

    passed = result["status"] == 200 and "hubs" in result.get("body", {})
    print_result("List hubs", passed, f"Status: {result['status']}, Hubs count: {len(result.get('body', {}).get('hubs', []))}")
    return passed

def test_register_agent():
    """Test POST /api/v1/agents - Register new agent"""
    print("\n=== Testing POST /api/v1/agents ===")

    agent_data = {
        "name": f"TestAgent-{str(uuid.uuid4())[:8]}",
        "description": "I am an AI agent specialized in natural language processing and data analysis",
        "capabilities": ["nlp", "data-analysis", "python"],
        "hubs": ["general", "nlp"],
        "endpoint": "http://localhost:9000/api"
    }

    result = make_request("POST", "/agents", agent_data)

    if "error" in result:
        print_result("Register agent", False, f"Request failed: {result['error']}")
        return None

    passed = result["status"] == 200 and "agent_id" in result.get("body", {})
    print_result("Register agent", passed, f"Status: {result['status']}")

    if passed:
        body = result["body"]
        print(f"       Agent ID: {body.get('agent_id')}")
        print(f"       Passkey: {body.get('agent_passkey', 'N/A')[:20]}...")
        print(f"       Interval: {body.get('interval')}s")
        return body

    return None

def test_heartbeat(agent_passkey, agent_id):
    """Test POST /api/v1/agents/heartbeat - Send heartbeat"""
    print("\n=== Testing POST /api/v1/agents/heartbeat ===")

    heartbeat_data = {
        "agent_id": agent_id,
        "passkey": agent_passkey
    }

    result = make_request("POST", "/agents/heartbeat", heartbeat_data)

    passed = result["status"] == 200
    print_result("Heartbeat", passed, f"Status: {result['status']}")
    return passed

def test_search_agents():
    """Test GET /api/v1/agents/search - Search agents"""
    print("\n=== Testing GET /api/v1/agents/search ===")

    # Test search with capability filter
    result = make_request("GET", "/agents/search?capability=nlp")

    if "error" in result:
        print_result("Search agents (capability)", False, f"Request failed: {result['error']}")
        return False

    passed = result["status"] == 200 and "agents" in result.get("body", {})
    body = result.get("body", {})
    print_result("Search agents (capability)", passed,
                 f"Status: {result['status']}, Found: {body.get('total', 0)} agents")

    # Test keyword search
    result2 = make_request("GET", "/agents/search?q=natural+language")
    passed2 = result2["status"] == 200
    print_result("Search agents (keyword)", passed2,
                 f"Status: {result2['status']}, Found: {result2.get('body', {}).get('total', 0)} agents")

    return passed and passed2

def test_hub_search():
    """Test POST /api/v1/hubs/search - Search agents in hub"""
    print("\n=== Testing POST /api/v1/hubs/search ===")

    search_data = {
        "q": "nlp",
        "limit": 10
    }

    result = make_request("POST", "/hubs/search", search_data)

    passed = result["status"] == 200 and "agents" in result.get("body", {})
    body = result.get("body", {})
    print_result("Hub search", passed,
                 f"Status: {result['status']}, Found: {body.get('total', 0)} agents")
    return passed

def test_get_hub_agents(hub_id):
    """Test GET /api/v1/hubs/{hub_id}/agents - Get agents in hub"""
    print(f"\n=== Testing GET /api/v1/hubs/{hub_id}/agents ===")

    result = make_request("GET", f"/hubs/{hub_id}/agents")

    passed = result["status"] in [200, 404]  # 404 is ok if hub is empty
    print_result("Get hub agents", passed, f"Status: {result['status']}")
    return passed

def test_rate_agent(agent_id, rater_agent_id, rater_passkey):
    """Test POST /api/v1/agents/rate - Rate an agent"""
    print("\n=== Testing POST /api/v1/agents/rate ===")

    rating_data = {
        "rater_agent_id": rater_agent_id,
        "rater_passkey": rater_passkey,
        "rated_agent_id": agent_id,
        "stars": 5,
        "comment": "Excellent agent, very helpful!"
    }

    result = make_request("POST", "/agents/rate", rating_data)

    passed = result["status"] == 200
    print_result("Rate agent", passed, f"Status: {result['status']}")
    return passed

def test_get_ratings(agent_id):
    """Test GET /api/v1/agents/{agent_id}/ratings - Get ratings for an agent"""
    print(f"\n=== Testing GET /api/v1/agents/{agent_id}/ratings ===")

    result = make_request("GET", f"/agents/{agent_id}/ratings")

    passed = result["status"] == 200
    body = result.get("body", {})
    print_result("Get ratings", passed,
                 f"Status: {result['status']}, Ratings: {body.get('total', 0)}, Avg: {body.get('avg_rating', 'N/A')}")
    return passed

def run_smoke_tests():
    """Run all smoke tests"""
    print("=" * 60)
    print("BITCLAW TRACKER API SMOKE TESTS")
    print("=" * 60)

    results = []

    # Test 1: List hubs (should work with default data)
    results.append(test_list_hubs())

    # Test 2: Register first agent
    agent1 = test_register_agent()
    if agent1:
        agent1_id = agent1["agent_id"]
        agent1_passkey = agent1["agent_passkey"]

        # Test 3: Heartbeat for first agent
        results.append(test_heartbeat(agent1_passkey, agent1_id))

        # Wait a moment for agent to be indexed
        time.sleep(0.5)

        # Test 4: Search agents
        results.append(test_search_agents())

        # Test 5: Hub search
        results.append(test_hub_search())

        # Test 6: Get hub agents (use first hub from list)
        hubs_result = make_request("GET", "/hubs")
        if hubs_result.get("body", {}).get("hubs"):
            hub_id = hubs_result["body"]["hubs"][0]["hub_id"]
            results.append(test_get_hub_agents(hub_id))

        # Register second agent to test rating
        print("\n--- Registering second agent for rating test ---")
        agent2 = test_register_agent()
        if agent2:
            agent2_id = agent2["agent_id"]
            agent2_passkey = agent2["agent_passkey"]

            # Wait for agent to be indexed
            time.sleep(0.5)

            # Test 7: Rate first agent (from second agent)
            results.append(test_rate_agent(agent1_id, agent2_id, agent2_passkey))

            # Test 8: Get ratings for first agent
            results.append(test_get_ratings(agent1_id))
    else:
        print("Skipping dependent tests - first agent registration failed")

    # Summary
    print("\n" + "=" * 60)
    passed = sum(results)
    total = len(results)
    print(f"RESULTS: {passed}/{total} tests passed")
    print("=" * 60)

    return all(results) if results else False

if __name__ == "__main__":
    success = run_smoke_tests()
    sys.exit(0 if success else 1)
