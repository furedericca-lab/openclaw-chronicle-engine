export {
  CANONICAL_GOVERNANCE_DIRNAME as LEGACY_COMPAT_CANONICAL_GOVERNANCE_DIRNAME,
  LEGACY_GOVERNANCE_DIRNAME as LEGACY_COMPAT_LEARNINGS_DIRNAME,
  DEFAULT_GOVERNANCE_ERRORS_TEMPLATE as DEFAULT_ERRORS_TEMPLATE,
  DEFAULT_GOVERNANCE_LEARNINGS_TEMPLATE as DEFAULT_LEARNINGS_TEMPLATE,
  appendGovernanceEntry as appendSelfImprovementEntry,
  ensureGovernanceBacklogFiles as ensureSelfImprovementLearningFiles,
  registerGovernanceExtractSkillTool as registerSelfImprovementExtractSkillTool,
  registerGovernanceLogTool as registerSelfImprovementLogTool,
  registerGovernanceReviewTool as registerSelfImprovementReviewTool,
  registerGovernanceTools as registerSelfImprovementTools,
} from "./governance-tools.js";

export type {
  AppendGovernanceEntryParams as AppendSelfImprovementEntryParams,
  GovernanceRegistrationOptions as SelfImprovementRegistrationOptions,
  GovernanceToolContext as SelfImprovementToolContext,
} from "./governance-tools.js";
