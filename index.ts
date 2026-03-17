/**
 * Chronicle Engine Plugin
 * Remote-backend-authoritative memory plugin with local context orchestration.
 */

import type { OpenClawPluginApi } from "openclaw/plugin-sdk";
import { homedir } from "node:os";
import { join } from "node:path";
import { readFile } from "node:fs/promises";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";

import { registerRemoteMemoryTools } from "./src/backend-tools.js";
import { ensureSelfImprovementLearningFiles } from "./src/self-improvement-files.js";
import { registerSelfImprovementTools } from "./src/self-improvement-registration.js";
import {
  createSessionExposureState,
  DEFAULT_SESSION_EXPOSURE_MAX_TRACKED_SESSIONS,
} from "./src/context/session-exposure-state.js";
import { createAutoRecallPlanner } from "./src/context/auto-recall-orchestrator.js";
import { createReflectionPromptPlanner } from "./src/context/reflection-prompt-planner.js";
import { createMemoryBackendClient } from "./src/backend-client/client.js";
import {
  resolveBackendCallContext,
  type RuntimeContextDefaults,
} from "./src/backend-client/runtime-context.js";
import type {
  BackendCaptureItem,
  MemoryBackendClient,
  ReflectionTrigger as BackendReflectionTrigger,
  ReflectionRecallMode as BackendReflectionRecallMode,
} from "./src/backend-client/types.js";

// ============================================================================
// Configuration & Types
// ============================================================================

interface PluginConfig {
  autoCapture?: boolean;
  autoRecall?: boolean;
  autoRecallMinLength?: number;
  autoRecallMinRepeated?: number;
  autoRecallTopK?: number;
  autoRecallSelectionMode?: AutoRecallSelectionMode;
  autoRecallCategories?: MemoryCategory[];
  autoRecallExcludeReflection?: boolean;
  autoRecallMaxAgeDays?: number;
  autoRecallMaxEntriesPerKey?: number;
  captureAssistant?: boolean;
  enableManagementTools?: boolean;
  sessionStrategy?: SessionStrategy;
  selfImprovement?: {
    enabled?: boolean;
    beforeResetNote?: boolean;
    skipSubagentBootstrap?: boolean;
    ensureLearningFiles?: boolean;
  };
  memoryReflection?: {
    enabled?: boolean;
    injectMode?: ReflectionInjectMode;
    messageCount?: number;
    errorReminderMaxEntries?: number;
    dedupeErrorSignals?: boolean;
    recall?: {
      mode?: ReflectionRecallMode;
      topK?: number;
      includeKinds?: ReflectionRecallKind[];
      maxAgeDays?: number;
      maxEntriesPerKey?: number;
      minRepeated?: number;
      minScore?: number;
      minPromptLength?: number;
    };
  };
  remoteBackend?: {
    enabled?: boolean;
    baseURL?: string;
    authToken?: string;
    timeoutMs?: number;
    maxRetries?: number;
    retryBackoffMs?: number;
  };
}
type SessionStrategy = "memoryReflection" | "systemSessionMemory" | "none";
type ReflectionInjectMode = "inheritance-only" | "inheritance+derived";
type ReflectionRecallMode = "fixed" | "dynamic";
type ReflectionRecallKind = "invariant" | "derived";
type AutoRecallSelectionMode = "mmr" | "setwise-v2";
type MemoryCategory = "preference" | "fact" | "decision" | "entity" | "other" | "reflection";

// ============================================================================
// Default Configuration
// ============================================================================

function getDefaultWorkspaceDir(): string {
  const home = homedir();
  return join(home, ".openclaw", "workspace");
}

function resolveWorkspaceDirFromContext(context: Record<string, unknown> | undefined): string {
  const runtimePath = typeof context?.workspaceDir === "string" ? context.workspaceDir.trim() : "";
  return runtimePath || getDefaultWorkspaceDir();
}

function resolveEnvVars(value: string): string {
  return value.replace(/\$\{([^}]+)\}/g, (_, envVar) => {
    const envValue = process.env[envVar];
    if (!envValue) {
      throw new Error(`Environment variable ${envVar} is not set`);
    }
    return envValue;
  });
}

function hasOwnKey(target: Record<string, unknown>, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(target, key);
}

function rejectRemovedConfigField(fieldPath: string, replacement?: string): never {
  throw new Error(
    replacement
      ? `${fieldPath} is no longer supported in 1.0.0-beta.0; use ${replacement}`
      : `${fieldPath} is no longer supported in 1.0.0-beta.0`
  );
}

function parsePositiveInt(value: unknown): number | undefined {
  if (typeof value === "number" && Number.isFinite(value) && value > 0) {
    return Math.floor(value);
  }
  if (typeof value === "string") {
    const s = value.trim();
    if (!s) return undefined;
    const resolved = resolveEnvVars(s);
    const n = Number(resolved);
    if (Number.isFinite(n) && n > 0) return Math.floor(n);
  }
  return undefined;
}

function parseNonNegativeNumber(value: unknown): number | undefined {
  if (typeof value === "number" && Number.isFinite(value) && value >= 0) {
    return value;
  }
  if (typeof value === "string") {
    const s = value.trim();
    if (!s) return undefined;
    const resolved = resolveEnvVars(s);
    const n = Number(resolved);
    if (Number.isFinite(n) && n >= 0) return n;
  }
  return undefined;
}

function parseMemoryCategories(value: unknown, fallback: MemoryCategory[]): MemoryCategory[] {
  if (!Array.isArray(value)) return [...fallback];
  const parsed = value
    .filter((item): item is string => typeof item === "string")
    .map((item) => item.trim())
    .filter((item): item is MemoryCategory =>
      item === "preference" ||
      item === "fact" ||
      item === "decision" ||
      item === "entity" ||
      item === "other" ||
      item === "reflection"
    );
  return parsed.length > 0 ? [...new Set(parsed)] : [...fallback];
}

function parseReflectionRecallKinds(value: unknown, fallback: ReflectionRecallKind[]): ReflectionRecallKind[] {
  if (!Array.isArray(value)) return [...fallback];
  const parsed = value
    .filter((item): item is string => typeof item === "string")
    .map((item) => item.trim())
    .filter((item): item is ReflectionRecallKind => item === "invariant" || item === "derived");
  return parsed.length > 0 ? [...new Set(parsed)] : [...fallback];
}

const DEFAULT_SELF_IMPROVEMENT_REMINDER = `## Self-Improvement Reminder

After completing tasks, evaluate if any learnings should be captured:

**Log when:**
- User corrects you -> .learnings/LEARNINGS.md
- Command/operation fails -> .learnings/ERRORS.md
- You discover your knowledge was wrong -> .learnings/LEARNINGS.md
- You find a better approach -> .learnings/LEARNINGS.md

**Promote when pattern is proven:**
- Behavioral patterns -> SOUL.md
- Workflow improvements -> AGENTS.md
- Tool gotchas -> TOOLS.md

Keep entries simple: date, title, what happened, what to do differently.`;

