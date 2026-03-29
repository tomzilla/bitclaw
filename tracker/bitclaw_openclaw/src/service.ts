import { spawn } from "node:child_process";
import * as path from "node:path";
import * as fs from "node:fs/promises";
import type {
  OpenClawPluginApi,
  OpenClawPluginService,
  OpenClawPluginServiceContext,
} from "./api.js";

/**
 * Arcadia Tracker Service
 *
 * Manages the arcadia-agent CLI backend process and maintains connection state.
 * This service spawns the Rust binary on startup and keeps it running for P2P connections.
 */

// Path to the arcadia-agent binary (relative to plugin directory or from PATH)
const ARCADIA_AGENT_BIN = process.env.ARCADIA_AGENT_BIN || "arcadia-agent";

export interface TrackerServiceState {
  connected: boolean;
  hubName?: string;
  clientId?: string;
  localAddress?: string;
  publicAddress?: string;
  upnpEnabled: boolean;
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
  };

  return {
    id: "arcadia-tracker",

    async start(ctx: OpenClawPluginServiceContext): Promise<void> {
      ctx.logger.info("arcadia tracker service starting");

      // Verify the arcadia-agent binary is available
      try {
        await verifyBinary();
        ctx.logger.info("arcadia-agent binary found");
      } catch (error) {
        ctx.logger.warn(
          `arcadia-agent binary not found: ${error instanceof Error ? error.message : String(error)}. ` +
          "Set ARCADIA_AGENT_BIN env var or install the binary.",
        );
      }

      ctx.logger.info("arcadia tracker service ready");
    },

    async stop(_ctx: OpenClawPluginServiceContext): Promise<void> {
      ctx.logger.info("arcadia tracker service stopping");
      state = { connected: false, upnpEnabled: false };
    },
  };
}

async function verifyBinary(): Promise<void> {
  // Check if binary exists in PATH or is an absolute path
  const bin = ARCADIA_AGENT_BIN;

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
 * Execute arcadia-agent CLI command and parse JSON output
 */
export async function runArcadiaCommand<T>(
  args: string[],
  logger: { info: (msg: string) => void; warn: (msg: string) => void; error: (msg: string) => void },
): Promise<T> {
  const { exec } = await import("node:child_process");
  const { promisify } = await import("node:util");
  const execAsync = promisify(exec);

  const cmd = `${ARCADIA_AGENT_BIN} ${args.join(" ")}`;
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
