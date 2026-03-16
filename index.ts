/**
 * Memory LanceDB Pro Plugin
 * Remote-backend-authoritative memory plugin with local context orchestration.
 */

import type { OpenClawPluginApi } from "openclaw/plugin-sdk";
import { homedir, tmpdir } from "node:os";
import { join, dirname, basename } from "node:path";
import { readFile, readdir, writeFile, mkdir, appendFile, unlink, stat } from "node:fs/promises";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { pathToFileURL } from "node:url";
import { createRequire } from "node:module";
import { spawn } from "node:child_process";

import { registerRemoteMemoryTools } from "./src/backend-tools.js";
import { ensureSelfImprovementLearningFiles } from "./src/self-improvement-files.js";
import { runWithReflectionTransientRetryOnce } from "./src/reflection-retry.js";
import { resolveReflectionSessionSearchDirs, stripResetSuffix } from "./src/session-recovery.js";
import {
  createSessionExposureState,
  DEFAULT_SESSION_EXPOSURE_MAX_TRACKED_SESSIONS,
  type ReflectionErrorSignal,
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
  sessionMemory?: { enabled?: boolean; messageCount?: number };
  selfImprovement?: {
    enabled?: boolean;
    beforeResetNote?: boolean;
    skipSubagentBootstrap?: boolean;
    ensureLearningFiles?: boolean;
  };
  memoryReflection?: {
    enabled?: boolean;
    injectMode?: ReflectionInjectMode;
    agentId?: string;
    messageCount?: number;
    maxInputChars?: number;
    timeoutMs?: number;
    thinkLevel?: ReflectionThinkLevel;
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

type ReflectionThinkLevel = "off" | "minimal" | "low" | "medium" | "high";
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
const DEFAULT_REFLECTION_MAX_INPUT_CHARS = 24_000;
const DEFAULT_REFLECTION_TIMEOUT_MS = 20_000;
const DEFAULT_REFLECTION_THINK_LEVEL: ReflectionThinkLevel = "medium";
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
const REFLECTION_FALLBACK_MARKER = "(fallback) Reflection generation failed; storing minimal pointer only.";
const DIAG_BUILD_TAG = "memory-lancedb-pro-diag-20260308-0058";

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

type EmbeddedPiRunner = (params: Record<string, unknown>) => Promise<unknown>;

const requireFromHere = createRequire(import.meta.url);
let embeddedPiRunnerPromise: Promise<EmbeddedPiRunner> | null = null;

function toImportSpecifier(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return "";
  if (trimmed.startsWith("file://")) return trimmed;
  if (trimmed.startsWith("/")) return pathToFileURL(trimmed).href;
  return trimmed;
}

function getExtensionApiImportSpecifiers(): string[] {
  const envPath = process.env.OPENCLAW_EXTENSION_API_PATH?.trim();
  const specifiers: string[] = [];

  if (envPath) specifiers.push(toImportSpecifier(envPath));
  specifiers.push("openclaw/dist/extensionAPI.js");

  try {
    specifiers.push(toImportSpecifier(requireFromHere.resolve("openclaw/dist/extensionAPI.js")));
  } catch {
    // ignore resolve failures and continue fallback probing
  }

  specifiers.push(toImportSpecifier("/usr/lib/node_modules/openclaw/dist/extensionAPI.js"));
  specifiers.push(toImportSpecifier("/usr/local/lib/node_modules/openclaw/dist/extensionAPI.js"));

  return [...new Set(specifiers.filter(Boolean))];
}

async function loadEmbeddedPiRunner(): Promise<EmbeddedPiRunner> {
  if (!embeddedPiRunnerPromise) {
    embeddedPiRunnerPromise = (async () => {
      const importErrors: string[] = [];
      for (const specifier of getExtensionApiImportSpecifiers()) {
        try {
          const mod = await import(specifier);
          const runner = (mod as Record<string, unknown>).runEmbeddedPiAgent;
          if (typeof runner === "function") return runner as EmbeddedPiRunner;
          importErrors.push(`${specifier}: runEmbeddedPiAgent export not found`);
        } catch (err) {
          importErrors.push(`${specifier}: ${err instanceof Error ? err.message : String(err)}`);
        }
      }
      throw new Error(
        `Unable to load OpenClaw embedded runtime API. ` +
        `Set OPENCLAW_EXTENSION_API_PATH if runtime layout differs. ` +
        `Attempts: ${importErrors.join(" | ")}`
      );
    })();
  }

  try {
    return await embeddedPiRunnerPromise;
  } catch (err) {
    embeddedPiRunnerPromise = null;
    throw err;
  }
}

function clipDiagnostic(text: string, maxLen = 400): string {
  const oneLine = text.replace(/\s+/g, " ").trim();
  if (oneLine.length <= maxLen) return oneLine;
  return `${oneLine.slice(0, maxLen - 3)}...`;
}

function withTimeout<T>(promise: Promise<T>, timeoutMs: number, label: string): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => {
      reject(new Error(`${label} timed out after ${timeoutMs}ms`));
    }, timeoutMs);

    promise.then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (err) => {
        clearTimeout(timer);
        reject(err);
      }
    );
  });
}