const SELF_IMPROVEMENT_NOTE_PREFIX = "/note self-improvement (before reset):";
const DEFAULT_REFLECTION_MESSAGE_COUNT = 120;
const DEFAULT_REFLECTION_ERROR_REMINDER_MAX_ENTRIES = 3;
const DEFAULT_REFLECTION_DEDUPE_ERROR_SIGNALS = true;
const DEFAULT_REFLECTION_ERROR_SCAN_MAX_CHARS = 8_000;
const DEFAULT_AUTO_RECALL_TOP_K = 3;
const DEFAULT_AUTO_RECALL_SELECTION_MODE: AutoRecallSelectionMode = "mmr";
const DEFAULT_AUTO_RECALL_EXCLUDE_REFLECTION = true;
const DEFAULT_AUTO_RECALL_MAX_AGE_DAYS = 30;
const DEFAULT_AUTO_RECALL_MAX_ENTRIES_PER_KEY = 10;
const DEFAULT_AUTO_RECALL_CATEGORIES: MemoryCategory[] = ["preference", "fact", "decision", "entity", "other"];
const DEFAULT_REFLECTION_RECALL_MODE: ReflectionRecallMode = "fixed";
const DEFAULT_REFLECTION_RECALL_TOP_K = 6;
const DEFAULT_REFLECTION_RECALL_INCLUDE_KINDS: ReflectionRecallKind[] = ["invariant"];
const DEFAULT_REFLECTION_RECALL_MAX_AGE_DAYS = 45;
const DEFAULT_REFLECTION_RECALL_MAX_ENTRIES_PER_KEY = 10;
const DEFAULT_REFLECTION_RECALL_MIN_REPEATED = 2;
const DEFAULT_REFLECTION_RECALL_MIN_SCORE = 0.18;
const DEFAULT_REFLECTION_RECALL_MIN_PROMPT_LENGTH = 8;
const DIAG_BUILD_TAG = "openclaw-chronicle-engine-diag-20260308-0058";

function buildSelfImprovementResetNote(params?: { openLoopsBlock?: string; derivedFocusBlock?: string }): string {
  const openLoopsBlock = typeof params?.openLoopsBlock === "string" ? params.openLoopsBlock : "";
  const derivedFocusBlock = typeof params?.derivedFocusBlock === "string" ? params.derivedFocusBlock : "";
  const base = [
    SELF_IMPROVEMENT_NOTE_PREFIX,
    "- If anything was learned/corrected, log it now:",
    "  - .learnings/LEARNINGS.md (corrections/best practices)",
    "  - .learnings/ERRORS.md (failures/root causes)",
    "- Distill reusable rules to AGENTS.md / SOUL.md / TOOLS.md.",
    "- If reusable across tasks, extract a new skill from the learning.",
  ];
  if (openLoopsBlock) {
    base.push("- Fresh run handoff:");
    base.push(openLoopsBlock);
  }
  if (derivedFocusBlock) {
    base.push("- Historical reflection-derived focus:");
    base.push(derivedFocusBlock);
  }
  base.push("- Then proceed with the new session.");
  return base.join("\n");
}

async function loadSelfImprovementReminderContent(workspaceDir?: string): Promise<string> {
  const baseDir = typeof workspaceDir === "string" && workspaceDir.trim().length ? workspaceDir.trim() : "";
  if (!baseDir) return DEFAULT_SELF_IMPROVEMENT_REMINDER;

  const reminderPath = join(baseDir, "SELF_IMPROVEMENT_REMINDER.md");
  try {
    const content = await readFile(reminderPath, "utf-8");
    const trimmed = content.trim();
    return trimmed.length ? trimmed : DEFAULT_SELF_IMPROVEMENT_REMINDER;
  } catch {
    return DEFAULT_SELF_IMPROVEMENT_REMINDER;
  }
}

function asNonEmptyString(value: unknown): string | undefined {
  if (typeof value !== "string") return undefined;
  const trimmed = value.trim();
  return trimmed.length ? trimmed : undefined;
}

function isInternalReflectionSessionKey(sessionKey: unknown): boolean {
  return typeof sessionKey === "string" && sessionKey.trim().startsWith("temp:memory-reflection");
}

function resolveRemoteBackendRuntimeDefaults(config: PluginConfig): RuntimeContextDefaults {
  void config;
  return {
    sessionIdPrefix: "memory-backend",
  };
}

function mergeContextSources(...sources: unknown[]): Record<string, unknown> {
  const merged: Record<string, unknown> = {};
  for (const source of sources) {
    if (!source || typeof source !== "object") continue;
    Object.assign(merged, source as Record<string, unknown>);
  }
  return merged;
}

function parseEventMessagesToCaptureItems(messages: unknown[]): BackendCaptureItem[] {
  const items: BackendCaptureItem[] = [];
  for (const msg of messages) {
    if (typeof msg === "string" && msg.trim()) {
      items.push({ role: "system", text: msg.trim() });
      continue;
    }
    if (!msg || typeof msg !== "object") continue;
    const obj = msg as Record<string, unknown>;
    const roleRaw = typeof obj.role === "string" ? obj.role.toLowerCase() : "system";
    const text = extractTextContent(obj.content);
    if (!text || !text.trim()) continue;
    items.push({
      role: roleRaw === "assistant" || roleRaw === "user" ? roleRaw : "system",
      text: text.trim(),
    });
  }
  return items;
}

function extractTextContent(content: unknown): string | null {
  if (!content) return null;
  if (typeof content === "string") return content;
  if (Array.isArray(content)) {
    const block = content.find(
      (c) => c && typeof c === "object" && (c as Record<string, unknown>).type === "text" && typeof (c as Record<string, unknown>).text === "string"
    ) as Record<string, unknown> | undefined;
    const text = block?.text;
    return typeof text === "string" ? text : null;
  }
  return null;
}

function sha256Hex(text: string): string {
  return createHash("sha256").update(text, "utf8").digest("hex");
}

function buildSessionTranscriptAppendIdempotencyKey(
  actor: { sessionId: string; sessionKey: string },
  items: BackendCaptureItem[]
): string {
  const digest = sha256Hex(
    JSON.stringify({
      sessionId: actor.sessionId,
      sessionKey: actor.sessionKey,
      items,
    })
  );
  return `session-transcript-append:${digest.slice(0, 48)}`;
}

// ============================================================================
// Capture & Category Detection (from old plugin)
// ============================================================================

const MEMORY_TRIGGERS = [
  /zapamatuj si|pamatuj|remember/i,
  /preferuji|radši|nechci|prefer/i,
  /rozhodli jsme|budeme používat/i,
  /\b(we )?decided\b|we'?ll use|we will use|switch(ed)? to|migrate(d)? to|going forward|from now on/i,
  /\+\d{10,}/,
  /[\w.-]+@[\w.-]+\.\w+/,
  /můj\s+\w+\s+je|je\s+můj/i,
  /my\s+\w+\s+is|is\s+my/i,
  /i (like|prefer|hate|love|want|need|care)/i,
  /always|never|important/i,
  // Chinese triggers (Traditional & Simplified)
  /記住|记住|記一下|记一下|別忘了|别忘了|備註|备注/,
  /偏好|喜好|喜歡|喜欢|討厭|讨厌|不喜歡|不喜欢|愛用|爱用|習慣|习惯/,
  /決定|决定|選擇了|选择了|改用|換成|换成|以後用|以后用/,
  /我的\S+是|叫我|稱呼|称呼/,
  /老是|講不聽|總是|总是|從不|从不|一直|每次都/,
  /重要|關鍵|关键|注意|千萬別|千万别/,
  /幫我|筆記|存檔|存起來|存一下|重點|原則|底線/,
];

