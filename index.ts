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
import { ensureGovernanceBacklogFiles, registerGovernanceTools } from "./src/governance-tools.js";
import {
  createSessionExposureState,
  DEFAULT_SESSION_EXPOSURE_MAX_TRACKED_SESSIONS,
} from "./src/context/session-exposure-state.js";
import {
  createAutoRecallBehavioralPlanner,
  createAutoRecallPlanner,
} from "./src/context/auto-recall-orchestrator.js";
import { createMemoryBackendClient } from "./src/backend-client/client.js";
import {
  resolveBackendCallContext,
  type RuntimeContextDefaults,
} from "./src/backend-client/runtime-context.js";
import type {
  BackendCaptureItem,
  MemoryBackendClient,
  DistillMode as BackendDistillMode,
  DistillPersistMode as BackendDistillPersistMode,
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
  autoRecallExcludeBehavioral?: boolean;
  autoRecallExcludeReflection?: boolean;
  autoRecallMaxAgeDays?: number;
  autoRecallMaxEntriesPerKey?: number;
  captureAssistant?: boolean;
  enableManagementTools?: boolean;
  sessionStrategy?: SessionStrategy;
  governance?: {
    enabled?: boolean;
    ensureBacklogFiles?: boolean;
  };
  autoRecallBehavioral?: {
    enabled?: boolean;
    injectMode?: BehavioralGuidanceInjectMode;
    errorReminderMaxEntries?: number;
    dedupeErrorSignals?: boolean;
    beforeResetNote?: boolean;
    skipSubagentBootstrap?: boolean;
    ensureGovernanceFiles?: boolean;
    recall?: {
      mode?: BehavioralRecallMode;
      topK?: number;
      includeKinds?: BehavioralRecallKind[];
      maxAgeDays?: number;
      maxEntriesPerKey?: number;
      minRepeated?: number;
      minScore?: number;
      minPromptLength?: number;
    };
  };
  distill?: {
    enabled?: boolean;
    mode?: BackendDistillMode;
    persistMode?: BackendDistillPersistMode;
    everyTurns?: number;
    maxMessages?: number;
    maxArtifacts?: number;
    chunkChars?: number;
    chunkOverlapMessages?: number;
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
type SessionStrategy = "autoRecall" | "systemSessionMemory" | "none";
type BehavioralGuidanceInjectMode = "durable-only" | "durable+adaptive";
type BehavioralRecallMode = "fixed" | "dynamic";
type BehavioralRecallKind = "durable" | "adaptive";
type AutoRecallSelectionMode = "mmr";
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

function parseBehavioralRecallKinds(value: unknown, fallback: BehavioralRecallKind[]): BehavioralRecallKind[] {
  if (!Array.isArray(value)) return [...fallback];
  const parsed = value
    .filter((item): item is string => typeof item === "string")
    .map((item) => item.trim())
    .map((item): BehavioralRecallKind | undefined => {
      if (item === "durable" || item === "invariant") return "durable";
      if (item === "adaptive" || item === "derived") return "adaptive";
      return undefined;
    })
    .filter((item): item is BehavioralRecallKind => item === "durable" || item === "adaptive");
  return parsed.length > 0 ? [...new Set(parsed)] : [...fallback];
}

const DEFAULT_BEHAVIORAL_GUIDANCE_REMINDER = `## AutoRecall Behavioral Guidance Reminder

After completing tasks, evaluate whether anything belongs in governance backlog:

**Log when:**
- User correction / reusable best practice / knowledge gap -> .governance/LEARNINGS.md
- Command / tool / integration failure -> .governance/ERRORS.md

**Promote when pattern is proven:**
- Behavioral rules -> SOUL.md
- Workflow rules -> AGENTS.md
- Tool gotchas -> TOOLS.md

Keep entries simple: date, title, what happened, what to do differently.`;

const BEHAVIORAL_GUIDANCE_NOTE_PREFIX = "/note behavioral-guidance (before reset):";
const DEFAULT_BEHAVIORAL_ERROR_REMINDER_MAX_ENTRIES = 3;
const DEFAULT_BEHAVIORAL_DEDUPE_ERROR_SIGNALS = true;
const DEFAULT_BEHAVIORAL_ERROR_SCAN_MAX_CHARS = 8_000;
const DEFAULT_AUTO_RECALL_TOP_K = 3;
const DEFAULT_AUTO_RECALL_SELECTION_MODE: AutoRecallSelectionMode = "mmr";
const DEFAULT_AUTO_RECALL_EXCLUDE_BEHAVIORAL = true;
const DEFAULT_AUTO_RECALL_MAX_AGE_DAYS = 30;
const DEFAULT_AUTO_RECALL_MAX_ENTRIES_PER_KEY = 10;
const DEFAULT_AUTO_RECALL_CATEGORIES: MemoryCategory[] = ["preference", "fact", "decision", "entity", "other"];
const DEFAULT_BEHAVIORAL_RECALL_MODE: BehavioralRecallMode = "fixed";
const DEFAULT_BEHAVIORAL_RECALL_TOP_K = 6;
const DEFAULT_BEHAVIORAL_RECALL_INCLUDE_KINDS: BehavioralRecallKind[] = ["durable"];
const DEFAULT_BEHAVIORAL_RECALL_MAX_AGE_DAYS = 45;
const DEFAULT_BEHAVIORAL_RECALL_MAX_ENTRIES_PER_KEY = 10;
const DEFAULT_BEHAVIORAL_RECALL_MIN_REPEATED = 2;
const DEFAULT_BEHAVIORAL_RECALL_MIN_SCORE = 0.18;
const DEFAULT_BEHAVIORAL_RECALL_MIN_PROMPT_LENGTH = 8;
const DEFAULT_DISTILL_MODE: BackendDistillMode = "session-lessons";
const DEFAULT_DISTILL_PERSIST_MODE: BackendDistillPersistMode = "artifacts-only";
const DEFAULT_DISTILL_EVERY_TURNS = 5;
const DEFAULT_DISTILL_MAX_MESSAGES = 400;
const DEFAULT_DISTILL_MAX_ARTIFACTS = 20;
const DEFAULT_DISTILL_CHUNK_CHARS = 12_000;
const DEFAULT_DISTILL_CHUNK_OVERLAP_MESSAGES = 10;
const DIAG_BUILD_TAG = "openclaw-chronicle-engine-diag-20260308-0058";

interface DistillRuntimeConfig {
  enabled: boolean;
  mode: BackendDistillMode;
  persistMode: BackendDistillPersistMode;
  everyTurns: number;
  maxMessages: number;
  maxArtifacts: number;
  chunkChars: number;
  chunkOverlapMessages: number;
}

interface DistillCadenceSessionState {
  completedUserTurns: number;
  lastEnqueuedBucket: number;
}

function buildBehavioralGuidanceResetNote(params?: { openLoopsBlock?: string; derivedFocusBlock?: string }): string {
  const openLoopsBlock = typeof params?.openLoopsBlock === "string" ? params.openLoopsBlock : "";
  const derivedFocusBlock = typeof params?.derivedFocusBlock === "string" ? params.derivedFocusBlock : "";
  const base = [
    BEHAVIORAL_GUIDANCE_NOTE_PREFIX,
    "- If anything was learned/corrected, log it now:",
    "  - .governance/LEARNINGS.md (corrections/best practices)",
    "  - .governance/ERRORS.md (failures/root causes)",
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

async function loadBehavioralGuidanceReminderContent(workspaceDir?: string): Promise<string> {
  const baseDir = typeof workspaceDir === "string" && workspaceDir.trim().length ? workspaceDir.trim() : "";
  if (!baseDir) return DEFAULT_BEHAVIORAL_GUIDANCE_REMINDER;

  const reminderPath = join(baseDir, "AUTORECALL_BEHAVIORAL_GUIDANCE.md");
  try {
    const content = await readFile(reminderPath, "utf-8");
    const trimmed = content.trim();
    return trimmed.length ? trimmed : DEFAULT_BEHAVIORAL_GUIDANCE_REMINDER;
  } catch {
    return DEFAULT_BEHAVIORAL_GUIDANCE_REMINDER;
  }
}

function asNonEmptyString(value: unknown): string | undefined {
  if (typeof value !== "string") return undefined;
  const trimmed = value.trim();
  return trimmed.length ? trimmed : undefined;
}

function isInternalBehavioralGuidanceSessionKey(sessionKey: unknown): boolean {
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

function normalizeDistillMode(value: unknown): BackendDistillMode {
  return value === "governance-candidates" ? "governance-candidates" : DEFAULT_DISTILL_MODE;
}

function normalizeDistillPersistMode(value: unknown): BackendDistillPersistMode {
  return value === "persist-memory-rows" ? "persist-memory-rows" : DEFAULT_DISTILL_PERSIST_MODE;
}

function countUserTurns(items: BackendCaptureItem[]): number {
  return items.reduce((count, item) => count + (item.role === "user" ? 1 : 0), 0);
}

function buildAutomaticDistillIdempotencyKey(
  actor: { userId: string; agentId: string; sessionId: string; sessionKey: string },
  bucket: number,
  config: DistillRuntimeConfig
): string {
  const digest = sha256Hex(
    JSON.stringify({
      kind: "automatic-distill",
      userId: actor.userId,
      agentId: actor.agentId,
      sessionId: actor.sessionId,
      sessionKey: actor.sessionKey,
      bucket,
      mode: config.mode,
      persistMode: config.persistMode,
      maxMessages: config.maxMessages,
      maxArtifacts: config.maxArtifacts,
      chunkChars: config.chunkChars,
      chunkOverlapMessages: config.chunkOverlapMessages,
    })
  );
  return `automatic-distill:${digest.slice(0, 48)}`;
}

function createDistillCadenceState(maxSessions = DEFAULT_SESSION_EXPOSURE_MAX_TRACKED_SESSIONS) {
  const sessions = new Map<string, DistillCadenceSessionState>();

  function touch(sessionKey: string, next: DistillCadenceSessionState) {
    sessions.delete(sessionKey);
    sessions.set(sessionKey, next);
    while (sessions.size > maxSessions) {
      const oldest = sessions.keys().next().value;
      if (!oldest) break;
      sessions.delete(oldest);
    }
  }

  return {
    advance(sessionKey: string, userTurnCount: number, everyTurns: number) {
      const normalizedSessionKey = asNonEmptyString(sessionKey);
      if (!normalizedSessionKey || userTurnCount <= 0 || everyTurns <= 0) {
        return { bucket: 0, crossedBoundary: false, completedUserTurns: 0 };
      }
      const current = sessions.get(normalizedSessionKey) ?? {
        completedUserTurns: 0,
        lastEnqueuedBucket: 0,
      };
      const completedUserTurns = current.completedUserTurns + userTurnCount;
      const bucket = Math.floor(completedUserTurns / everyTurns);
      const crossedBoundary = bucket > current.lastEnqueuedBucket;
      touch(normalizedSessionKey, {
        completedUserTurns,
        lastEnqueuedBucket: crossedBoundary ? bucket : current.lastEnqueuedBucket,
      });
      return { bucket, crossedBoundary, completedUserTurns };
    },
    clear(sessionKey?: string) {
      const normalizedSessionKey = asNonEmptyString(sessionKey);
      if (!normalizedSessionKey) return;
      sessions.delete(normalizedSessionKey);
    },
  };
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
    registerGovernanceTools(api, {
      enabled: config.governance?.enabled !== false,
      enableManagementTools: config.enableManagementTools,
      defaultWorkspaceDir: getDefaultWorkspaceDir(),
    });

    // ========================================================================
    // Lifecycle Hooks
    // ========================================================================

    const distillCadenceState = createDistillCadenceState();

    api.on("session_end", (_event, ctx) => {
      sessionExposureState.clearDynamicRecallForContext(ctx || {});
      distillCadenceState.clear(typeof ctx?.sessionKey === "string" ? ctx.sessionKey : undefined);
    }, { priority: 20 });

    const autoRecallPlanner = createAutoRecallPlanner(
      {
        enabled: config.autoRecall === true,
        minPromptLength: config.autoRecallMinLength,
        minRepeated: config.autoRecallMinRepeated,
        topK: config.autoRecallTopK ?? DEFAULT_AUTO_RECALL_TOP_K,
        selectionMode: config.autoRecallSelectionMode ?? DEFAULT_AUTO_RECALL_SELECTION_MODE,
        categories: config.autoRecallCategories,
        excludeBehavioral: config.autoRecallExcludeBehavioral === true,
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
            excludeReflection: params.excludeBehavioral,
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

      if (config.distill?.enabled === true) {
        const userTurnCount = countUserTurns(transcriptItems);
        const cadence = distillCadenceState.advance(
          resolvedBackendCtx.context.actor.sessionKey,
          userTurnCount,
          config.distill.everyTurns
        );
        if (userTurnCount > 0 && cadence.crossedBoundary) {
          try {
            const job = await memoryBackendClient.enqueueDistillJob(resolvedBackendCtx.context, {
              mode: config.distill.mode,
              source: {
                kind: "session-transcript",
                sessionKey: resolvedBackendCtx.context.actor.sessionKey,
                sessionId: resolvedBackendCtx.context.actor.sessionId,
              },
              options: {
                persistMode: config.distill.persistMode,
                maxMessages: config.distill.maxMessages,
                maxArtifacts: config.distill.maxArtifacts,
                chunkChars: config.distill.chunkChars,
                chunkOverlapMessages: config.distill.chunkOverlapMessages,
              },
              idempotencyKey: buildAutomaticDistillIdempotencyKey(
                resolvedBackendCtx.context.actor,
                cadence.bucket,
                config.distill
              ),
            });
            api.logger.info(
              `openclaw-chronicle-engine: automatic distill enqueued (${job.jobId}, everyTurns=${config.distill.everyTurns}, completedUserTurns=${cadence.completedUserTurns})`
            );
          } catch (err) {
            api.logger.warn(`openclaw-chronicle-engine: automatic distill enqueue failed: ${String(err)}`);
          }
        }
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

    if (config.autoRecallBehavioral?.enabled === true) {
      let registeredBeforeResetNoteHooks = false;
      api.registerHook("agent:bootstrap", async (event) => {
        try {
          const context = (event.context || {}) as Record<string, unknown>;
          const sessionKey = typeof event.sessionKey === "string" ? event.sessionKey : "";
          const workspaceDir = resolveWorkspaceDirFromContext(context);

          if (isInternalBehavioralGuidanceSessionKey(sessionKey)) {
            return;
          }

          if (config.autoRecallBehavioral?.skipSubagentBootstrap !== false && sessionKey.includes(":subagent:")) {
            return;
          }

          if (config.autoRecallBehavioral?.ensureGovernanceFiles !== false || config.governance?.ensureBacklogFiles !== false) {
            await ensureGovernanceBacklogFiles(workspaceDir);
          }

          const bootstrapFiles = context.bootstrapFiles;
          if (!Array.isArray(bootstrapFiles)) return;

          const exists = bootstrapFiles.some((f) => {
            if (!f || typeof f !== "object") return false;
            const pathValue = (f as Record<string, unknown>).path;
            return typeof pathValue === "string" && pathValue === "AUTORECALL_BEHAVIORAL_GUIDANCE.md";
          });
          if (exists) return;

          const content = await loadBehavioralGuidanceReminderContent(workspaceDir);
          bootstrapFiles.push({
            path: "AUTORECALL_BEHAVIORAL_GUIDANCE.md",
            content,
            virtual: true,
          });
        } catch (err) {
          api.logger.warn(`auto-recall.behavioral-guidance: bootstrap inject failed: ${String(err)}`);
        }
      }, {
        name: "openclaw-chronicle-engine.auto-recall-behavioral.agent-bootstrap",
        description: "Inject behavioral autoRecall reminder on agent bootstrap",
      });

      if (config.autoRecallBehavioral?.beforeResetNote !== false) {
        registeredBeforeResetNoteHooks = true;
        const appendBehavioralGuidanceNote = async (event: any) => {
          try {
            const action = String(event?.action || "unknown");
            const sessionKeyForLog = typeof event?.sessionKey === "string" ? event.sessionKey : "";
            const contextForLog = (event?.context && typeof event.context === "object")
              ? (event.context as Record<string, unknown>)
              : {};
            const commandSource = typeof contextForLog.commandSource === "string" ? contextForLog.commandSource : "";
            const contextKeys = Object.keys(contextForLog).slice(0, 8).join(",");
            api.logger.info(
              `auto-recall.behavioral-guidance: command:${action} hook start; sessionKey=${sessionKeyForLog || "(none)"}; source=${commandSource || "(unknown)"}; hasMessages=${Array.isArray(event?.messages)}; contextKeys=${contextKeys || "(none)"}`
            );

            if (!Array.isArray(event.messages)) {
              api.logger.warn(`auto-recall.behavioral-guidance: command:${action} missing event.messages array; skip note inject`);
              return;
            }

            const exists = event.messages.some((m: unknown) => typeof m === "string" && m.includes(BEHAVIORAL_GUIDANCE_NOTE_PREFIX));
            if (exists) {
              api.logger.info(`auto-recall.behavioral-guidance: command:${action} note already present; skip duplicate inject`);
              return;
            }

            event.messages.push(buildBehavioralGuidanceResetNote());
            api.logger.info(
              `auto-recall.behavioral-guidance: command:${action} injected note; messages=${event.messages.length}`
            );
          } catch (err) {
            api.logger.warn(`auto-recall.behavioral-guidance: note inject failed: ${String(err)}`);
          }
        };

        const behavioralGuidanceNewHookOptions = {
          name: "openclaw-chronicle-engine.auto-recall-behavioral.command-new",
          description: "Append behavioral guidance note before /new",
        } as const;
        const behavioralGuidanceResetHookOptions = {
          name: "openclaw-chronicle-engine.auto-recall-behavioral.command-reset",
          description: "Append behavioral guidance note before /reset",
        } as const;
        registerDurableCommandHook("command:new", appendBehavioralGuidanceNote, behavioralGuidanceNewHookOptions, "auto-recall-behavioral");
        registerDurableCommandHook("command:reset", appendBehavioralGuidanceNote, behavioralGuidanceResetHookOptions, "auto-recall-behavioral");
        api.on("gateway_start", () => {
          registerDurableCommandHook("command:new", appendBehavioralGuidanceNote, behavioralGuidanceNewHookOptions, "auto-recall-behavioral");
          registerDurableCommandHook("command:reset", appendBehavioralGuidanceNote, behavioralGuidanceResetHookOptions, "auto-recall-behavioral");
          api.logger.info("auto-recall.behavioral-guidance: command hooks refreshed after gateway_start");
        }, { priority: 12 });
      }

      api.logger.info(
        registeredBeforeResetNoteHooks
          ? "auto-recall.behavioral-guidance: reminder hooks registered (agent:bootstrap, command:new, command:reset)"
          : "auto-recall.behavioral-guidance: reminder hooks registered (agent:bootstrap)"
      );
    }

    // ========================================================================
    // Behavioral AutoRecall Guidance
    // ========================================================================

    if (config.sessionStrategy === "autoRecall" && config.autoRecallBehavioral?.enabled === true) {
      const behavioralErrorReminderMaxEntries =
        parsePositiveInt(config.autoRecallBehavioral?.errorReminderMaxEntries) ?? DEFAULT_BEHAVIORAL_ERROR_REMINDER_MAX_ENTRIES;
      const behavioralDedupeErrorSignals = config.autoRecallBehavioral?.dedupeErrorSignals !== false;
      const behavioralInjectMode = config.autoRecallBehavioral?.injectMode ?? "durable+adaptive";
      const behavioralRecallMode = config.autoRecallBehavioral?.recall?.mode ?? DEFAULT_BEHAVIORAL_RECALL_MODE;
      const behavioralRecallTopK = config.autoRecallBehavioral?.recall?.topK ?? DEFAULT_BEHAVIORAL_RECALL_TOP_K;
      const behavioralRecallIncludeKinds = config.autoRecallBehavioral?.recall?.includeKinds ?? DEFAULT_BEHAVIORAL_RECALL_INCLUDE_KINDS;
      const behavioralRecallMaxAgeDays = config.autoRecallBehavioral?.recall?.maxAgeDays ?? DEFAULT_BEHAVIORAL_RECALL_MAX_AGE_DAYS;
      const behavioralRecallMaxEntriesPerKey = config.autoRecallBehavioral?.recall?.maxEntriesPerKey ?? DEFAULT_BEHAVIORAL_RECALL_MAX_ENTRIES_PER_KEY;
      const behavioralRecallMinRepeated = config.autoRecallBehavioral?.recall?.minRepeated ?? DEFAULT_BEHAVIORAL_RECALL_MIN_REPEATED;
      const behavioralRecallMinScore = config.autoRecallBehavioral?.recall?.minScore ?? DEFAULT_BEHAVIORAL_RECALL_MIN_SCORE;
      const behavioralRecallMinPromptLength = config.autoRecallBehavioral?.recall?.minPromptLength ?? DEFAULT_BEHAVIORAL_RECALL_MIN_PROMPT_LENGTH;

      const behavioralPromptPlanner = createAutoRecallBehavioralPlanner(
        {
          enabled: true,
          injectMode: behavioralInjectMode,
          dedupeErrorSignals: behavioralDedupeErrorSignals,
          errorReminderMaxEntries: behavioralErrorReminderMaxEntries,
          errorScanMaxChars: DEFAULT_BEHAVIORAL_ERROR_SCAN_MAX_CHARS,
          recall: {
            mode: behavioralRecallMode,
            topK: behavioralRecallTopK,
            includeKinds: behavioralRecallIncludeKinds,
            maxAgeDays: behavioralRecallMaxAgeDays,
            maxEntriesPerKey: behavioralRecallMaxEntriesPerKey,
            minRepeated: behavioralRecallMinRepeated,
            minScore: behavioralRecallMinScore,
            minPromptLength: behavioralRecallMinPromptLength,
          },
        },
        {
          sessionState: sessionExposureState,
          recallBehavioral: async (params) => {
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
                `auto-recall.behavioral-guidance: remote behavioral recall skipped (missing runtime principal: ${resolved.missingPrincipalFields.join(", ")})`
              );
              return [];
            }
            const mode: BackendReflectionRecallMode = params.mode === "durable-only"
              ? "invariant-only"
              : "invariant+derived";
            return await memoryBackendClient.recallReflection(resolved.context, {
              query: String(params.prompt || "behavioral-guidance"),
              mode,
              limit: params.limit,
              includeKinds: params.includeKinds?.map((kind) => kind === "durable" ? "invariant" : "derived"),
              minScore: params.minScore,
            });
          },
          sanitizeForContext,
          logger: api.logger,
        }
      );

      api.on("after_tool_call", (event, ctx) => {
        const sessionKey = typeof ctx.sessionKey === "string" ? ctx.sessionKey : "";
        if (isInternalBehavioralGuidanceSessionKey(sessionKey)) return;
        if (!sessionKey) return;
        behavioralPromptPlanner.captureAfterToolCall(event, sessionKey);
      }, { priority: 15 });

      api.on("before_prompt_build", async (event, ctx) => {
        const sessionKey = typeof ctx.sessionKey === "string" ? ctx.sessionKey : "";
        if (isInternalBehavioralGuidanceSessionKey(sessionKey)) return;
        const prependContext = await behavioralPromptPlanner.buildBeforePromptPrependContext({
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
        behavioralPromptPlanner.clearSession({
          sessionKey,
          sessionId: typeof ctx.sessionId === "string" ? ctx.sessionId : undefined,
        });
        behavioralPromptPlanner.pruneSessionState();
      }, { priority: 20 });
      api.on("before_reset", (_event, ctx) => {
        const sessionKey = typeof ctx.sessionKey === "string" ? ctx.sessionKey.trim() : "";
        if (!sessionKey) return;
        behavioralPromptPlanner.clearSession({
          sessionKey,
          sessionId: typeof ctx.sessionId === "string" ? ctx.sessionId : undefined,
        });
        behavioralPromptPlanner.pruneSessionState();
      }, { priority: 12 });
      api.logger.info("auto-recall.behavioral-guidance: hooks registered (after_tool_call, before_prompt_build[behavioral-guidance,error-detected], before_reset cleanup)");
    }

    if (config.sessionStrategy === "systemSessionMemory") {
      api.logger.info("session-strategy: using systemSessionMemory (plugin behavioral auto-recall hooks disabled)");
    }
    if (config.sessionStrategy === "none") {
      api.logger.info("session-strategy: using none (plugin behavioral auto-recall hooks disabled)");
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

  const autoRecallBehavioralRaw = typeof cfg.autoRecallBehavioral === "object" && cfg.autoRecallBehavioral !== null
    ? cfg.autoRecallBehavioral as Record<string, unknown>
    : null;
  const governanceRaw = typeof cfg.governance === "object" && cfg.governance !== null
    ? cfg.governance as Record<string, unknown>
    : null;
  const distillRaw = typeof cfg.distill === "object" && cfg.distill !== null
    ? cfg.distill as Record<string, unknown>
    : null;
  if (hasOwnKey(cfg as Record<string, unknown>, "sessionMemory")) {
    rejectRemovedConfigField("sessionMemory", "sessionStrategy");
  }
  if (hasOwnKey(cfg as Record<string, unknown>, "memoryReflection")) {
    rejectRemovedConfigField("memoryReflection", "autoRecallBehavioral");
  }
  if (hasOwnKey(cfg as Record<string, unknown>, "selfImprovement")) {
    rejectRemovedConfigField("selfImprovement", "governance or autoRecallBehavioral");
  }
  const sessionStrategyRaw = cfg.sessionStrategy;
  if (sessionStrategyRaw === "memoryReflection") {
    throw new Error(
      "sessionStrategy=memoryReflection is no longer supported in 1.0.0-beta.0; use sessionStrategy=autoRecall"
    );
  }
  const sessionStrategy: SessionStrategy =
    sessionStrategyRaw === "autoRecall" || sessionStrategyRaw === "systemSessionMemory" || sessionStrategyRaw === "none"
      ? sessionStrategyRaw
      : "systemSessionMemory";
  const injectModeRaw = autoRecallBehavioralRaw?.injectMode;
  const behavioralInjectMode: BehavioralGuidanceInjectMode =
    injectModeRaw === "durable-only" || injectModeRaw === "inheritance-only"
      ? "durable-only"
      : injectModeRaw === "durable+adaptive" || injectModeRaw === "inheritance+derived"
        ? "durable+adaptive"
        : "durable+adaptive";
  const autoRecallBehavioralRecallRaw = typeof autoRecallBehavioralRaw?.recall === "object" && autoRecallBehavioralRaw.recall !== null
    ? autoRecallBehavioralRaw.recall as Record<string, unknown>
    : null;
  const behavioralRecallMode: BehavioralRecallMode =
    autoRecallBehavioralRecallRaw?.mode === "dynamic" ? "dynamic" : DEFAULT_BEHAVIORAL_RECALL_MODE;
  const behavioralRecallTopK = parsePositiveInt(autoRecallBehavioralRecallRaw?.topK) ?? DEFAULT_BEHAVIORAL_RECALL_TOP_K;
  const behavioralRecallIncludeKinds = parseBehavioralRecallKinds(
    autoRecallBehavioralRecallRaw?.includeKinds,
    DEFAULT_BEHAVIORAL_RECALL_INCLUDE_KINDS
  );
  const behavioralRecallMaxAgeDays = parsePositiveInt(autoRecallBehavioralRecallRaw?.maxAgeDays) ?? DEFAULT_BEHAVIORAL_RECALL_MAX_AGE_DAYS;
  const behavioralRecallMaxEntriesPerKey = parsePositiveInt(autoRecallBehavioralRecallRaw?.maxEntriesPerKey) ?? DEFAULT_BEHAVIORAL_RECALL_MAX_ENTRIES_PER_KEY;
  const behavioralRecallMinRepeated = parsePositiveInt(autoRecallBehavioralRecallRaw?.minRepeated) ?? DEFAULT_BEHAVIORAL_RECALL_MIN_REPEATED;
  const behavioralRecallMinScore = parseNonNegativeNumber(autoRecallBehavioralRecallRaw?.minScore) ?? DEFAULT_BEHAVIORAL_RECALL_MIN_SCORE;
  const behavioralRecallMinPromptLength = parsePositiveInt(autoRecallBehavioralRecallRaw?.minPromptLength) ?? DEFAULT_BEHAVIORAL_RECALL_MIN_PROMPT_LENGTH;
  if (cfg.autoRecallSelectionMode === "setwise-v2") {
    throw new Error("autoRecallSelectionMode=setwise-v2 is no longer supported; use mmr");
  }
  const autoRecallSelectionMode: AutoRecallSelectionMode =
    cfg.autoRecallSelectionMode === "mmr"
      ? "mmr"
      : DEFAULT_AUTO_RECALL_SELECTION_MODE;
  const distillEveryTurns = parsePositiveInt(distillRaw?.everyTurns) ?? DEFAULT_DISTILL_EVERY_TURNS;
  const governanceEnabled = governanceRaw
    ? governanceRaw.enabled !== false
    : true;
  const ensureGovernanceFiles = typeof autoRecallBehavioralRaw?.ensureGovernanceFiles === "boolean"
    ? autoRecallBehavioralRaw.ensureGovernanceFiles
    : typeof governanceRaw?.ensureBacklogFiles === "boolean"
      ? governanceRaw.ensureBacklogFiles
      : true;
  const behavioralBeforeResetNote = typeof autoRecallBehavioralRaw?.beforeResetNote === "boolean"
    ? autoRecallBehavioralRaw.beforeResetNote
    : true;
  const behavioralSkipSubagentBootstrap = typeof autoRecallBehavioralRaw?.skipSubagentBootstrap === "boolean"
    ? autoRecallBehavioralRaw.skipSubagentBootstrap
    : true;
  const autoRecallExcludeBehavioral = typeof cfg.autoRecallExcludeBehavioral === "boolean"
    ? cfg.autoRecallExcludeBehavioral
    : typeof cfg.autoRecallExcludeReflection === "boolean"
      ? cfg.autoRecallExcludeReflection
      : DEFAULT_AUTO_RECALL_EXCLUDE_BEHAVIORAL;
  const behavioralEnabled = sessionStrategy === "autoRecall" && autoRecallBehavioralRaw?.enabled !== false;

  return {
    autoCapture: cfg.autoCapture !== false,
    // Default OFF: only enable when explicitly set to true.
    autoRecall: cfg.autoRecall === true,
    autoRecallMinLength: parsePositiveInt(cfg.autoRecallMinLength),
    autoRecallMinRepeated: parsePositiveInt(cfg.autoRecallMinRepeated),
    autoRecallTopK: parsePositiveInt(cfg.autoRecallTopK) ?? DEFAULT_AUTO_RECALL_TOP_K,
    autoRecallSelectionMode,
    autoRecallCategories: parseMemoryCategories(cfg.autoRecallCategories, DEFAULT_AUTO_RECALL_CATEGORIES),
    autoRecallExcludeBehavioral,
    autoRecallExcludeReflection: autoRecallExcludeBehavioral,
    autoRecallMaxAgeDays: parsePositiveInt(cfg.autoRecallMaxAgeDays) ?? DEFAULT_AUTO_RECALL_MAX_AGE_DAYS,
    autoRecallMaxEntriesPerKey: parsePositiveInt(cfg.autoRecallMaxEntriesPerKey) ?? DEFAULT_AUTO_RECALL_MAX_ENTRIES_PER_KEY,
    captureAssistant: cfg.captureAssistant === true,
    enableManagementTools: cfg.enableManagementTools === true,
    sessionStrategy,
    distill: distillRaw
      ? {
        enabled: distillRaw.enabled === true,
        mode: normalizeDistillMode(distillRaw.mode),
        persistMode: normalizeDistillPersistMode(distillRaw.persistMode),
        everyTurns: distillEveryTurns,
        maxMessages: parsePositiveInt(distillRaw.maxMessages) ?? DEFAULT_DISTILL_MAX_MESSAGES,
        maxArtifacts: parsePositiveInt(distillRaw.maxArtifacts) ?? DEFAULT_DISTILL_MAX_ARTIFACTS,
        chunkChars: parsePositiveInt(distillRaw.chunkChars) ?? DEFAULT_DISTILL_CHUNK_CHARS,
        chunkOverlapMessages: parsePositiveInt(distillRaw.chunkOverlapMessages) ?? DEFAULT_DISTILL_CHUNK_OVERLAP_MESSAGES,
      }
      : {
        enabled: false,
        mode: DEFAULT_DISTILL_MODE,
        persistMode: DEFAULT_DISTILL_PERSIST_MODE,
        everyTurns: DEFAULT_DISTILL_EVERY_TURNS,
        maxMessages: DEFAULT_DISTILL_MAX_MESSAGES,
        maxArtifacts: DEFAULT_DISTILL_MAX_ARTIFACTS,
        chunkChars: DEFAULT_DISTILL_CHUNK_CHARS,
        chunkOverlapMessages: DEFAULT_DISTILL_CHUNK_OVERLAP_MESSAGES,
      },
    governance: {
      enabled: governanceEnabled,
      ensureBacklogFiles: ensureGovernanceFiles,
    },
    autoRecallBehavioral: {
      enabled: behavioralEnabled,
      injectMode: behavioralInjectMode,
      errorReminderMaxEntries: parsePositiveInt(autoRecallBehavioralRaw?.errorReminderMaxEntries) ?? DEFAULT_BEHAVIORAL_ERROR_REMINDER_MAX_ENTRIES,
      dedupeErrorSignals: autoRecallBehavioralRaw?.dedupeErrorSignals !== false,
      beforeResetNote: behavioralBeforeResetNote,
      skipSubagentBootstrap: behavioralSkipSubagentBootstrap,
      ensureGovernanceFiles,
      recall: {
        mode: behavioralRecallMode,
        topK: behavioralRecallTopK,
        includeKinds: behavioralRecallIncludeKinds,
        maxAgeDays: behavioralRecallMaxAgeDays,
        maxEntriesPerKey: behavioralRecallMaxEntriesPerKey,
        minRepeated: behavioralRecallMinRepeated,
        minScore: behavioralRecallMinScore,
        minPromptLength: behavioralRecallMinPromptLength,
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