function tryParseJsonObject(raw: string): Record<string, unknown> | null {
  try {
    const parsed = JSON.parse(raw);
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return parsed as Record<string, unknown>;
    }
  } catch {
    // ignore
  }
  return null;
}

function extractJsonObjectFromOutput(stdout: string): Record<string, unknown> {
  const trimmed = stdout.trim();
  if (!trimmed) throw new Error("empty stdout");

  const direct = tryParseJsonObject(trimmed);
  if (direct) return direct;

  const lines = trimmed.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    if (!lines[i].trim().startsWith("{")) continue;
    const candidate = lines.slice(i).join("\n");
    const parsed = tryParseJsonObject(candidate);
    if (parsed) return parsed;
  }

  throw new Error(`unable to parse JSON from CLI output: ${clipDiagnostic(trimmed, 280)}`);
}

function extractReflectionTextFromCliResult(resultObj: Record<string, unknown>): string | null {
  const result = resultObj.result as Record<string, unknown> | undefined;
  const payloads = Array.isArray(resultObj.payloads)
    ? resultObj.payloads
    : Array.isArray(result?.payloads)
      ? result.payloads
      : [];
  const firstWithText = payloads.find(
    (p) => p && typeof p === "object" && typeof (p as Record<string, unknown>).text === "string" && ((p as Record<string, unknown>).text as string).trim().length
  ) as Record<string, unknown> | undefined;
  const text = typeof firstWithText?.text === "string" ? firstWithText.text.trim() : "";
  return text || null;
}