const CAPTURE_EXCLUDE_PATTERNS = [
  // Memory management / meta-ops: do not store as long-term memory
  /\b(memory-pro|memory_store|memory_recall|memory_forget|memory_update)\b/i,
  /\bopenclaw\s+memory-pro\b/i,
  /\b(delete|remove|forget|purge|cleanup|clean up|clear)\b.*\b(memory|memories|entry|entries)\b/i,
  /\b(memory|memories)\b.*\b(delete|remove|forget|purge|cleanup|clean up|clear)\b/i,
  /\bhow do i\b.*\b(delete|remove|forget|purge|cleanup|clear)\b/i,
  /(删除|刪除|清理|清除).{0,12}(记忆|記憶|memory)/i,
];

export function shouldCapture(text: string): boolean {
  let s = text.trim();

  // Strip OpenClaw metadata headers (Conversation info or Sender)
  const metadataPattern = /^(Conversation info|Sender) \(untrusted metadata\):[\s\S]*?\n\s*\n/gim;
  s = s.replace(metadataPattern, "");

  // CJK characters carry more meaning per character, use lower minimum threshold
  const hasCJK = /[\u4e00-\u9fff\u3040-\u309f\u30a0-\u30ff\uac00-\ud7af]/.test(
    s,
  );
  const minLen = hasCJK ? 4 : 10;
  if (s.length < minLen || s.length > 500) {
    return false;
  }
  // Skip injected context from memory recall
  if (s.includes("<relevant-memories>")) {
    return false;
  }
  // Skip system-generated content
  if (s.startsWith("<") && s.includes("</")) {
    return false;
  }
  // Skip agent summary responses (contain markdown formatting)
  if (s.includes("**") && s.includes("\n-")) {
    return false;
  }
  // Skip emoji-heavy responses (likely agent output)
  const emojiCount = (s.match(/[\u{1F300}-\u{1F9FF}]/gu) || []).length;
  if (emojiCount > 3) {
    return false;
  }
  // Exclude obvious memory-management prompts
  if (CAPTURE_EXCLUDE_PATTERNS.some((r) => r.test(s))) return false;

  return MEMORY_TRIGGERS.some((r) => r.test(s));
}

export function detectCategory(
  text: string,
): "preference" | "fact" | "decision" | "entity" | "other" {
  const lower = text.toLowerCase();
  if (
    /prefer|radši|like|love|hate|want|偏好|喜歡|喜欢|討厭|讨厌|不喜歡|不喜欢|愛用|爱用|習慣|习惯/i.test(
      lower,
    )
  ) {
    return "preference";
  }
  if (
    /rozhodli|decided|we decided|will use|we will use|we'?ll use|switch(ed)? to|migrate(d)? to|going forward|from now on|budeme|決定|决定|選擇了|选择了|改用|換成|换成|以後用|以后用|規則|流程|SOP/i.test(
      lower,
    )
  ) {
    return "decision";
  }
  if (
    /\+\d{10,}|@[\w.-]+\.\w+|is called|jmenuje se|我的\S+是|叫我|稱呼|称呼/i.test(
      lower,
    )
  ) {
    return "entity";
  }
  if (
    /\b(is|are|has|have|je|má|jsou)\b|總是|总是|從不|从不|一直|每次都|老是/i.test(
      lower,
    )
  ) {
    return "fact";
  }
  return "other";
}

function sanitizeForContext(text: string): string {
  return text
    .replace(/[\r\n]+/g, " ")
    .replace(/<\/?[a-zA-Z][^>]*>/g, "")
    .replace(/</g, "\uFF1C")
    .replace(/>/g, "\uFF1E")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 300);
}

// ============================================================================
// Markdown Mirror (dual-write)
// ============================================================================

type AgentWorkspaceMap = Record<string, string>;

function resolveAgentWorkspaceMap(api: OpenClawPluginApi): AgentWorkspaceMap {
  const map: AgentWorkspaceMap = {};

  const assignFromConfigRoot = (root: any) => {
    const defaultWorkspace = typeof root?.agents?.defaults?.workspace === "string"
      ? root.agents.defaults.workspace
      : undefined;
    const agents = Array.isArray(root?.agents?.list)
      ? root.agents.list
      : [];

    for (const agent of agents) {
      if (!agent?.id) continue;
      const explicitWorkspace = typeof agent.workspace === "string" ? agent.workspace : undefined;
      const resolvedWorkspace = explicitWorkspace || defaultWorkspace;
      if (resolvedWorkspace) {
        map[String(agent.id)] = resolvedWorkspace;
      }
    }
  };

  // Try api.config first (runtime config)
  assignFromConfigRoot((api as any).config || {});

  // Fallback: read from openclaw.json (respect OPENCLAW_HOME if set)
  if (Object.keys(map).length === 0) {
    try {
      const openclawHome = process.env.OPENCLAW_HOME || join(homedir(), ".openclaw");
      const configPath = join(openclawHome, "openclaw.json");
      const raw = readFileSync(configPath, "utf8");
      const parsed = JSON.parse(raw);
      assignFromConfigRoot(parsed || {});
    } catch {
      /* silent */
    }
  }

  return map;
}

// ============================================================================
// Version
// ============================================================================

function getPluginVersion(): string {
  try {
    const pkgUrl = new URL("./package.json", import.meta.url);
    const pkg = JSON.parse(readFileSync(pkgUrl, "utf8")) as {
      version?: string;
    };
    return pkg.version || "unknown";
  } catch {
    return "unknown";
  }
}

const pluginVersion = getPluginVersion();

// ============================================================================
// Plugin Definition
// ============================================================================

