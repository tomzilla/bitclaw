import { spawn, ChildProcess } from "node:child_process";
import * as path from "node:path";
import * as fs from "node:fs/promises";
import type {
  OpenClawPluginApi,
  OpenClawPluginService,
  OpenClawPluginServiceContext,
} from "./api.js";

/**
 * BitClaw Tracker Service
 *
 * Manages the bitclaw-agent CLI backend process and maintains connection state.
 * This service spawns the Rust binary on startup and keeps it running for P2P connections.
 * It also listens for incoming messages and forwards them to OpenClaw.
 */

// Path to the bitclaw-agent binary (relative to plugin directory or from PATH)
const BITCLAW_AGENT_BIN = process.env.BITCLAW_AGENT_BIN || "bitclaw-agent";

export interface TrackerServiceState {
  connected: boolean;
  hubName?: string;
  clientId?: string;
  localAddress?: string;
  publicAddress?: string;
  upnpEnabled: boolean;
  listenerProcess: ChildProcess | null;
  messageCallback: ((msg: IncomingMessage) => void) | null;
}

export interface IncomingMessage {
  type: string;
  from: string;
  content: {
    type: "Text" | "Json" | "Binary";
    [key: string]: unknown;
  };
  timestamp: string;
}

export interface CreateTrackerServiceParams {
  pluginConfig?: unknown;
}

export function createTrackerService(
  params: CreateTrackerServiceParams = {},
): OpenClawPluginService {
  let state: TrackerServiceState = {
    connected: false,
    upnpEnabled: false,
    listenerProcess: null,
    messageCallback: null,
  };

  return {
    id: "bitclaw-tracker",

    async start(ctx: OpenClawPluginServiceContext): Promise<void> {
      ctx.logger.info("bitclaw tracker service starting");

      // Verify the bitclaw-agent binary is available
      try {
        await verifyBinary();
        ctx.logger.info("bitclaw-agent binary found");
      } catch (error) {
        ctx.logger.warn(
          `bitclaw-agent binary not found: ${error instanceof Error ? error.message : String(error)}. ` +
          "Set BITCLAW_AGENT_BIN env var or install the binary.",
        );
      }

      ctx.logger.info("bitclaw tracker service ready");
    },

    async stop(_ctx: OpenClawPluginServiceContext): Promise<void> {
      ctx.logger.info("bitclaw tracker service stopping");

      // Kill listener process if running
      if (state.listenerProcess) {
        state.listenerProcess.kill("SIGTERM");
        state.listenerProcess = null;
      }

      state = {
        connected: false,
        upnpEnabled: false,
        listenerProcess: null,
        messageCallback: null,
      };
    },
  };
}

async function verifyBinary(): Promise<void> {
  // Check if binary exists in PATH or is an absolute path
  const bin = BITCLAW_AGENT_BIN;

  if (path.isAbsolute(bin)) {
    await fs.access(bin);
    return;
  }

  // Check if it's in PATH using which/where
  const { exec } = await import("node:child_process");
  const { promisify } = await import("node:util");
  const execAsync = promisify(exec);

  try {
    const cmd = process.platform === "win32" ? `where ${bin}` : `which ${bin}`;
    await execAsync(cmd);
  } catch {
    throw new Error(`Binary '${bin}' not found in PATH`);
  }
}

/**
 * Execute bitclaw-agent CLI command and parse JSON output
 */
export async function runBitclawCommand<T>(
  args: string[],
  logger: { info: (msg: string) => void; warn: (msg: string) => void; error: (msg: string) => void },
): Promise<T> {
  const { exec } = await import("node:child_process");
  const { promisify } = await import("node:util");
  const execAsync = promisify(exec);

  const cmd = `${BITCLAW_AGENT_BIN} ${args.join(" ")}`;
  logger.info(`Running: ${cmd}`);

  try {
    const { stdout, stderr } = await execAsync(cmd, {
      encoding: "utf-8",
      maxBuffer: 10 * 1024 * 1024, // 10MB buffer
    });

    // Parse JSON output
    const output = stdout.trim();
    logger.debug(`Command output: ${output}`);

    const result = JSON.parse(output) as { success: boolean; data?: T; error?: string };

    if (!result.success) {
      throw new Error(result.error || "Command failed");
    }

    if (result.data === undefined) {
      throw new Error("Command returned no data");
    }

    return result.data;
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error);
    logger.error(`Command failed: ${errorMsg}`);
    throw error;
  }
}

/**
 * Start the bitclaw-agent listen process and stream incoming messages
 */
export async function startMessageListener(
  trackerUrl: string,
  name: string,
  hub: string,
  onMessage: (msg: IncomingMessage) => void,
  logger: { info: (msg: string) => void; warn: (msg: string) => void; error: (msg: string) => void },
): Promise<{ stop: () => void }> {
  const args = [
    "listen",
    "--tracker-url", trackerUrl,
    "--name", name,
    "--hub", hub,
    "--lan-mode",
  ];

  logger.info(`Starting listener: ${BITCLAW_AGENT_BIN} ${args.join(" ")}`);

  const listener = spawn(BITCLAW_AGENT_BIN, args, {
    stdio: ["ignore", "pipe", "pipe"],
  });

  let isStopped = false;

  listener.stdout.on("data", (data: Buffer) => {
    const lines = data.toString().split("\n").filter(line => line.trim());

    for (const line of lines) {
      try {
        const msg = JSON.parse(line) as IncomingMessage;
        logger.info(`Received message from ${msg.from}`);
        onMessage(msg);
      } catch (error) {
        logger.warn(`Failed to parse JSONL line: ${line}`);
      }
    }
  });

  listener.stderr.on("data", (data: Buffer) => {
    const stderr = data.toString();
    // Filter out status messages, only log errors
    if (stderr.includes("ERROR") || stderr.includes("error")) {
      logger.error(stderr);
    } else {
      logger.info(`[listener] ${stderr.trim()}`);
    }
  });

  listener.on("error", (error) => {
    logger.error(`Listener process error: ${error.message}`);
  });

  listener.on("exit", (code) => {
    if (!isStopped) {
      logger.warn(`Listener process exited with code ${code}`);
    } else {
      logger.info("Listener stopped");
    }
  });

  const stop = () => {
    isStopped = true;
    if (listener.pid) {
      process.kill(listener.pid, "SIGTERM");
    }
  };

  return { stop };
}