async function runReflectionViaCli(params: {
  prompt: string;
  agentId: string;
  workspaceDir: string;
  timeoutMs: number;
  thinkLevel: ReflectionThinkLevel;
}): Promise<string> {
  const cliBin = process.env.OPENCLAW_CLI_BIN?.trim() || "openclaw";
  const outerTimeoutMs = Math.max(params.timeoutMs + 5000, 15000);
  const agentTimeoutSec = Math.max(1, Math.ceil(params.timeoutMs / 1000));
  const sessionId = `memory-reflection-cli-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

  const args = [
    "agent",
    "--local",
    "--agent",
    params.agentId,
    "--message",
    params.prompt,
    "--json",
    "--thinking",
    params.thinkLevel,
    "--timeout",
    String(agentTimeoutSec),
    "--session-id",
    sessionId,
  ];

  return await new Promise<string>((resolve, reject) => {
    const child = spawn(cliBin, args, {
      cwd: params.workspaceDir,
      env: { ...process.env, NO_COLOR: "1" },
      stdio: ["ignore", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";
    let settled = false;
    let timedOut = false;

    const timer = setTimeout(() => {
      timedOut = true;
      child.kill("SIGTERM");
      setTimeout(() => child.kill("SIGKILL"), 1500).unref();
    }, outerTimeoutMs);

    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });

    child.stderr.setEncoding("utf8");
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });

    child.once("error", (err) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      reject(new Error(`spawn ${cliBin} failed: ${err.message}`));
    });

    child.once("close", (code, signal) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);

      if (timedOut) {
        reject(new Error(`${cliBin} timed out after ${outerTimeoutMs}ms`));
        return;
      }
      if (signal) {
        reject(new Error(`${cliBin} exited by signal ${signal}. stderr=${clipDiagnostic(stderr)}`));
        return;
      }
      if (code !== 0) {
        reject(new Error(`${cliBin} exited with code ${code}. stderr=${clipDiagnostic(stderr)}`));
        return;
      }

      try {
        const parsed = extractJsonObjectFromOutput(stdout);
        const text = extractReflectionTextFromCliResult(parsed);
        if (!text) {
          reject(new Error(`CLI JSON returned no text payload. stdout=${clipDiagnostic(stdout)}`));
          return;
        }
        resolve(text);
      } catch (err) {
        reject(err instanceof Error ? err : new Error(String(err)));
      }
    });
  });
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

function resolveAgentPrimaryModelRef(cfg: unknown, agentId: string): string | undefined {
  try {
    const root = cfg as Record<string, unknown>;
    const agents = root.agents as Record<string, unknown> | undefined;
    const list = agents?.list as unknown;

    if (Array.isArray(list)) {
      const found = list.find((x) => {
        if (!x || typeof x !== "object") return false;
        return (x as Record<string, unknown>).id === agentId;
      }) as Record<string, unknown> | undefined;
      const model = found?.model as Record<string, unknown> | undefined;
      const primary = model?.primary;
      if (typeof primary === "string" && primary.trim()) return primary.trim();
    }

    const defaults = agents?.defaults as Record<string, unknown> | undefined;
    const defModel = defaults?.model as Record<string, unknown> | undefined;
    const defPrimary = defModel?.primary;
    if (typeof defPrimary === "string" && defPrimary.trim()) return defPrimary.trim();
  } catch {
    // ignore
  }
  return undefined;
}

function splitProviderModel(modelRef: string): { provider?: string; model?: string } {
  const s = modelRef.trim();
  if (!s) return {};
  const idx = s.indexOf("/");
  if (idx > 0) {
    const provider = s.slice(0, idx).trim();
    const model = s.slice(idx + 1).trim();
    return { provider: provider || undefined, model: model || undefined };
  }
  return { model: s };
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

function parseConversationToCaptureItems(conversation: string): BackendCaptureItem[] {
  const rows: BackendCaptureItem[] = [];
  const lines = conversation
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);

  for (const line of lines) {
    const match = line.match(/^(user|assistant)\s*:\s*(.+)$/i);
    if (!match) continue;
    const roleRaw = match[1].toLowerCase();
    const text = match[2].trim();
    if (!text) continue;
    rows.push({
      role: roleRaw === "assistant" ? "assistant" : "user",
      text,
    });
  }
  return rows;
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

function shouldSkipReflectionMessage(role: string, text: string): boolean {
  const trimmed = text.trim();
  if (!trimmed) return true;
  if (trimmed.startsWith("/")) return true;

  if (role === "user") {
    if (
      trimmed.includes("<relevant-memories>") ||
      trimmed.includes("UNTRUSTED DATA") ||
      trimmed.includes("END UNTRUSTED DATA")
    ) {
      return true;
    }
  }

  return false;
}

function redactSecrets(text: string): string {
  const patterns: RegExp[] = [
    /Bearer\s+[A-Za-z0-9\-._~+/]+=*/g,
    /\bsk-[A-Za-z0-9]{20,}\b/g,
    /\bsk-proj-[A-Za-z0-9\-_]{20,}\b/g,
    /\bsk-ant-[A-Za-z0-9\-_]{20,}\b/g,
    /\bghp_[A-Za-z0-9]{36,}\b/g,
    /\bgho_[A-Za-z0-9]{36,}\b/g,
    /\bghu_[A-Za-z0-9]{36,}\b/g,
    /\bghs_[A-Za-z0-9]{36,}\b/g,
    /\bgithub_pat_[A-Za-z0-9_]{22,}\b/g,
    /\bxox[baprs]-[A-Za-z0-9-]{10,}\b/g,
    /\bAIza[0-9A-Za-z_-]{20,}\b/g,
    /\bAKIA[0-9A-Z]{16}\b/g,
    /\bnpm_[A-Za-z0-9]{36,}\b/g,
    /\b(?:token|api[_-]?key|secret|password)\s*[:=]\s*["']?[^\s"',;)}\]]{6,}["']?\b/gi,
    /-----BEGIN\s+(?:RSA\s+|EC\s+|DSA\s+|OPENSSH\s+)?PRIVATE\s+KEY-----[\s\S]*?-----END\s+(?:RSA\s+|EC\s+|DSA\s+|OPENSSH\s+)?PRIVATE\s+KEY-----/g,
    /(?<=:\/\/)[^@\s]+:[^@\s]+(?=@)/g,
    /\/home\/[^\s"',;)}\]]+/g,
    /\/Users\/[^\s"',;)}\]]+/g,
    /[A-Z]:\\[^\s"',;)}\]]+/g,
    /[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/g,
  ];

  let out = text;
  for (const re of patterns) {
    out = out.replace(re, (m) => (m.startsWith("Bearer") || m.startsWith("bearer") ? "Bearer [REDACTED]" : "[REDACTED]"));
  }
  return out;
}

function sha256Hex(text: string): string {
  return createHash("sha256").update(text, "utf8").digest("hex");
}

async function readSessionConversationForReflection(filePath: string, messageCount: number): Promise<string | null> {
  try {
    const lines = (await readFile(filePath, "utf-8")).trim().split("\n");
    const messages: string[] = [];

    for (const line of lines) {
      try {
        const entry = JSON.parse(line);
        if (entry?.type !== "message" || !entry?.message) continue;

        const msg = entry.message as Record<string, unknown>;
        const role = typeof msg.role === "string" ? msg.role : "";
        if (role !== "user" && role !== "assistant") continue;

        const text = extractTextContent(msg.content);
        if (!text || shouldSkipReflectionMessage(role, text)) continue;

        messages.push(`${role}: ${redactSecrets(text)}`);
      } catch {
        // ignore JSON parse errors
      }
    }

    if (messages.length === 0) return null;
    return messages.slice(-messageCount).join("\n");
  } catch {
    return null;
  }
}

export async function readSessionConversationWithResetFallback(sessionFilePath: string, messageCount: number): Promise<string | null> {
  const primary = await readSessionConversationForReflection(sessionFilePath, messageCount);
  if (primary) return primary;

  try {
    const dir = dirname(sessionFilePath);
    const resetPrefix = `${basename(sessionFilePath)}.reset.`;
    const files = await readdir(dir);
    const resetCandidates = await sortFileNamesByMtimeDesc(
      dir,
      files.filter((name) => name.startsWith(resetPrefix))
    );
    if (resetCandidates.length > 0) {
      const latestResetPath = join(dir, resetCandidates[0]);
      return await readSessionConversationForReflection(latestResetPath, messageCount);
    }
  } catch {
    // ignore
  }

  return primary;
}

async function ensureDailyLogFile(dailyPath: string, dateStr: string): Promise<void> {
  try {
    await readFile(dailyPath, "utf-8");
  } catch {
    await writeFile(dailyPath, `# ${dateStr}\n\n`, "utf-8");
  }
}

