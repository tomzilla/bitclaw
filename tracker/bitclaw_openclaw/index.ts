import { definePluginEntry, type AnyAgentTool, type OpenClawPluginApi } from "./api.js";
import { createTrackerService } from "./src/service.js";
import { createTrackerTools } from "./src/tools.js";

export default definePluginEntry({
  id: "arcadia",
  name: "Arcadia Tracker",
  description: "P2P agent coordination via arcadia tracker - enables distributed multi-agent workflows",
  register(api: OpenClawPluginApi) {
    // Register the tracker service that manages the arcadia-agent CLI backend
    api.registerService(createTrackerService(api));

    // Register tools for agent interaction with the tracker network
    api.registerTool((ctx) => createTrackerTools({ api, context: ctx }), {
      name: "arcadia_tracker",
    });
  },
});
