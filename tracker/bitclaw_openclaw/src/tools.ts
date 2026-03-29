import { Type } from "@sinclair/typebox";
import type { OpenClawPluginApi, OpenClawPluginToolContext } from "./api.js";
import { runArcadiaCommand } from "./service.js";

/**
 * Arcadia Tracker Tools
 *
 * Provides agent-facing tools for interacting with the tracker network:
 * - list_hubs: List available tracker hubs
 * - register: Register as an agent with the tracker
 * - find_agent: Find agents by search query
 */

interface HubInfo {
  hub_id: string;
  name: string;
  description?: string;
  is_public: boolean;
}

interface RegisterResult {
  client_id: string;
  agent_name: string;
  description: string;
  local_address: string;
  public_address: string;
  upnp_enabled: boolean;
  hub_joined?: string;
}

interface AgentInfo {
  agent_id: string;
  name: string;
  description: string;
  capabilities: string[];
  ip_address?: string;
  port?: number;
  endpoint?: string;
  avg_rating?: string;
  total_ratings?: number;
}

interface ToolResult<T> {
  content: Array<{ type: "text"; text: string }>;
  details?: T;
}

export interface CreateTrackerToolsParams {
  api: OpenClawPluginApi;
  context: OpenClawPluginToolContext;
}

export function createTrackerTools(params: CreateTrackerToolsParams) {
  const { api, context } = params;
  const logger = api.logger;

  return {
    name: "arcadia_tracker",
    label: "Arcadia Tracker",
    description:
      "P2P agent coordination via arcadia tracker. Use to discover hubs, register as an agent, and find other agents for collaboration.",
    parameters: Type.Object({
      action: Type.String({
        description:
          "Action to perform: 'list_hubs', 'register', 'find_agents'",
      }),
      name: Type.Optional(
        Type.String({ description: "Agent name (for register action)" }),
      ),
      description: Type.Optional(
        Type.String({ description: "Agent description (for register action)" }),
      ),
      hub: Type.Optional(
        Type.String({
          description: "Hub name to join (for register action)",
        }),
      ),
      query: Type.Optional(
        Type.String({ description: "Search query to find agents (for find_agents action)" }),
      ),
      lan_mode: Type.Optional(
        Type.Boolean({ description: "Use LAN mode without UPnP port forwarding (default: false)" }),
      ),
    }),

    async execute(_id: string, toolParams: Record<string, unknown>): Promise<ToolResult<unknown>> {
      const action = toolParams.action as string;

      if (!action) {
        throw new Error("Missing required parameter: action");
      }

      try {
        switch (action) {
          case "list_hubs":
            return await handleListHubs();
          case "register":
            return await handleRegister(toolParams);
          case "find_agents":
            return await handleFindAgents(toolParams);
          default:
            throw new Error(
              `Unknown action: ${action}. Valid actions: list_hubs, register, find_agents`,
            );
        }
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        logger.error(`arcadia_tracker tool error: ${errorMsg}`);
        throw error;
      }
    },
  };
}

async function handleListHubs(): Promise<ToolResult<{ hubs: HubInfo[] }>> {
  const trackerUrl = process.env.ARCADIA_TRACKER_URL || "http://localhost:8000";
  const data = await runArcadiaCommand<{ hubs: HubInfo[] }>(
    ["list-hubs", "--tracker-url", trackerUrl],
    console,
  );

  const hubList = data.hubs || [];
  const text =
    hubList.length > 0
      ? `Found ${hubList.length} hub(s):\n${hubList.map((h) => `- ${h.name} (${h.hub_id})${h.description ? `: ${h.description}` : ""}`).join("\n")}`
      : "No hubs available";

  return {
    content: [{ type: "text", text }],
    details: { hubs: hubList },
  };
}

async function handleRegister(
  toolParams: Record<string, unknown>,
): Promise<ToolResult<RegisterResult>> {
  const name = toolParams.name as string;
  if (!name) {
    throw new Error("Missing required parameter: name");
  }

  const trackerUrl = process.env.ARCADIA_TRACKER_URL || "http://localhost:8000";
  const description = (toolParams.description as string) || `OpenClaw agent: ${name}`;
  const hub = toolParams.hub as string | undefined;
  const lanMode = toolParams.lan_mode as boolean;

  const args = [
    "register",
    "--tracker-url",
    trackerUrl,
    "--name",
    name,
    "--description",
    description,
  ];

  if (hub) {
    args.push("--hub", hub);
  }

  if (lanMode) {
    args.push("--lan-mode");
  }

  const data = await runArcadiaCommand<RegisterResult>(args, console);

  const text = `Registered agent "${data.agent_name}" (${data.client_id})\n` +
    `Description: ${data.description}\n` +
    `Local address: ${data.local_address}\n` +
    `Public address: ${data.public_address}\n` +
    (data.hub_joined ? `Hub joined: ${data.hub_joined}\n` : '') +
    `UPnP enabled: ${data.upnp_enabled}`;

  return {
    content: [{ type: "text", text }],
    details: data,
  };
}

async function handleFindAgents(
  toolParams: Record<string, unknown>,
): Promise<ToolResult<{ agents: AgentInfo[] }>> {
  const query = toolParams.query as string;

  if (!query) {
    throw new Error("Missing required parameter: query");
  }

  const trackerUrl = process.env.ARCADIA_TRACKER_URL || "http://localhost:8000";

  // Use the tracker's search API directly via curl
  const { exec } = await import("node:child_process");
  const { promisify } = await import("node:util");
  const execAsync = promisify(exec);

  try {
    const { stdout } = await execAsync(
      `curl -s -X POST "${trackerUrl}/api/v1/hubs/search" -H "Content-Type: application/json" -d '{"q":"${query}","limit":50}'`
    );

    const result = JSON.parse(stdout) as {
      query: string;
      hub_id?: string;
      agents: AgentInfo[];
      total: number;
    };

    const agentList = result.agents || [];
    const text =
      agentList.length > 0
        ? `Found ${agentList.length} agent(s) matching "${query}":\n` +
          agentList
            .map(
              (a) =>
                `- ${a.name} (${a.agent_id}): ${a.description}${a.ip_address ? ` @ ${a.ip_address}:${a.port}` : ""}`,
            )
            .join("\n")
        : `No agents found matching "${query}"`;

    return {
      content: [{ type: "text", text }],
      details: { agents: agentList },
    };
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to search agents: ${errorMsg}`);
  }
}