function buildReflectionPrompt(
  conversation: string,
  maxInputChars: number,
  toolErrorSignals: ReflectionErrorSignal[] = []
): string {
  const clipped = conversation.slice(-maxInputChars);
  const errorHints = toolErrorSignals.length > 0
    ? toolErrorSignals
      .map((e, i) => `${i + 1}. [${e.toolName}] ${e.summary} (sig:${e.signatureHash.slice(0, 8)})`)
      .join("\n")
    : "- (none)";
  return [
    "You are generating a durable MEMORY REFLECTION entry for an AI assistant system.",
    "",
    "Output Markdown only. No intro text. No outro text. No extra headings.",
    "",
    "Use these headings exactly once, in this exact order, with exact spelling:",
    "## Context (session background)",
    "## Decisions (durable)",
    "## User model deltas (about the human)",
    "## Agent model deltas (about the assistant/system)",
    "## Lessons & pitfalls (symptom / cause / fix / prevention)",
    "## Learning governance candidates (.learnings / promotion / skill extraction)",
    "## Open loops / next actions",
    "## Retrieval tags / keywords",
    "## Invariants",
    "## Derived",
    "",
    "Hard rules:",
    "- Do not rename, translate, merge, reorder, or omit headings.",
    "- Every section must appear exactly once.",
    "- For bullet sections, use one item per line, starting with '- '.",
    "- Do not wrap one bullet across multiple lines.",
    "- If a bullet section is empty, write exactly: '- (none captured)'",
    "- Do not paste raw transcript.",
    "- Do not invent Logged timestamps, ids, file paths, commit hashes, session ids, or storage metadata unless they already appear in the input.",
    "- If secrets/tokens/passwords appear, keep them as [REDACTED].",
    "",
    "Section rules:",
    "- Context / Decisions / User model / Agent model / Open loops / Retrieval tags / Invariants / Derived = bullet lists only.",
    "- Lessons & pitfalls = bullet list only; each bullet must be one single line in this shape:",
    "  - Symptom: ... Cause: ... Fix: ... Prevention: ...",
    "- Invariants = stable cross-session rules only; prefer bullets starting with Always / Never / When / If / Before / After / Prefer / Avoid / Require.",
    "- Derived = recent-run distilled learnings, adjustments, and follow-up heuristics that may help the next several runs, but should decay over time.",
    "- Keep Invariants stable and long-lived; keep Derived recent, reusable across near-term runs, and decayable.",
    "- Start Derived bullets with varied lead-ins (for example: Next run..., When..., If..., To avoid...) instead of repeating one opening phrase.",
    "- Keep Derived phrasing non-redundant; do not start every bullet with the same words.",
    "- Do not restate long-term rules in Derived.",
    "",
    "Governance section rules:",
    "- If empty, write exactly:",
    "  - (none captured)",
    "- Otherwise, do NOT use bullet lists there.",
    "- Use one or more entries in exactly this format:",
    "",
    "### Entry 1",
    "**Priority**: low|medium|high|critical",
    "**Status**: pending|triage|promoted_to_skill|done",
    "**Area**: frontend|backend|infra|tests|docs|config|<custom area>",
    "### Summary",
    "<one concise candidate>",
    "### Details",
    "<short supporting details>",
    "### Suggested Action",
    "<one concrete next action>",
    "",
    "Notes:",
    "- Keep writer-owned metadata out of the output. The writer generates Logged and IDs.",
    "- Prefer structured, machine-parseable output over elegant prose.",
    "",
    "OUTPUT TEMPLATE (copy this structure exactly):",
    "## Context (session background)",
    "- ...",
    "",
    "## Decisions (durable)",
    "- ...",
    "",
    "## User model deltas (about the human)",
    "- ...",
    "",
    "## Agent model deltas (about the assistant/system)",
    "- ...",
    "",
    "## Lessons & pitfalls (symptom / cause / fix / prevention)",
    "- Symptom: ... Cause: ... Fix: ... Prevention: ...",
    "",
    "## Learning governance candidates (.learnings / promotion / skill extraction)",
    "### Entry 1",
    "**Priority**: medium",
    "**Status**: pending",
    "**Area**: config",
    "### Summary",
    "...",
    "### Details",
    "...",
    "### Suggested Action",
    "...",
    "",
    "## Open loops / next actions",
    "- ...",
    "",
    "## Retrieval tags / keywords",
    "- ...",
    "",
    "## Invariants",
    "- Always ...",
    "",
    "## Derived",
    "- Next run, ...",
    "",
    "Recent tool error signals:",
    errorHints,
    "",
    "INPUT:",
    "```",
    clipped,
    "```",
  ].join("\n");
}