const chronicleEnginePlugin = {
  id: "openclaw-chronicle-engine",
  name: "Chronicle Engine",
  description:
    "Enhanced long-term memory with remote-backend authority, hybrid recall orchestration, and management tools",
  kind: "memory" as const,

  register(api: OpenClawPluginApi) {
    // Parse and validate configuration
    const config = parsePluginConfig(api.pluginConfig);
    const remoteRuntimeDefaults = resolveRemoteBackendRuntimeDefaults(config);
    const memoryBackendClient: MemoryBackendClient = createMemoryBackendClient({
      baseUrl: config.remoteBackend?.baseURL || "",
      bearerToken: config.remoteBackend?.authToken || "",
      timeoutMs: config.remoteBackend?.timeoutMs || 10_000,
      maxRetries: Number(config.remoteBackend?.maxRetries ?? 1),
      retryBaseDelayMs: config.remoteBackend?.retryBackoffMs || 250,
      logger: api.logger,
    });
    const agentWorkspaceMap = resolveAgentWorkspaceMap(api);
    const sessionExposureState = createSessionExposureState({
      maxTrackedSessions: DEFAULT_SESSION_EXPOSURE_MAX_TRACKED_SESSIONS,
    });

    api.logger.info(
      `openclaw-chronicle-engine@${pluginVersion}: plugin registered ` +
      `(mode: remote-backend, authority: backend-owned)`,
    );
    api.logger.info(`openclaw-chronicle-engine: diagnostic build tag loaded (${DIAG_BUILD_TAG})`);
    api.logger.info(
      `openclaw-chronicle-engine: remote backend enabled (${config.remoteBackend?.baseURL || "(missing baseURL)"})`
    );
    // ========================================================================
    // Register Tools
    // ========================================================================

    registerRemoteMemoryTools(
      api,
      {
        backendClient: memoryBackendClient,
        runtimeDefaults: remoteRuntimeDefaults,
      },
      {
        enableManagementTools: config.enableManagementTools,
      }
    );
    registerSelfImprovementTools(api, {
      enabled: config.selfImprovement?.enabled !== false,
      enableManagementTools: config.enableManagementTools,
      defaultWorkspaceDir: getDefaultWorkspaceDir(),
    });

    // ========================================================================
    // Lifecycle Hooks
    // ========================================================================

    api.on("session_end", (_event, ctx) => {
      sessionExposureState.clearDynamicRecallForContext(ctx || {});
    }, { priority: 20 });

    const autoRecallPlanner = createAutoRecallPlanner(
      {
        enabled: config.autoRecall === true,
        minPromptLength: config.autoRecallMinLength,
        minRepeated: config.autoRecallMinRepeated,
        topK: config.autoRecallTopK ?? DEFAULT_AUTO_RECALL_TOP_K,
        selectionMode: config.autoRecallSelectionMode ?? DEFAULT_AUTO_RECALL_SELECTION_MODE,
        categories: config.autoRecallCategories,
        excludeReflection: config.autoRecallExcludeReflection === true,
        maxAgeDays: config.autoRecallMaxAgeDays,
        maxEntriesPerKey: config.autoRecallMaxEntriesPerKey,
      },
      {
        state: sessionExposureState.autoRecallState,
        recallGeneric: async (params) => {
          const resolved = resolveBackendCallContext(
            {
              userId: params.userId,
              agentId: params.agentId,
              sessionId: params.sessionId,
              sessionKey: params.sessionKey,
            },
            remoteRuntimeDefaults,
          );
          if (!resolved.hasPrincipalIdentity) {
            api.logger.warn(
              `openclaw-chronicle-engine: auto-recall skipped remote recall (missing runtime principal: ${resolved.missingPrincipalFields.join(", ")})`
            );
            return [];
          }
          return await memoryBackendClient.recallGeneric(resolved.context, {
            query: params.query,
            limit: params.limit,
            categories: params.categories,
            excludeReflection: params.excludeReflection,
            maxAgeDays: params.maxAgeDays,
            maxEntriesPerKey: params.maxEntriesPerKey,
          });
        },
        sanitizeForContext,
        logger: api.logger,
      }
    );

    // Auto-Recall: inject relevant memories before agent starts.
    // Default is OFF to prevent the model from accidentally echoing injected context.
    if (config.autoRecall === true) {
      api.on("before_agent_start", async (event, ctx) => {

        try {
          return await autoRecallPlanner.plan({
            prompt: event.prompt,
            agentId: ctx?.agentId,
            sessionId: ctx?.sessionId,
            sessionKey: typeof ctx?.sessionKey === "string" ? ctx.sessionKey : undefined,
            userId: typeof ctx?.userId === "string" ? ctx.userId : undefined,
          });
        } catch (err) {
          api.logger.warn(`openclaw-chronicle-engine: auto-recall failed: ${String(err)}`);
        }
      });
    }

    api.on("agent_end", async (event, ctx) => {
      if (!event.success || !Array.isArray(event.messages) || event.messages.length === 0) {
        return;
      }

      const transcriptItems = parseEventMessagesToCaptureItems(event.messages);
      if (transcriptItems.length === 0) {
        return;
      }

      const resolvedBackendCtx = resolveBackendCallContext(
        mergeContextSources(ctx, event),
        remoteRuntimeDefaults,
        {
          agentId: typeof ctx?.agentId === "string" ? ctx.agentId : undefined,
          sessionId: typeof ctx?.sessionId === "string" ? ctx.sessionId : undefined,
          sessionKey: typeof ctx?.sessionKey === "string" ? ctx.sessionKey : undefined,
        }
      );
      if (!resolvedBackendCtx.hasPrincipalIdentity) {
        api.logger.warn(
          `openclaw-chronicle-engine: transcript append blocked (missing runtime principal: ${resolvedBackendCtx.missingPrincipalFields.join(", ")})`
        );
        return;
      }

      try {
        const appendResult = await memoryBackendClient.appendSessionTranscript(
          resolvedBackendCtx.context,
          {
            items: transcriptItems.slice(0, 256),
            // Use a stable batch fingerprint so duplicate agent_end deliveries do not replay transcript rows.
            idempotencyKey: buildSessionTranscriptAppendIdempotencyKey(
              resolvedBackendCtx.context.actor,
              transcriptItems.slice(0, 256)
            ),
          }
        );
        api.logger.info(
          `openclaw-chronicle-engine: session transcript appended to remote backend (${transcriptItems.length} item(s), appended=${appendResult.appended})`
        );
      } catch (err) {
        api.logger.warn(`openclaw-chronicle-engine: session transcript append failed: ${String(err)}`);
      }

      if (config.autoCapture === false) {
        return;
      }

      try {
        const captureItems = transcriptItems
          .filter((item) => item.role === "user" || (config.captureAssistant === true && item.role === "assistant"))
          .slice(0, 64);
        if (captureItems.length === 0) {
          return;
        }
        const result = await memoryBackendClient.storeAutoCapture(resolvedBackendCtx.context, {
          items: captureItems,
        });
        api.logger.info(
          `openclaw-chronicle-engine: auto-capture forwarded to remote backend (${captureItems.length} item(s), mutations=${result.length})`
        );
      } catch (err) {
        api.logger.warn(`openclaw-chronicle-engine: capture failed: ${String(err)}`);
      }
    });

    // ========================================================================
    // Integrated Self-Improvement (inheritance + derived)
    // ========================================================================

    const COMMAND_HOOK_EVENT_MARKER_PREFIX = "__memoryLanceDbProCommandHandled__";
    const markCommandHookEventHandled = (event: unknown, marker: string): boolean => {
      if (!event || typeof event !== "object") return false;
      const target = event as Record<string, unknown>;
      if (target[marker] === true) return true;
      try {
        Object.defineProperty(target, marker, {
          value: true,
          enumerable: false,
          configurable: true,
          writable: true,
        });
      } catch {
        target[marker] = true;
      }
      return false;
    };

    const registerDurableCommandHook = (
      eventName: "command:new" | "command:reset",
      handler: (event: any) => Promise<unknown> | unknown,
      options: { name: string; description: string },
      markerSuffix: string,
    ) => {
      const marker = `${COMMAND_HOOK_EVENT_MARKER_PREFIX}${markerSuffix}:${eventName}`;
      const wrapped = async (event: any) => {
        if (markCommandHookEventHandled(event, marker)) return;
        return await handler(event);
      };

      let registeredViaEventBus = false;
      let registeredViaInternalHook = false;

      const onFn = (api as any).on;
      if (typeof onFn === "function") {
        try {
          onFn.call(api, eventName, wrapped, { priority: 12 });
          registeredViaEventBus = true;
        } catch (err) {
          api.logger.warn(
            `openclaw-chronicle-engine: failed to register ${eventName} via api.on, continue fallback: ${String(err)}`,
          );
        }
      }

      const registerHookFn = (api as any).registerHook;
      if (typeof registerHookFn === "function") {
        try {
          registerHookFn.call(api, eventName, wrapped, options);
          registeredViaInternalHook = true;
        } catch (err) {
          api.logger.warn(
            `openclaw-chronicle-engine: failed to register ${eventName} via api.registerHook: ${String(err)}`,
          );
        }
      }

      if (!registeredViaEventBus && !registeredViaInternalHook) {
        api.logger.warn(
          `openclaw-chronicle-engine: command hook registration failed for ${eventName}; no compatible API method available`,
        );
      }
    };

    if (config.selfImprovement?.enabled !== false) {
      let registeredBeforeResetNoteHooks = false;
      api.registerHook("agent:bootstrap", async (event) => {
        try {
          const context = (event.context || {}) as Record<string, unknown>;
          const sessionKey = typeof event.sessionKey === "string" ? event.sessionKey : "";
          const workspaceDir = resolveWorkspaceDirFromContext(context);

          if (isInternalReflectionSessionKey(sessionKey)) {
            return;
          }

          if (config.selfImprovement?.skipSubagentBootstrap !== false && sessionKey.includes(":subagent:")) {
            return;
          }

          if (config.selfImprovement?.ensureLearningFiles !== false) {
            await ensureSelfImprovementLearningFiles(workspaceDir);
          }

          const bootstrapFiles = context.bootstrapFiles;
          if (!Array.isArray(bootstrapFiles)) return;

          const exists = bootstrapFiles.some((f) => {
            if (!f || typeof f !== "object") return false;
            const pathValue = (f as Record<string, unknown>).path;
            return typeof pathValue === "string" && pathValue === "SELF_IMPROVEMENT_REMINDER.md";
          });
          if (exists) return;

          const content = await loadSelfImprovementReminderContent(workspaceDir);
          bootstrapFiles.push({
            path: "SELF_IMPROVEMENT_REMINDER.md",
            content,
            virtual: true,
          });
        } catch (err) {
          api.logger.warn(`self-improvement: bootstrap inject failed: ${String(err)}`);
        }
      }, {
        name: "openclaw-chronicle-engine.self-improvement.agent-bootstrap",
        description: "Inject self-improvement reminder on agent bootstrap",
      });

      if (config.selfImprovement?.beforeResetNote !== false && config.sessionStrategy !== "memoryReflection") {
        registeredBeforeResetNoteHooks = true;
        const appendSelfImprovementNote = async (event: any) => {
          try {
            const action = String(event?.action || "unknown");
            const sessionKeyForLog = typeof event?.sessionKey === "string" ? event.sessionKey : "";
            const contextForLog = (event?.context && typeof event.context === "object")
              ? (event.context as Record<string, unknown>)
              : {};
            const commandSource = typeof contextForLog.commandSource === "string" ? contextForLog.commandSource : "";
            const contextKeys = Object.keys(contextForLog).slice(0, 8).join(",");
            api.logger.info(
              `self-improvement: command:${action} hook start; sessionKey=${sessionKeyForLog || "(none)"}; source=${commandSource || "(unknown)"}; hasMessages=${Array.isArray(event?.messages)}; contextKeys=${contextKeys || "(none)"}`
            );

            if (!Array.isArray(event.messages)) {
              api.logger.warn(`self-improvement: command:${action} missing event.messages array; skip note inject`);
              return;
            }

            const exists = event.messages.some((m: unknown) => typeof m === "string" && m.includes(SELF_IMPROVEMENT_NOTE_PREFIX));
            if (exists) {
              api.logger.info(`self-improvement: command:${action} note already present; skip duplicate inject`);
              return;
            }

            event.messages.push(buildSelfImprovementResetNote());
            api.logger.info(
              `self-improvement: command:${action} injected note; messages=${event.messages.length}`
            );
          } catch (err) {
            api.logger.warn(`self-improvement: note inject failed: ${String(err)}`);
          }
        };

        const selfImprovementNewHookOptions = {
          name: "openclaw-chronicle-engine.self-improvement.command-new",
          description: "Append self-improvement note before /new",
        } as const;
        const selfImprovementResetHookOptions = {
          name: "openclaw-chronicle-engine.self-improvement.command-reset",
          description: "Append self-improvement note before /reset",
        } as const;
        registerDurableCommandHook("command:new", appendSelfImprovementNote, selfImprovementNewHookOptions, "self-improvement");
        registerDurableCommandHook("command:reset", appendSelfImprovementNote, selfImprovementResetHookOptions, "self-improvement");
        api.on("gateway_start", () => {
          registerDurableCommandHook("command:new", appendSelfImprovementNote, selfImprovementNewHookOptions, "self-improvement");
          registerDurableCommandHook("command:reset", appendSelfImprovementNote, selfImprovementResetHookOptions, "self-improvement");
          api.logger.info("self-improvement: command hooks refreshed after gateway_start");
        }, { priority: 12 });
      }

      api.logger.info(
        registeredBeforeResetNoteHooks
          ? "self-improvement: integrated hooks registered (agent:bootstrap, command:new, command:reset)"
          : "self-improvement: integrated hooks registered (agent:bootstrap)"
      );
    }

    // ========================================================================
    // Integrated Memory Reflection (reflection)
    // ========================================================================

    if (config.sessionStrategy === "memoryReflection") {
      const reflectionMessageCount = config.memoryReflection?.messageCount ?? DEFAULT_REFLECTION_MESSAGE_COUNT;
      const reflectionErrorReminderMaxEntries =
        parsePositiveInt(config.memoryReflection?.errorReminderMaxEntries) ?? DEFAULT_REFLECTION_ERROR_REMINDER_MAX_ENTRIES;
      const reflectionDedupeErrorSignals = config.memoryReflection?.dedupeErrorSignals !== false;
      const reflectionInjectMode = config.memoryReflection?.injectMode ?? "inheritance+derived";
      const reflectionRecallMode = config.memoryReflection?.recall?.mode ?? DEFAULT_REFLECTION_RECALL_MODE;
      const reflectionRecallTopK = config.memoryReflection?.recall?.topK ?? DEFAULT_REFLECTION_RECALL_TOP_K;
      const reflectionRecallIncludeKinds = config.memoryReflection?.recall?.includeKinds ?? DEFAULT_REFLECTION_RECALL_INCLUDE_KINDS;
      const reflectionRecallMaxAgeDays = config.memoryReflection?.recall?.maxAgeDays ?? DEFAULT_REFLECTION_RECALL_MAX_AGE_DAYS;
      const reflectionRecallMaxEntriesPerKey = config.memoryReflection?.recall?.maxEntriesPerKey ?? DEFAULT_REFLECTION_RECALL_MAX_ENTRIES_PER_KEY;
      const reflectionRecallMinRepeated = config.memoryReflection?.recall?.minRepeated ?? DEFAULT_REFLECTION_RECALL_MIN_REPEATED;
      const reflectionRecallMinScore = config.memoryReflection?.recall?.minScore ?? DEFAULT_REFLECTION_RECALL_MIN_SCORE;
      const reflectionRecallMinPromptLength = config.memoryReflection?.recall?.minPromptLength ?? DEFAULT_REFLECTION_RECALL_MIN_PROMPT_LENGTH;
      const reflectionTriggerSeenAt = new Map<string, number>();
      const REFLECTION_TRIGGER_DEDUPE_MS = 12_000;

      const pruneReflectionTriggerSeenAt = () => {
        const now = Date.now();
        for (const [key, ts] of reflectionTriggerSeenAt.entries()) {
          if (now - ts > REFLECTION_TRIGGER_DEDUPE_MS * 3) {
            reflectionTriggerSeenAt.delete(key);
          }
        }
      };

      const isDuplicateReflectionTrigger = (key: string): boolean => {
        pruneReflectionTriggerSeenAt();
        const now = Date.now();
        const prev = reflectionTriggerSeenAt.get(key);
        reflectionTriggerSeenAt.set(key, now);
        return typeof prev === "number" && (now - prev) < REFLECTION_TRIGGER_DEDUPE_MS;
      };

      // Shared command contract across local and remote reflection paths.
      // Unknown values degrade to "reset" to preserve fail-open command flow.
      const normalizeReflectionTrigger = (action: unknown): BackendReflectionTrigger => {
        const raw = typeof action === "string" ? action.trim().toLowerCase() : "";
        return raw === "new" ? "new" : "reset";
      };

      const toReflectionCommandName = (trigger: BackendReflectionTrigger): string =>
        `command:${trigger}`;

      const reflectionPromptPlanner = createReflectionPromptPlanner(
        {
          injectMode: reflectionInjectMode,
          dedupeErrorSignals: reflectionDedupeErrorSignals,
          errorReminderMaxEntries: reflectionErrorReminderMaxEntries,
          errorScanMaxChars: DEFAULT_REFLECTION_ERROR_SCAN_MAX_CHARS,
          recall: {
            mode: reflectionRecallMode,
            topK: reflectionRecallTopK,
            includeKinds: reflectionRecallIncludeKinds,
            maxAgeDays: reflectionRecallMaxAgeDays,
            maxEntriesPerKey: reflectionRecallMaxEntriesPerKey,
            minRepeated: reflectionRecallMinRepeated,
            minScore: reflectionRecallMinScore,
            minPromptLength: reflectionRecallMinPromptLength,
          },
        },
        {
          sessionState: sessionExposureState,
          recallReflection: async (params) => {
            const resolved = resolveBackendCallContext(
              {
                userId: params.userId,
                agentId: params.agentId,
                sessionId: params.sessionId,
                sessionKey: params.sessionKey,
              },
              remoteRuntimeDefaults
            );
            if (!resolved.hasPrincipalIdentity) {
              api.logger.warn(
                `memory-reflection: reflection-recall skipped remote call (missing runtime principal: ${resolved.missingPrincipalFields.join(", ")})`
              );
              return [];
            }
            const mode: BackendReflectionRecallMode = params.mode === "invariant-only"
              ? "invariant-only"
              : "invariant+derived";
            return await memoryBackendClient.recallReflection(resolved.context, {
              query: String(params.prompt || "reflection-context"),
              mode,
              limit: params.limit,
              includeKinds: params.includeKinds,
              minScore: params.minScore,
            });
          },
          sanitizeForContext,
          logger: api.logger,
        }
      );

      api.on("after_tool_call", (event, ctx) => {
        const sessionKey = typeof ctx.sessionKey === "string" ? ctx.sessionKey : "";
        if (isInternalReflectionSessionKey(sessionKey)) return;
        if (!sessionKey) return;
        reflectionPromptPlanner.captureAfterToolCall(event, sessionKey);
      }, { priority: 15 });

      api.on("before_prompt_build", async (event, ctx) => {
        const sessionKey = typeof ctx.sessionKey === "string" ? ctx.sessionKey : "";
        if (isInternalReflectionSessionKey(sessionKey)) return;
        const prependContext = await reflectionPromptPlanner.buildBeforePromptPrependContext({
          prompt: event?.prompt,
          agentId: ctx?.agentId,
          sessionId: ctx?.sessionId,
          sessionKey: ctx?.sessionKey,
          userId: ctx?.userId,
        });
        if (!prependContext) return;
        return { prependContext };
      }, { priority: 15 });

      api.on("session_end", (_event, ctx) => {
        const sessionKey = typeof ctx.sessionKey === "string" ? ctx.sessionKey.trim() : "";
        if (!sessionKey) return;
        reflectionPromptPlanner.clearSession({
          sessionKey,
          sessionId: typeof ctx.sessionId === "string" ? ctx.sessionId : undefined,
        });
        reflectionPromptPlanner.pruneSessionState();
      }, { priority: 20 });

      const runMemoryReflection = async (event: any) => {
        const sessionKey = typeof event.sessionKey === "string" ? event.sessionKey : "";
        let clearSessionId = typeof event?.sessionId === "string" ? event.sessionId : undefined;
        try {
          reflectionPromptPlanner.pruneSessionState();
          const trigger = normalizeReflectionTrigger(event?.action);
          const commandName = toReflectionCommandName(trigger);
          const context = (event.context || {}) as Record<string, unknown>;
          const cfg = context.cfg;
          const workspaceDir = resolveWorkspaceDirFromContext(context);
          const sessionEntry = (context.previousSessionEntry || context.sessionEntry || {}) as Record<string, unknown>;
          const currentSessionId = typeof sessionEntry.sessionId === "string"
            ? sessionEntry.sessionId
            : (typeof event?.sessionId === "string" ? event.sessionId : "unknown");
          clearSessionId = currentSessionId;
          const runtimeAgentId =
            asNonEmptyString(typeof event?.agentId === "string" ? event.agentId : undefined) ??
            asNonEmptyString(typeof context.agentId === "string" ? context.agentId : undefined);
          const commandSource = typeof context.commandSource === "string" ? context.commandSource : "";
          const triggerKey = `${trigger}|${sessionKey || "(none)"}|${currentSessionId || "unknown"}`;
          if (isDuplicateReflectionTrigger(triggerKey)) {
            api.logger.info(`memory-reflection: duplicate trigger skipped; key=${triggerKey}`);
            return;
          }
          api.logger.info(
            `memory-reflection: ${commandName} enqueue start; sessionKey=${sessionKey || "(none)"}; source=${commandSource || "(unknown)"}; sessionId=${currentSessionId}`
          );

          const resolvedBackendCtx = resolveBackendCallContext(
            mergeContextSources(event, context, {
              agentId: runtimeAgentId,
              sessionId: currentSessionId,
              sessionKey,
            }),
            remoteRuntimeDefaults,
            {
              agentId: runtimeAgentId,
              sessionId: currentSessionId,
              sessionKey,
            }
          );
          if (!resolvedBackendCtx.hasPrincipalIdentity) {
            api.logger.warn(
              `memory-reflection: ${commandName} enqueue blocked (missing runtime principal: ${resolvedBackendCtx.missingPrincipalFields.join(", ")})`
            );
            return;
          }

          let captureItems: BackendCaptureItem[] = [];
          try {
            const reflectionSource = await memoryBackendClient.loadReflectionSource(
              resolvedBackendCtx.context,
              {
                trigger,
                maxMessages: reflectionMessageCount,
              }
            );
            if (Array.isArray(reflectionSource.messages) && reflectionSource.messages.length > 0) {
              captureItems = reflectionSource.messages;
            }
          } catch (err) {
            api.logger.warn(`memory-reflection: ${commandName} source load failed: ${String(err)}`);
          }
          if (captureItems.length === 0 && Array.isArray(event.messages)) {
            captureItems = parseEventMessagesToCaptureItems(event.messages);
          }
          if (captureItems.length === 0) {
            api.logger.warn(`memory-reflection: ${commandName} no capture payload found; skip enqueue`);
            return;
          }

          if (config.selfImprovement?.enabled !== false && config.selfImprovement?.beforeResetNote !== false) {
            if (Array.isArray(event.messages)) {
              const exists = event.messages.some((m: unknown) => typeof m === "string" && m.includes(SELF_IMPROVEMENT_NOTE_PREFIX));
              if (!exists) {
                event.messages.push(buildSelfImprovementResetNote());
              }
            }
          }

          const enqueueInput = {
            trigger,
            messages: captureItems.slice(0, 256),
          };
          void memoryBackendClient
            .enqueueReflectionJob(resolvedBackendCtx.context, enqueueInput)
            .then((enqueue) => {
              api.logger.info(
                `memory-reflection: ${commandName} enqueue accepted; jobId=${enqueue.jobId}; status=${enqueue.status}; items=${captureItems.length}`
              );
            })
            .catch((err) => {
              const msg = err instanceof Error ? err.message : String(err);
              api.logger.warn(`memory-reflection: ${commandName} enqueue failed: ${msg}`);
            });
        } catch (err) {
          api.logger.warn(`memory-reflection: hook failed: ${String(err)}`);
        } finally {
          if (sessionKey) {
            reflectionPromptPlanner.clearSession({
              sessionKey,
              sessionId: clearSessionId,
            });
          }
          reflectionPromptPlanner.pruneSessionState();
        }
      };

      const memoryReflectionNewHookOptions = {
        name: "openclaw-chronicle-engine.memory-reflection.command-new",
        description: "Run reflection pipeline before /new",
      } as const;
      const memoryReflectionResetHookOptions = {
        name: "openclaw-chronicle-engine.memory-reflection.command-reset",
        description: "Run reflection pipeline before /reset",
      } as const;
      registerDurableCommandHook("command:new", runMemoryReflection, memoryReflectionNewHookOptions, "memory-reflection");
      registerDurableCommandHook("command:reset", runMemoryReflection, memoryReflectionResetHookOptions, "memory-reflection");
      api.on("gateway_start", () => {
        registerDurableCommandHook("command:new", runMemoryReflection, memoryReflectionNewHookOptions, "memory-reflection");
        registerDurableCommandHook("command:reset", runMemoryReflection, memoryReflectionResetHookOptions, "memory-reflection");
        api.logger.info("memory-reflection: command hooks refreshed after gateway_start");
      }, { priority: 12 });
      api.on("before_reset", async (event, ctx) => {
        try {
          const trigger = normalizeReflectionTrigger(event.reason);
          await runMemoryReflection({
            action: trigger,
            sessionKey: typeof ctx.sessionKey === "string" ? ctx.sessionKey : "",
            sessionId: typeof ctx.sessionId === "string" ? ctx.sessionId : "unknown",
            timestamp: Date.now(),
            messages: Array.isArray(event.messages) ? event.messages : [],
            context: {
              cfg: api.config,
              workspaceDir: ctx.workspaceDir,
              commandSource: `lifecycle:before_reset:${trigger}`,
              sessionEntry: {
                sessionId: typeof ctx.sessionId === "string" ? ctx.sessionId : "unknown",
              },
            },
          });
        } catch (err) {
          api.logger.warn(`memory-reflection: before_reset fallback failed: ${String(err)}`);
        }
      }, { priority: 12 });
      api.logger.info("memory-reflection: integrated hooks registered (command:new, command:reset, after_tool_call, before_prompt_build[inherited-rules,error-detected])");
    }

    if (config.sessionStrategy === "systemSessionMemory") {
      api.logger.info("session-strategy: using systemSessionMemory (plugin memory-reflection hooks disabled)");
    }
    if (config.sessionStrategy === "none") {
      api.logger.info("session-strategy: using none (plugin memory-reflection hooks disabled)");
    }

    // ========================================================================
    // Service Registration
    // ========================================================================

    api.registerService({
      id: "openclaw-chronicle-engine",
      start: async () => {
        api.logger.info("openclaw-chronicle-engine: remote backend mode active");
      },
      stop: async () => {
        api.logger.info("openclaw-chronicle-engine: stopped (remote backend mode)");
      },
    });
  },
};

