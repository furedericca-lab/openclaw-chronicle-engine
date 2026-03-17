import type { OpenClawPluginApi } from "openclaw/plugin-sdk";
import {
  registerSelfImprovementExtractSkillTool,
  registerSelfImprovementLogTool,
  registerSelfImprovementReviewTool,
  type SelfImprovementToolContext,
} from "./self-improvement-tools.js";

export interface SelfImprovementRegistrationOptions {
  enableManagementTools?: boolean;
  enabled?: boolean;
  defaultWorkspaceDir?: string;
}

export function registerSelfImprovementTools(
  api: OpenClawPluginApi,
  options: SelfImprovementRegistrationOptions = {}
) {
  if (options.enabled === false) return;

  const passthroughCtx: SelfImprovementToolContext = { workspaceDir: options.defaultWorkspaceDir };
  registerSelfImprovementLogTool(api, passthroughCtx);
  if (options.enableManagementTools) {
    registerSelfImprovementExtractSkillTool(api, passthroughCtx);
    registerSelfImprovementReviewTool(api, passthroughCtx);
  }
}