function buildReflectionFallbackText(): string {
  return [
    "## Context (session background)",
    `- ${REFLECTION_FALLBACK_MARKER}`,
    "",
    "## Decisions (durable)",
    "- (none captured)",
    "",
    "## User model deltas (about the human)",
    "- (none captured)",
    "",
    "## Agent model deltas (about the assistant/system)",
    "- (none captured)",
    "",
    "## Lessons & pitfalls (symptom / cause / fix / prevention)",
    "- (none captured)",
    "",
    "## Learning governance candidates (.learnings / promotion / skill extraction)",
    "### Entry 1",
    "**Priority**: medium",
    "**Status**: triage",
    "**Area**: config",
    "### Summary",
    "Investigate last failed tool execution and decide whether it belongs in .learnings/ERRORS.md.",
    "### Details",
    "The reflection pipeline fell back; confirm the failure is reproducible before treating it as a durable error record.",
    "### Suggested Action",
    "Reproduce the latest failed tool execution, classify it as triage or error, and then log it with the appropriate tool/file path evidence.",
    "",
    "## Open loops / next actions",
    "- Investigate why embedded reflection generation failed.",
    "",
    "## Retrieval tags / keywords",
    "- memory-reflection",
    "",
    "## Invariants",
    "- (none captured)",
    "",
    "## Derived",
    "- If embedded reflection generation fails again, investigate root cause before trusting any next-run delta.",
  ].join("\n");
}

