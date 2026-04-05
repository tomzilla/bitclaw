import { Type } from "@sinclair/typebox";
import type { OpenClawPluginApi, OpenClawPluginToolContext } from "./api.js";
import { runBitclawCommand, startMessageListener, IncomingMessage } from "./service.js";

/**
 * BitClaw Tracker Tools
 *
 * Provides agent-facing tools for interacting with the tracker network:
 * - list_hubs: List available tracker hubs
 * - register: Register as an agent with the tracker
 * - find_agent: Find agents by search query
 * - listen: Start listening for incoming P2P messages
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

// Global listener state
let activeListener: { stop: () => void; api?: OpenClawPluginApi } | null = null;
let isListening = false;
const messageBuffer: IncomingMessage[] = [];
const MAX_BUFFER_SIZE = 100;
let autoForwardEnabled = true;

export function createTrackerTools(params: CreateTrackerToolsParams) {
  const { api, context } = params;
  const logger = api.logger;

  return {
    name: "bitclaw_tracker",
    label: "BitClaw Tracker",
    description:
      "P2P agent coordination via bitclaw tracker. Use to discover hubs, register as an agent, find other agents, and listen for incoming messages.",
    parameters: Type.Object({
      action: Type.String({
        description:
          "Action to perform: 'list_hubs' (list hubs), 'register' (register agent), 'find_agents' (search agents), 'listen' (start message listener), 'stop_listen' (stop listener), 'get_messages' (retrieve buffered messages), 'send_message' (send P2P message to agent)",
      }),
      name: Type.Optional(
        Type.String({ description: "Agent name (for register/listen actions)" }),
      ),
      description: Type.Optional(
        Type.String({ description: "Agent description (for register action)" }),
      ),
      hub: Type.Optional(
        Type.String({
          description: "Hub name to join (for register/listen actions)",
        }),
      ),
      query: Type.Optional(
        Type.String({ description: "Search query to find agents (for find_agents action)" }),
      ),
      lan_mode: Type.Optional(
        Type.Boolean({ description: "Use LAN mode without UPnP port forwarding (default: false)" }),
      ),
      auto_listen: Type.Optional(
        Type.Boolean({ description: "Start listening for messages after registration (default: false)" }),
      ),
      // send_message parameters
      target_ip: Type.Optional(
        Type.String({ description: "Target agent IP for sending message (default: 127.0.0.1)" }),
      ),
      target_port: Type.Optional(
        Type.Number({ description: "Target agent port for sending message (required for send_message)" }),
      ),
      message: Type.Optional(
        Type.String({ description: "Message content to send (for send_message action)" }),
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
            return await handleRegister(toolParams, api);
          case "find_agents":
            return await handleFindAgents(toolParams);
          case "listen":
            return await handleListen(toolParams, api);
          case "stop_listen":
            return await handleStopListen();
          case "get_messages":
            return await handleGetMessages();
          case "send_message":
            return await handleSendMessage(toolParams);
          default:
            throw new Error(
              `Unknown action: ${action}. Valid actions: list_hubs, register, find_agents, listen, stop_listen, get_messages, send_message`,
            );
        }
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        logger.error(`bitclaw_tracker tool error: ${errorMsg}`);
        throw error;
      }
    },
  };
}

async function handleListHubs(): Promise<ToolResult<{ hubs: HubInfo[] }>> {
  const trackerUrl = process.env.ARCADIA_TRACKER_URL || "http://localhost:8000";
  const data = await runBitclawCommand<{ hubs: HubInfo[] }>(
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
  api: OpenClawPluginApi,
): Promise<ToolResult<RegisterResult>> {
  const name = toolParams.name as string;
  if (!name) {
    throw new Error("Missing required parameter: name");
  }

  const trackerUrl = process.env.ARCADIA_TRACKER_URL || "http://localhost:8000";
  const description = (toolParams.description as string) || `OpenClaw agent: ${name}`;
  const hub = toolParams.hub as string | undefined;
  const lanMode = toolParams.lan_mode as boolean;
  const autoListen = toolParams.auto_listen as boolean | undefined;

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

  const data = await runBitclawCommand<RegisterResult>(args, console);

  let text = `Registered agent "${data.agent_name}" (${data.client_id})\n` +
    `Description: ${data.description}\n` +
    `Local address: ${data.local_address}\n` +
    `Public address: ${data.public_address}\n` +
    (data.hub_joined ? `Hub joined: ${data.hub_joined}\n` : '') +
    `UPnP enabled: ${data.upnp_enabled}`;

  // Auto-start listener if requested
  if (autoListen) {
    const onMessage = (msg: IncomingMessage) => {
      messageBuffer.push(msg);
      while (messageBuffer.length > MAX_BUFFER_SIZE) {
        messageBuffer.shift();
      }
      console.info(`Received message from ${msg.from}`);

      if (autoForwardEnabled) {
        const contentText = msg.content.type === "Text" || msg.content.type === "Json"
          ? (msg.content as any).text || (msg.content as any).json || JSON.stringify(msg.content)
          : `Binary (${(msg.content as any).length || 'unknown'} bytes)`;

        const formattedText = `📨 **Message from** \`${msg.from.slice(0, 8)}...\` **at** ${new Date(msg.timestamp).toLocaleTimeString()}\n\n${contentText}`;

        api.context.assistantMessage({
          role: "assistant",
          content: formattedText,
        });
      }
    };

    const listener = await startMessageListener(
      trackerUrl,
      name,
      hub || "general",
      onMessage,
      console,
    );

    activeListener = listener;
    activeListener.api = api;
    isListening = true;

    text += `\n\n✅ **Auto-listen enabled** - Listening for incoming messages on hub "${hub || 'general'}"\nMessages will auto-forward to this conversation.`;
  }

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
      `curl -s "${trackerUrl}/api/v1/agents/search?q=${encodeURIComponent(query)}"`
    );

    const result = JSON.parse(stdout) as {
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

async function handleListen(
  toolParams: Record<string, unknown>,
  api: OpenClawPluginApi,
): Promise<ToolResult<{ listening: boolean; hub?: string }>> {
  if (isListening) {
    return {
      content: [{ type: "text", text: "Already listening for messages. Use 'stop_listen' to stop." }],
      details: { listening: true },
    };
  }

  const name = toolParams.name as string || "openclaw-agent";
  const hub = toolParams.hub as string || "general";
  const trackerUrl = process.env.ARCADIA_TRACKER_URL || "http://localhost:8000";

  // Create message handler that stores messages and auto-forwards to conversation
  const onMessage = (msg: IncomingMessage) => {
    messageBuffer.push(msg);
    // Trim buffer if too large
    while (messageBuffer.length > MAX_BUFFER_SIZE) {
      messageBuffer.shift();
    }
    logger.info(`Received message from ${msg.from}`);

    // Auto-forward to conversation
    if (autoForwardEnabled) {
      const contentText = msg.content.type === "Text" || msg.content.type === "Json"
        ? (msg.content as any).text || (msg.content as any).json || JSON.stringify(msg.content)
        : `Binary (${(msg.content as any).length || 'unknown'} bytes)`;

      const formattedText = `📨 **Message from** \`${msg.from.slice(0, 8)}...\` **at** ${new Date(msg.timestamp).toLocaleTimeString()}\n\n${contentText}`;

      api.context.assistantMessage({
        role: "assistant",
        content: formattedText,
      });
    }
  };

  try {
    const listener = await startMessageListener(
      trackerUrl,
      name,
      hub,
      onMessage,
      console,
    );

    activeListener = listener;
    activeListener.api = api;
    isListening = true;

    return {
      content: [{
        type: "text",
        text: `✅ Started listening for incoming messages\n\nTracker: ${trackerUrl}\nHub: ${hub}\nName: ${name}\n\n**Auto-forward enabled** - Messages will appear in the conversation as they arrive.\n\nMessages are also buffered and can be retrieved with action 'get_messages'.\n\nUse action 'stop_listen' to stop listening.`,
      }],
      details: { listening: true, hub },
    };
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to start listener: ${errorMsg}`);
  }
}

async function handleGetMessages(): Promise<ToolResult<{ messages: IncomingMessage[]; count: number }>> {
  const messages = [...messageBuffer];

  if (messages.length === 0) {
    return {
      content: [{ type: "text", text: "No messages received yet." }],
      details: { messages: [], count: 0 },
    };
  }

  // Clear buffer after retrieving
  messageBuffer.length = 0;

  const text = `Received ${messages.length} message(s):\n\n` +
    messages.map((msg, i) =>
      `[${i + 1}] From: ${msg.from.slice(0, 8)}... at ${new Date(msg.timestamp).toLocaleTimeString()}\n` +
      `    Type: ${msg.content.type}\n` +
      `    Content: ${msg.content.type === "Text" ? (msg.content as any).text || JSON.stringify(msg.content) : JSON.stringify(msg.content)}`
    ).join("\n\n");

  return {
    content: [{ type: "text", text }],
    details: { messages, count: messages.length },
  };
}

async function handleStopListen(): Promise<ToolResult<{ listening: boolean }>> {
  if (!isListening || !activeListener) {
    return {
      content: [{ type: "text", text: "Not currently listening." }],
      details: { listening: false },
    };
  }

  activeListener.stop();
  activeListener = null;
  isListening = false;

  return {
    content: [{ type: "text", text: "✅ Stopped listening for messages." }],
    details: { listening: false },
  };
}

async function handleSendMessage(toolParams: Record<string, unknown>): Promise<ToolResult<{ sent: boolean; target: string }>> {
  const targetIp = (toolParams.target_ip as string) || "127.0.0.1";
  const targetPort = toolParams.target_port as number;
  const message = (toolParams.message as string) || "Hello from BitClaw!";

  if (!targetPort) {
    throw new Error("Missing required parameter: target_port");
  }

  const { exec } = await import("node:child_process");
  const { promisify } = await import("node:util");
  const execAsync = promisify(exec);

  // Derive bitclaw-sender path from BITCLAW_AGENT_BIN or use default
  const agentBin = process.env.BITCLAW_AGENT_BIN
    ? process.env.BITCLAW_AGENT_BIN.replace("bitclaw-agent", "bitclaw-sender")
    : "/Users/tomwu/bitagents/target/debug/bitclaw-sender";

  try {
    const { stdout, stderr } = await execAsync(
      `${agentBin} --target-ip ${targetIp} --target-port ${targetPort} --message "${message.replace(/"/g, '\\"')}"`,
      { encoding: "utf-8" }
    );

    const output = stdout + stderr;
    const peerMatch = output.match(/Connected to peer: ([\w-]+)/);
    const peerId = peerMatch ? peerMatch[1] : "unknown";

    return {
      content: [{
        type: "text",
        text: `✅ Message sent successfully!\n\nTo: ${targetIp}:${targetPort}\nPeer ID: ${peerId}\nMessage: "${message}"`
      }],
      details: { sent: true, target: `${targetIp}:${targetPort}` },
    };
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to send message: ${errorMsg}`);
  }
}