export function parsePluginConfig(value: unknown): PluginConfig {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("openclaw-chronicle-engine config required");
  }
  const cfg = value as Record<string, unknown>;

  const remoteBackendRaw = typeof cfg.remoteBackend === "object" && cfg.remoteBackend !== null
    ? cfg.remoteBackend as Record<string, unknown>
    : null;
  if (!remoteBackendRaw || remoteBackendRaw.enabled !== true) {
    throw new Error(
      "remoteBackend.enabled=true is required; local-authority runtime has been removed"
    );
  }
  const remoteBackendBaseURL = typeof remoteBackendRaw?.baseURL === "string"
    ? resolveEnvVars(remoteBackendRaw.baseURL)
    : undefined;
  const remoteBackendAuthToken = typeof remoteBackendRaw?.authToken === "string"
    ? resolveEnvVars(remoteBackendRaw.authToken)
    : undefined;

  if (!remoteBackendBaseURL) {
    throw new Error("remoteBackend.baseURL is required when remoteBackend is enabled");
  }
  if (!remoteBackendAuthToken) {
    throw new Error("remoteBackend.authToken is required when remoteBackend is enabled");
  }

  const memoryReflectionRaw = typeof cfg.memoryReflection === "object" && cfg.memoryReflection !== null
    ? cfg.memoryReflection as Record<string, unknown>
    : null;
  if (hasOwnKey(cfg as Record<string, unknown>, "sessionMemory")) {
    rejectRemovedConfigField("sessionMemory", "sessionStrategy and memoryReflection.messageCount");
  }
  if (memoryReflectionRaw) {
    for (const field of ["agentId", "maxInputChars", "timeoutMs", "thinkLevel"]) {
      if (hasOwnKey(memoryReflectionRaw, field)) {
        rejectRemovedConfigField(`memoryReflection.${field}`);
      }
    }
  }
  const sessionStrategyRaw = cfg.sessionStrategy;
  const sessionStrategy: SessionStrategy =
    sessionStrategyRaw === "systemSessionMemory" || sessionStrategyRaw === "memoryReflection" || sessionStrategyRaw === "none"
      ? sessionStrategyRaw
      : "systemSessionMemory";
  const reflectionMessageCount = parsePositiveInt(memoryReflectionRaw?.messageCount) ?? DEFAULT_REFLECTION_MESSAGE_COUNT;
  const injectModeRaw = memoryReflectionRaw?.injectMode;
  const reflectionInjectMode: ReflectionInjectMode =
    injectModeRaw === "inheritance-only" || injectModeRaw === "inheritance+derived"
      ? injectModeRaw
      : "inheritance+derived";
  const memoryReflectionRecallRaw = typeof memoryReflectionRaw?.recall === "object" && memoryReflectionRaw.recall !== null
    ? memoryReflectionRaw.recall as Record<string, unknown>
    : null;
  const reflectionRecallMode: ReflectionRecallMode =
    memoryReflectionRecallRaw?.mode === "dynamic" ? "dynamic" : DEFAULT_REFLECTION_RECALL_MODE;
  const reflectionRecallTopK = parsePositiveInt(memoryReflectionRecallRaw?.topK) ?? DEFAULT_REFLECTION_RECALL_TOP_K;
  const reflectionRecallIncludeKinds = parseReflectionRecallKinds(
    memoryReflectionRecallRaw?.includeKinds,
    DEFAULT_REFLECTION_RECALL_INCLUDE_KINDS
  );
  const reflectionRecallMaxAgeDays = parsePositiveInt(memoryReflectionRecallRaw?.maxAgeDays) ?? DEFAULT_REFLECTION_RECALL_MAX_AGE_DAYS;
  const reflectionRecallMaxEntriesPerKey = parsePositiveInt(memoryReflectionRecallRaw?.maxEntriesPerKey) ?? DEFAULT_REFLECTION_RECALL_MAX_ENTRIES_PER_KEY;
  const reflectionRecallMinRepeated = parsePositiveInt(memoryReflectionRecallRaw?.minRepeated) ?? DEFAULT_REFLECTION_RECALL_MIN_REPEATED;
  const reflectionRecallMinScore = parseNonNegativeNumber(memoryReflectionRecallRaw?.minScore) ?? DEFAULT_REFLECTION_RECALL_MIN_SCORE;
  const reflectionRecallMinPromptLength = parsePositiveInt(memoryReflectionRecallRaw?.minPromptLength) ?? DEFAULT_REFLECTION_RECALL_MIN_PROMPT_LENGTH;
  const autoRecallSelectionMode: AutoRecallSelectionMode =
    cfg.autoRecallSelectionMode === "setwise-v2"
      ? "setwise-v2"
      : cfg.autoRecallSelectionMode === "mmr"
        ? "mmr"
      : DEFAULT_AUTO_RECALL_SELECTION_MODE;

  return {
    autoCapture: cfg.autoCapture !== false,
    // Default OFF: only enable when explicitly set to true.
    autoRecall: cfg.autoRecall === true,
    autoRecallMinLength: parsePositiveInt(cfg.autoRecallMinLength),
    autoRecallMinRepeated: parsePositiveInt(cfg.autoRecallMinRepeated),
    autoRecallTopK: parsePositiveInt(cfg.autoRecallTopK) ?? DEFAULT_AUTO_RECALL_TOP_K,
    autoRecallSelectionMode,
    autoRecallCategories: parseMemoryCategories(cfg.autoRecallCategories, DEFAULT_AUTO_RECALL_CATEGORIES),
    autoRecallExcludeReflection: typeof cfg.autoRecallExcludeReflection === "boolean"
      ? cfg.autoRecallExcludeReflection
      : DEFAULT_AUTO_RECALL_EXCLUDE_REFLECTION,
    autoRecallMaxAgeDays: parsePositiveInt(cfg.autoRecallMaxAgeDays) ?? DEFAULT_AUTO_RECALL_MAX_AGE_DAYS,
    autoRecallMaxEntriesPerKey: parsePositiveInt(cfg.autoRecallMaxEntriesPerKey) ?? DEFAULT_AUTO_RECALL_MAX_ENTRIES_PER_KEY,
    captureAssistant: cfg.captureAssistant === true,
    enableManagementTools: cfg.enableManagementTools === true,
    sessionStrategy,
    selfImprovement: typeof cfg.selfImprovement === "object" && cfg.selfImprovement !== null
      ? {
        enabled: (cfg.selfImprovement as Record<string, unknown>).enabled !== false,
        beforeResetNote: (cfg.selfImprovement as Record<string, unknown>).beforeResetNote !== false,
        skipSubagentBootstrap: (cfg.selfImprovement as Record<string, unknown>).skipSubagentBootstrap !== false,
        ensureLearningFiles: (cfg.selfImprovement as Record<string, unknown>).ensureLearningFiles !== false,
      }
      : {
        enabled: true,
        beforeResetNote: true,
        skipSubagentBootstrap: true,
        ensureLearningFiles: true,
      },
    memoryReflection: memoryReflectionRaw
      ? {
        enabled: sessionStrategy === "memoryReflection",
        injectMode: reflectionInjectMode,
        messageCount: reflectionMessageCount,
        errorReminderMaxEntries: parsePositiveInt(memoryReflectionRaw.errorReminderMaxEntries) ?? DEFAULT_REFLECTION_ERROR_REMINDER_MAX_ENTRIES,
        dedupeErrorSignals: memoryReflectionRaw.dedupeErrorSignals !== false,
        recall: {
          mode: reflectionRecallMode,
          topK: reflectionRecallTopK,
          includeKinds: reflectionRecallIncludeKinds,
          maxAgeDays: reflectionRecallMaxAgeDays,
          maxEntriesPerKey: reflectionRecallMaxEntriesPerKey,
          minRepeated: reflectionRecallMinRepeated,
          minScore: reflectionRecallMinScore,
          minPromptLength: reflectionRecallMinPromptLength,
        },
      }
      : {
        enabled: sessionStrategy === "memoryReflection",
        injectMode: "inheritance+derived",
        messageCount: reflectionMessageCount,
        errorReminderMaxEntries: DEFAULT_REFLECTION_ERROR_REMINDER_MAX_ENTRIES,
        dedupeErrorSignals: DEFAULT_REFLECTION_DEDUPE_ERROR_SIGNALS,
        recall: {
          mode: DEFAULT_REFLECTION_RECALL_MODE,
          topK: DEFAULT_REFLECTION_RECALL_TOP_K,
          includeKinds: [...DEFAULT_REFLECTION_RECALL_INCLUDE_KINDS],
          maxAgeDays: DEFAULT_REFLECTION_RECALL_MAX_AGE_DAYS,
          maxEntriesPerKey: DEFAULT_REFLECTION_RECALL_MAX_ENTRIES_PER_KEY,
          minRepeated: DEFAULT_REFLECTION_RECALL_MIN_REPEATED,
          minScore: DEFAULT_REFLECTION_RECALL_MIN_SCORE,
          minPromptLength: DEFAULT_REFLECTION_RECALL_MIN_PROMPT_LENGTH,
        },
      },
    remoteBackend: {
      enabled: true,
      baseURL: remoteBackendBaseURL,
      authToken: remoteBackendAuthToken,
      timeoutMs: parsePositiveInt(remoteBackendRaw?.timeoutMs) ?? 10_000,
      maxRetries: parseNonNegativeNumber(remoteBackendRaw?.maxRetries) ?? 1,
      retryBackoffMs: parsePositiveInt(remoteBackendRaw?.retryBackoffMs) ?? 250,
    },
  };
}

export default chronicleEnginePlugin;