async function generateReflectionText(params: {
  conversation: string;
  maxInputChars: number;
  cfg: unknown;
  agentId: string;
  workspaceDir: string;
  timeoutMs: number;
  thinkLevel: ReflectionThinkLevel;
  toolErrorSignals?: ReflectionErrorSignal[];
  logger?: { info?: (message: string) => void; warn?: (message: string) => void };
}): Promise<{ text: string; usedFallback: boolean; promptHash: string; error?: string; runner: "embedded" | "cli" | "fallback" }> {
  const prompt = buildReflectionPrompt(
    params.conversation,
    params.maxInputChars,
    params.toolErrorSignals ?? []
  );
  const promptHash = sha256Hex(prompt);
  const tempSessionFile = join(
    tmpdir(),
    `memory-reflection-${Date.now()}-${Math.random().toString(36).slice(2)}.jsonl`
  );
  let reflectionText: string | null = null;
  const errors: string[] = [];
  const retryState = { count: 0 };
  const onRetryLog = (level: "info" | "warn", message: string) => {
    if (level === "warn") params.logger?.warn?.(message);
    else params.logger?.info?.(message);
  };

  try {
    const result: unknown = await runWithReflectionTransientRetryOnce({
      scope: "reflection",
      runner: "embedded",
      retryState,
      onLog: onRetryLog,
      execute: async () => {
        const runEmbeddedPiAgent = await loadEmbeddedPiRunner();
        const modelRef = resolveAgentPrimaryModelRef(params.cfg, params.agentId);
        const { provider, model } = modelRef ? splitProviderModel(modelRef) : {};
        const embeddedTimeoutMs = Math.max(params.timeoutMs + 5000, 15000);

        return await withTimeout(
          runEmbeddedPiAgent({
            sessionId: `reflection-${Date.now()}`,
            sessionKey: "temp:memory-reflection",
            agentId: params.agentId,
            sessionFile: tempSessionFile,
            workspaceDir: params.workspaceDir,
            config: params.cfg,
            prompt,
            disableTools: true,
            disableMessageTool: true,
            timeoutMs: params.timeoutMs,
            runId: `memory-reflection-${Date.now()}`,
            bootstrapContextMode: "lightweight",
            thinkLevel: params.thinkLevel,
            provider,
            model,
          }),
          embeddedTimeoutMs,
          "embedded reflection run"
        );
      },
    });

    const payloads = (() => {
      if (!result || typeof result !== "object") return [];
      const maybePayloads = (result as Record<string, unknown>).payloads;
      return Array.isArray(maybePayloads) ? maybePayloads : [];
    })();

    if (payloads.length > 0) {
      const firstWithText = payloads.find((p) => {
        if (!p || typeof p !== "object") return false;
        const text = (p as Record<string, unknown>).text;
        return typeof text === "string" && text.trim().length > 0;
      }) as Record<string, unknown> | undefined;
      reflectionText = typeof firstWithText?.text === "string" ? firstWithText.text.trim() : null;
    }
  } catch (err) {
    errors.push(`embedded: ${err instanceof Error ? `${err.name}: ${err.message}` : String(err)}`);
  } finally {
    await unlink(tempSessionFile).catch(() => { });
  }

  if (reflectionText) {
    return { text: reflectionText, usedFallback: false, promptHash, error: errors[0], runner: "embedded" };
  }

  try {
    reflectionText = await runWithReflectionTransientRetryOnce({
      scope: "reflection",
      runner: "cli",
      retryState,
      onLog: onRetryLog,
      execute: async () => await runReflectionViaCli({
        prompt,
        agentId: params.agentId,
        workspaceDir: params.workspaceDir,
        timeoutMs: params.timeoutMs,
        thinkLevel: params.thinkLevel,
      }),
    });
  } catch (err) {
    errors.push(`cli: ${err instanceof Error ? err.message : String(err)}`);
  }

  if (reflectionText) {
    return {
      text: reflectionText,
      usedFallback: false,
      promptHash,
      error: errors.length > 0 ? errors.join(" | ") : undefined,
      runner: "cli",
    };
  }

  return {
    text: buildReflectionFallbackText(),
    usedFallback: true,
    promptHash,
    error: errors.length > 0 ? errors.join(" | ") : undefined,
    runner: "fallback",
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
// Session Path Helpers
// ============================================================================

async function sortFileNamesByMtimeDesc(dir: string, fileNames: string[]): Promise<string[]> {
  const candidates = await Promise.all(
    fileNames.map(async (name) => {
      try {
        const st = await stat(join(dir, name));
        return { name, mtimeMs: st.mtimeMs };
      } catch {
        return null;
      }
    })
  );

  return candidates
    .filter((x): x is { name: string; mtimeMs: number } => x !== null)
    .sort((a, b) => (b.mtimeMs - a.mtimeMs) || b.name.localeCompare(a.name))
    .map((x) => x.name);
}

function sanitizeFileToken(value: string, fallback: string): string {
  const normalized = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 32);
  return normalized || fallback;
}

async function findPreviousSessionFile(
  sessionsDir: string,
  currentSessionFile?: string,
  sessionId?: string,
): Promise<string | undefined> {
  try {
    const files = await readdir(sessionsDir);
    const fileSet = new Set(files);

    // Try recovering the non-reset base file
    const baseFromReset = currentSessionFile
      ? stripResetSuffix(basename(currentSessionFile))
      : undefined;
    if (baseFromReset && fileSet.has(baseFromReset))
      return join(sessionsDir, baseFromReset);

    // Try canonical session ID file
    const trimmedId = sessionId?.trim();
    if (trimmedId) {
      const canonicalFile = `${trimmedId}.jsonl`;
      if (fileSet.has(canonicalFile)) return join(sessionsDir, canonicalFile);

      // Try topic variants
      const topicVariants = await sortFileNamesByMtimeDesc(
        sessionsDir,
        files.filter(
          (name) =>
            name.startsWith(`${trimmedId}-topic-`) &&
            name.endsWith(".jsonl") &&
            !name.includes(".reset."),
        )
      );
      if (topicVariants.length > 0) return join(sessionsDir, topicVariants[0]);
    }

    // Fallback to most recent non-reset JSONL
    if (currentSessionFile) {
      const nonReset = await sortFileNamesByMtimeDesc(
        sessionsDir,
        files.filter((name) => name.endsWith(".jsonl") && !name.includes(".reset."))
      );
      if (nonReset.length > 0) return join(sessionsDir, nonReset[0]);
    }
  } catch { }
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

const memoryLanceDBProPlugin = {
  id: "memory-lancedb-pro",
  name: "Memory (LanceDB Pro)",
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
      `memory-lancedb-pro@${pluginVersion}: plugin registered ` +
      `(mode: remote-backend, authority: backend-owned)`,
    );
    api.logger.info(`memory-lancedb-pro: diagnostic build tag loaded (${DIAG_BUILD_TAG})`);
    api.logger.info(
      `memory-lancedb-pro: remote backend enabled (${config.remoteBackend?.baseURL || "(missing baseURL)"})`
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
        enableSelfImprovementTools: config.selfImprovement?.enabled !== false,
        defaultWorkspaceDir: getDefaultWorkspaceDir(),
      }
    );

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
              `memory-lancedb-pro: auto-recall skipped remote recall (missing runtime principal: ${resolved.missingPrincipalFields.join(", ")})`
            );
            return [];
          }
          return await memoryBackendClient.recallGeneric(resolved.context, {
            query: params.query,
            limit: params.limit,
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
          api.logger.warn(`memory-lancedb-pro: auto-recall failed: ${String(err)}`);
        }
      });
    }

    // Auto-capture: analyze and store important information after agent ends
    if (config.autoCapture !== false) {
      api.on("agent_end", async (event, ctx) => {
        if (!event.success || !event.messages || event.messages.length === 0) {
          return;
        }

        try {
          const captureItems: BackendCaptureItem[] = [];
          for (const msg of event.messages) {
            if (!msg || typeof msg !== "object") continue;
            const msgObj = msg as Record<string, unknown>;
            const roleRaw = typeof msgObj.role === "string" ? msgObj.role.toLowerCase() : "";
            if (roleRaw !== "user" && !(config.captureAssistant === true && roleRaw === "assistant")) {
              continue;
            }
            const text = extractTextContent(msgObj.content);
            if (!text || !text.trim()) continue;
            captureItems.push({
              role: roleRaw === "assistant" ? "assistant" : "user",
              text: text.trim(),
            });
          }

          if (captureItems.length === 0) {
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
              `memory-lancedb-pro: auto-capture blocked (missing runtime principal: ${resolvedBackendCtx.missingPrincipalFields.join(", ")})`
            );
            return;
          }
          const result = await memoryBackendClient.storeAutoCapture(resolvedBackendCtx.context, {
            items: captureItems.slice(0, 64),
          });
          api.logger.info(
            `memory-lancedb-pro: auto-capture forwarded to remote backend (${captureItems.length} item(s), mutations=${result.length})`
          );
        } catch (err) {
          api.logger.warn(`memory-lancedb-pro: capture failed: ${String(err)}`);
        }
      });
    }

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
            `memory-lancedb-pro: failed to register ${eventName} via api.on, continue fallback: ${String(err)}`,
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
            `memory-lancedb-pro: failed to register ${eventName} via api.registerHook: ${String(err)}`,
          );
        }
      }

      if (!registeredViaEventBus && !registeredViaInternalHook) {
        api.logger.warn(
          `memory-lancedb-pro: command hook registration failed for ${eventName}; no compatible API method available`,
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
        name: "memory-lancedb-pro.self-improvement.agent-bootstrap",
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
          name: "memory-lancedb-pro.self-improvement.command-new",
          description: "Append self-improvement note before /new",
        } as const;
        const selfImprovementResetHookOptions = {
          name: "memory-lancedb-pro.self-improvement.command-reset",
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

      const parseSessionIdFromSessionFile = (sessionFile: string | undefined): string | undefined => {
        if (!sessionFile) return undefined;
        const fileName = basename(sessionFile);
        const stripped = fileName.replace(/\.jsonl(?:\.reset\..+)?$/i, "");
        if (!stripped || stripped === fileName) return undefined;
        return stripped;
      };

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
          let currentSessionFile = typeof sessionEntry.sessionFile === "string" ? sessionEntry.sessionFile : undefined;
          const runtimeAgentId =
            asNonEmptyString(typeof event?.agentId === "string" ? event.agentId : undefined) ??
            asNonEmptyString(typeof context.agentId === "string" ? context.agentId : undefined);
          const sourceWorkspaceDir = runtimeAgentId
            ? (agentWorkspaceMap[runtimeAgentId] || workspaceDir)
            : workspaceDir;
          const commandSource = typeof context.commandSource === "string" ? context.commandSource : "";
          const triggerKey = `${trigger}|${sessionKey || "(none)"}|${currentSessionFile || currentSessionId || "unknown"}`;
          if (isDuplicateReflectionTrigger(triggerKey)) {
            api.logger.info(`memory-reflection: duplicate trigger skipped; key=${triggerKey}`);
            return;
          }
          api.logger.info(
            `memory-reflection: ${commandName} enqueue start; sessionKey=${sessionKey || "(none)"}; source=${commandSource || "(unknown)"}; sessionId=${currentSessionId}; sessionFile=${currentSessionFile || "(none)"}`
          );

          if ((!currentSessionFile || currentSessionFile.includes(".reset.")) && cfg) {
            const searchDirs = resolveReflectionSessionSearchDirs({
              context,
              cfg,
              workspaceDir: sourceWorkspaceDir,
              currentSessionFile,
              sourceAgentId: runtimeAgentId,
            });
            for (const sessionsDir of searchDirs) {
              const recovered = await findPreviousSessionFile(sessionsDir, currentSessionFile, currentSessionId);
              if (recovered) {
                currentSessionFile = recovered;
                break;
              }
            }
          }

          let captureItems: BackendCaptureItem[] = [];
          if (currentSessionFile) {
            const conversation = await readSessionConversationWithResetFallback(currentSessionFile, reflectionMessageCount);
            if (conversation) {
              captureItems = parseConversationToCaptureItems(conversation);
            }
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
        name: "memory-lancedb-pro.memory-reflection.command-new",
        description: "Run reflection pipeline before /new",
      } as const;
      const memoryReflectionResetHookOptions = {
        name: "memory-lancedb-pro.memory-reflection.command-reset",
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
          const sessionFile = typeof event.sessionFile === "string" ? event.sessionFile : undefined;
          const sessionId = parseSessionIdFromSessionFile(sessionFile) ?? "unknown";
          await runMemoryReflection({
            action: trigger,
            sessionKey: typeof ctx.sessionKey === "string" ? ctx.sessionKey : "",
            timestamp: Date.now(),
            messages: Array.isArray(event.messages) ? event.messages : [],
            context: {
              cfg: api.config,
              workspaceDir: ctx.workspaceDir,
              commandSource: `lifecycle:before_reset:${trigger}`,
              sessionEntry: {
                sessionId,
                sessionFile,
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
      id: "memory-lancedb-pro",
      start: async () => {
        api.logger.info("memory-lancedb-pro: remote backend mode active");
      },
      stop: async () => {
        api.logger.info("memory-lancedb-pro: stopped (remote backend mode)");
      },
    });
  },
};

export function parsePluginConfig(value: unknown): PluginConfig {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("memory-lancedb-pro config required");
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
  const sessionMemoryRaw = typeof cfg.sessionMemory === "object" && cfg.sessionMemory !== null
    ? cfg.sessionMemory as Record<string, unknown>
    : null;
  const sessionStrategyRaw = cfg.sessionStrategy;
  const legacySessionMemoryEnabled = typeof sessionMemoryRaw?.enabled === "boolean"
    ? sessionMemoryRaw.enabled
    : undefined;
  const sessionStrategy: SessionStrategy =
    sessionStrategyRaw === "systemSessionMemory" || sessionStrategyRaw === "memoryReflection" || sessionStrategyRaw === "none"
      ? sessionStrategyRaw
      : legacySessionMemoryEnabled === true
        ? "systemSessionMemory"
        : legacySessionMemoryEnabled === false
          ? "none"
          : "systemSessionMemory";
  const reflectionMessageCount = parsePositiveInt(memoryReflectionRaw?.messageCount ?? sessionMemoryRaw?.messageCount) ?? DEFAULT_REFLECTION_MESSAGE_COUNT;
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
        agentId: asNonEmptyString(memoryReflectionRaw.agentId),
        messageCount: reflectionMessageCount,
        maxInputChars: parsePositiveInt(memoryReflectionRaw.maxInputChars) ?? DEFAULT_REFLECTION_MAX_INPUT_CHARS,
        timeoutMs: parsePositiveInt(memoryReflectionRaw.timeoutMs) ?? DEFAULT_REFLECTION_TIMEOUT_MS,
        thinkLevel: (() => {
          const raw = memoryReflectionRaw.thinkLevel;
          if (raw === "off" || raw === "minimal" || raw === "low" || raw === "medium" || raw === "high") return raw;
          return DEFAULT_REFLECTION_THINK_LEVEL;
        })(),
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
        agentId: undefined,
        messageCount: reflectionMessageCount,
        maxInputChars: DEFAULT_REFLECTION_MAX_INPUT_CHARS,
        timeoutMs: DEFAULT_REFLECTION_TIMEOUT_MS,
        thinkLevel: DEFAULT_REFLECTION_THINK_LEVEL,
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
    sessionMemory:
      typeof cfg.sessionMemory === "object" && cfg.sessionMemory !== null
        ? {
          enabled:
            (cfg.sessionMemory as Record<string, unknown>).enabled !== false,
          messageCount:
            typeof (cfg.sessionMemory as Record<string, unknown>)
              .messageCount === "number"
              ? ((cfg.sessionMemory as Record<string, unknown>)
                .messageCount as number)
              : undefined,
        }
        : undefined,
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

export default memoryLanceDBProPlugin;
