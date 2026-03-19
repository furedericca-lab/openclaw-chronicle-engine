import { Type } from "@sinclair/typebox";
import { stringEnum } from "openclaw/plugin-sdk";
import type { OpenClawPluginApi } from "openclaw/plugin-sdk";
import { appendFile, mkdir, readFile, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";

export const CANONICAL_GOVERNANCE_DIRNAME = ".governance";
const LEGACY_GOVERNANCE_DIRNAME = ".learnings";

export const DEFAULT_GOVERNANCE_LEARNINGS_TEMPLATE = `# Learnings

Append structured entries:
- LRN-YYYYMMDD-001 for corrections / best practices / knowledge gaps
- Include summary, details, suggested action, metadata, and status`;

export const DEFAULT_GOVERNANCE_ERRORS_TEMPLATE = `# Errors

Append structured entries:
- ERR-YYYYMMDD-001 for command/tool/integration failures
- Include symptom, context, probable cause, and prevention`;

const fileWriteQueues = new Map<string, Promise<void>>();

async function withFileWriteQueue<T>(filePath: string, action: () => Promise<T>): Promise<T> {
  const previous = fileWriteQueues.get(filePath) ?? Promise.resolve();
  let release: (() => void) | undefined;
  const lock = new Promise<void>((resolve) => {
    release = resolve;
  });
  const next = previous.then(() => lock);
  fileWriteQueues.set(filePath, next);

  await previous;
  try {
    return await action();
  } finally {
    release?.();
    if (fileWriteQueues.get(filePath) === next) {
      fileWriteQueues.delete(filePath);
    }
  }
}

function todayYmd(): string {
  return new Date().toISOString().slice(0, 10).replace(/-/g, "");
}

async function nextGovernanceEntryId(filePath: string, prefix: "LRN" | "ERR"): Promise<string> {
  const date = todayYmd();
  let count = 0;
  try {
    const content = await readFile(filePath, "utf-8");
    const matches = content.match(new RegExp(`\\[${prefix}-${date}-\\d{3}\\]`, "g"));
    count = matches?.length ?? 0;
  } catch {
    // ignore
  }
  return `${prefix}-${date}-${String(count + 1).padStart(3, "0")}`;
}

function canonicalGovernanceDir(baseDir: string): string {
  return join(baseDir, CANONICAL_GOVERNANCE_DIRNAME);
}

function legacyGovernanceDir(baseDir: string): string {
  return join(baseDir, LEGACY_GOVERNANCE_DIRNAME);
}

async function maybeImportLegacyGovernanceFile(baseDir: string, fileName: string): Promise<void> {
  const canonicalPath = join(canonicalGovernanceDir(baseDir), fileName);
  const legacyPath = join(legacyGovernanceDir(baseDir), fileName);
  try {
    const existing = await readFile(canonicalPath, "utf-8");
    if (existing.trim().length > 0) return;
  } catch {
    // allow import attempt below
  }

  try {
    const legacy = await readFile(legacyPath, "utf-8");
    if (!legacy.trim()) return;
    await writeFile(canonicalPath, legacy.endsWith("\n") ? legacy : `${legacy}\n`, "utf-8");
  } catch {
    // legacy file missing or unreadable, ignore
  }
}

export async function ensureGovernanceBacklogFiles(baseDir: string): Promise<void> {
  const governanceDir = canonicalGovernanceDir(baseDir);
  await mkdir(governanceDir, { recursive: true });

  await maybeImportLegacyGovernanceFile(baseDir, "LEARNINGS.md");
  await maybeImportLegacyGovernanceFile(baseDir, "ERRORS.md");

  const ensureFile = async (filePath: string, content: string) => {
    try {
      const existing = await readFile(filePath, "utf-8");
      if (existing.trim().length > 0) return;
    } catch {
      // write default below
    }
    await writeFile(filePath, `${content.trim()}\n`, "utf-8");
  };

  await ensureFile(join(governanceDir, "LEARNINGS.md"), DEFAULT_GOVERNANCE_LEARNINGS_TEMPLATE);
  await ensureFile(join(governanceDir, "ERRORS.md"), DEFAULT_GOVERNANCE_ERRORS_TEMPLATE);
}

export interface AppendGovernanceEntryParams {
  baseDir: string;
  type: "learning" | "error";
  summary: string;
  details?: string;
  suggestedAction?: string;
  category?: string;
  area?: string;
  priority?: string;
  status?: string;
  source?: string;
}

export async function appendGovernanceEntry(params: AppendGovernanceEntryParams): Promise<{
  id: string;
  filePath: string;
}> {
  const {
    baseDir,
    type,
    summary,
    details = "",
    suggestedAction = "",
    category = "best_practice",
    area = "config",
    priority = "medium",
    status = "pending",
    source = "openclaw-chronicle-engine/governance_log",
  } = params;

  await ensureGovernanceBacklogFiles(baseDir);
  const governanceDir = canonicalGovernanceDir(baseDir);
  const fileName = type === "learning" ? "LEARNINGS.md" : "ERRORS.md";
  const filePath = join(governanceDir, fileName);
  const idPrefix = type === "learning" ? "LRN" : "ERR";

  const id = await withFileWriteQueue(filePath, async () => {
    const entryId = await nextGovernanceEntryId(filePath, idPrefix);
    const nowIso = new Date().toISOString();
    const titleSuffix = type === "learning" ? ` ${category}` : "";
    const entry = [
      `## [${entryId}]${titleSuffix}`,
      "",
      `**Logged**: ${nowIso}`,
      `**Priority**: ${priority}`,
      `**Status**: ${status}`,
      `**Area**: ${area}`,
      "",
      "### Summary",
      summary.trim(),
      "",
      "### Details",
      details.trim() || "-",
      "",
      "### Suggested Action",
      suggestedAction.trim() || "-",
      "",
      "### Metadata",
      `- Source: ${source}`,
      "---",
      "",
    ].join("\n");
    const prev = await readFile(filePath, "utf-8").catch(() => "");
    const separator = prev.trimEnd().length > 0 ? "\n\n" : "";
    await appendFile(filePath, `${separator}${entry}`, "utf-8");
    return entryId;
  });

  return { id, filePath };
}

export interface GovernanceToolContext {
  workspaceDir?: string;
}

export interface GovernanceRegistrationOptions {
  enableManagementTools?: boolean;
  enabled?: boolean;
  defaultWorkspaceDir?: string;
}

function resolveWorkspaceDir(toolCtx: unknown, fallback?: string): string {
  const runtime = toolCtx as Record<string, unknown> | undefined;
  const runtimePath = typeof runtime?.workspaceDir === "string" ? runtime.workspaceDir.trim() : "";
  if (runtimePath) return runtimePath;
  if (fallback && fallback.trim()) return fallback;
  return join(homedir(), ".openclaw", "workspace");
}

function escapeRegExp(input: string): string {
  return input.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function registerGovernanceLogToolByName(
  api: OpenClawPluginApi,
  context: GovernanceToolContext,
  registrationName: string,
  label: string
) {
  api.registerTool(
    (toolCtx) => ({
      name: registrationName,
      label,
      description: "Log structured governance backlog entries into .governance for review and later promotion.",
      parameters: Type.Object({
        type: stringEnum(["learning", "error"]),
        summary: Type.String({ description: "One-line summary" }),
        details: Type.Optional(Type.String({ description: "Detailed context or error output" })),
        suggestedAction: Type.Optional(Type.String({ description: "Concrete action to prevent recurrence" })),
        category: Type.Optional(Type.String({ description: "learning category (correction/best_practice/knowledge_gap) when type=learning" })),
        area: Type.Optional(Type.String({ description: "frontend|backend|infra|tests|docs|config or custom area" })),
        priority: Type.Optional(Type.String({ description: "low|medium|high|critical" })),
      }),
      async execute(_toolCallId, params) {
        const {
          type,
          summary,
          details = "",
          suggestedAction = "",
          category = "best_practice",
          area = "config",
          priority = "medium",
        } = params as {
          type: "learning" | "error";
          summary: string;
          details?: string;
          suggestedAction?: string;
          category?: string;
          area?: string;
          priority?: string;
        };
        try {
          const workspaceDir = resolveWorkspaceDir(toolCtx, context.workspaceDir);
          const { id: entryId, filePath } = await appendGovernanceEntry({
            baseDir: workspaceDir,
            type,
            summary,
            details,
            suggestedAction,
            category,
            area,
            priority,
            source: `openclaw-chronicle-engine/${registrationName}`,
          });
          const fileName = type === "learning" ? "LEARNINGS.md" : "ERRORS.md";

          return {
            content: [{ type: "text", text: `Logged ${type} entry ${entryId} to .governance/${fileName}` }],
            details: { action: "logged", type, id: entryId, filePath },
          };
        } catch (error) {
          return {
            content: [{ type: "text", text: `Failed to log governance entry: ${error instanceof Error ? error.message : String(error)}` }],
            details: { error: "governance_log_failed", message: String(error) },
          };
        }
      },
    }),
    { name: registrationName }
  );
}

export function registerGovernanceLogTool(api: OpenClawPluginApi, context: GovernanceToolContext) {
  registerGovernanceLogToolByName(api, context, "governance_log", "Governance Log");
}

function registerGovernanceExtractSkillToolByName(
  api: OpenClawPluginApi,
  context: GovernanceToolContext,
  registrationName: string,
  label: string
) {
  api.registerTool(
    (toolCtx) => ({
      name: registrationName,
      label,
      description: "Create a new skill scaffold from a governance backlog entry and mark the source entry as promoted_to_skill.",
      parameters: Type.Object({
        learningId: Type.String({ description: "Learning ID like LRN-YYYYMMDD-001" }),
        skillName: Type.String({ description: "Skill folder name, lowercase with hyphens" }),
        sourceFile: Type.Optional(stringEnum(["LEARNINGS.md", "ERRORS.md"])),
        outputDir: Type.Optional(Type.String({ description: "Relative output dir under workspace (default: skills)" })),
      }),
      async execute(_toolCallId, params) {
        const { learningId, skillName, sourceFile = "LEARNINGS.md", outputDir = "skills" } = params as {
          learningId: string;
          skillName: string;
          sourceFile?: "LEARNINGS.md" | "ERRORS.md";
          outputDir?: string;
        };
        try {
          if (!/^(LRN|ERR)-\d{8}-\d{3}$/.test(learningId)) {
            return {
              content: [{ type: "text", text: "Invalid learningId format. Use LRN-YYYYMMDD-001 / ERR-..." }],
              details: { error: "invalid_learning_id" },
            };
          }
          if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(skillName)) {
            return {
              content: [{ type: "text", text: "Invalid skillName. Use lowercase letters, numbers, and hyphens only." }],
              details: { error: "invalid_skill_name" },
            };
          }

          const workspaceDir = resolveWorkspaceDir(toolCtx, context.workspaceDir);
          await ensureGovernanceBacklogFiles(workspaceDir);
          const learningsPath = join(canonicalGovernanceDir(workspaceDir), sourceFile);
          const learningBody = await readFile(learningsPath, "utf-8");
          const escapedLearningId = escapeRegExp(learningId.trim());
          const entryRegex = new RegExp(`## \\[${escapedLearningId}\\][\\s\\S]*?(?=\\n## \\[|$)`, "m");
          const match = learningBody.match(entryRegex);
          if (!match) {
            return {
              content: [{ type: "text", text: `Governance entry ${learningId} not found in .governance/${sourceFile}` }],
              details: { error: "learning_not_found", learningId, sourceFile },
            };
          }

          const summaryMatch = match[0].match(/### Summary\n([\s\S]*?)\n###/m);
          const summary = (summaryMatch?.[1] ?? "Summarize the source governance entry here.").trim();
          const safeOutputDir = outputDir
            .replace(/\\/g, "/")
            .split("/")
            .filter((segment) => segment && segment !== "." && segment !== "..")
            .join("/");
          const skillDir = join(workspaceDir, safeOutputDir || "skills", skillName);
          await mkdir(skillDir, { recursive: true });
          const skillPath = join(skillDir, "SKILL.md");
          const skillTitle = skillName
            .split("-")
            .map((s) => s.charAt(0).toUpperCase() + s.slice(1))
            .join(" ");
          const skillContent = [
            "---",
            `name: ${skillName}`,
            `description: "Extracted from governance entry ${learningId}. Replace with a concise description."`,
            "---",
            "",
            `# ${skillTitle}`,
            "",
            "## Why",
            summary,
            "",
            "## When To Use",
            "- Add concrete trigger conditions before relying on this skill in production workflows.",
            "",
            "## Steps",
            "1. Replace this line with the repeatable workflow steps.",
            "2. Replace this line with concrete verification steps.",
            "",
            "## Source Governance Entry",
            `- Learning ID: ${learningId}`,
            `- Source File: .governance/${sourceFile}`,
            "",
          ].join("\n");
          await writeFile(skillPath, skillContent, "utf-8");

          const promotedMarker = `**Status**: promoted_to_skill`;
          const skillPathMarker = `- Skill-Path: ${safeOutputDir || "skills"}/${skillName}`;
          let updatedEntry = match[0];
          updatedEntry = updatedEntry.includes("**Status**:")
            ? updatedEntry.replace(/\*\*Status\*\*:\s*.+/m, promotedMarker)
            : `${updatedEntry.trimEnd()}\n${promotedMarker}\n`;
          if (!updatedEntry.includes("Skill-Path:")) {
            updatedEntry = `${updatedEntry.trimEnd()}\n${skillPathMarker}\n`;
          }
          const updatedLearningBody = learningBody.replace(match[0], updatedEntry);
          await writeFile(learningsPath, updatedLearningBody, "utf-8");

          return {
            content: [{ type: "text", text: `Extracted skill scaffold to ${safeOutputDir || "skills"}/${skillName}/SKILL.md and updated ${learningId}.` }],
            details: {
              action: "skill_extracted",
              learningId,
              sourceFile,
              skillPath: `${safeOutputDir || "skills"}/${skillName}/SKILL.md`,
            },
          };
        } catch (error) {
          return {
            content: [{ type: "text", text: `Failed to extract governance skill: ${error instanceof Error ? error.message : String(error)}` }],
            details: { error: "governance_extract_skill_failed", message: String(error) },
          };
        }
      },
    }),
    { name: registrationName }
  );
}

export function registerGovernanceExtractSkillTool(api: OpenClawPluginApi, context: GovernanceToolContext) {
  registerGovernanceExtractSkillToolByName(api, context, "governance_extract_skill", "Governance Extract Skill");
}

function registerGovernanceReviewToolByName(
  api: OpenClawPluginApi,
  context: GovernanceToolContext,
  registrationName: string,
  label: string
) {
  api.registerTool(
    (toolCtx) => ({
      name: registrationName,
      label,
      description: "Summarize the governance backlog from .governance files (pending/high-priority/promoted counts).",
      parameters: Type.Object({}),
      async execute() {
        try {
          const workspaceDir = resolveWorkspaceDir(toolCtx, context.workspaceDir);
          await ensureGovernanceBacklogFiles(workspaceDir);
          const governanceDir = canonicalGovernanceDir(workspaceDir);
          const files = ["LEARNINGS.md", "ERRORS.md"] as const;
          const stats = { pending: 0, high: 0, promoted: 0, total: 0 };

          for (const f of files) {
            const content = await readFile(join(governanceDir, f), "utf-8").catch(() => "");
            stats.total += (content.match(/^## \[/gm) || []).length;
            stats.pending += (content.match(/\*\*Status\*\*:\s*pending/gi) || []).length;
            stats.high += (content.match(/\*\*Priority\*\*:\s*(high|critical)/gi) || []).length;
            stats.promoted += (content.match(/\*\*Status\*\*:\s*promoted(_to_skill)?/gi) || []).length;
          }

          const text = [
            "Governance backlog snapshot:",
            `- Total entries: ${stats.total}`,
            `- Pending: ${stats.pending}`,
            `- High/Critical: ${stats.high}`,
            `- Promoted: ${stats.promoted}`,
            "",
            "Recommended loop:",
            "1) Resolve high-priority pending entries",
            "2) Promote durable rules into AGENTS.md / SOUL.md / TOOLS.md when they stabilize",
            "3) Extract repeatable patterns as skills",
          ].join("\n");

          return {
            content: [{ type: "text", text }],
            details: { action: "review", stats },
          };
        } catch (error) {
          return {
            content: [{ type: "text", text: `Failed to review governance backlog: ${error instanceof Error ? error.message : String(error)}` }],
            details: { error: "governance_review_failed", message: String(error) },
          };
        }
      },
    }),
    { name: registrationName }
  );
}

export function registerGovernanceReviewTool(api: OpenClawPluginApi, context: GovernanceToolContext) {
  registerGovernanceReviewToolByName(api, context, "governance_review", "Governance Review");
}

export function registerGovernanceTools(
  api: OpenClawPluginApi,
  options: GovernanceRegistrationOptions = {}
) {
  if (options.enabled === false) return;

  const passthroughCtx: GovernanceToolContext = { workspaceDir: options.defaultWorkspaceDir };
  registerGovernanceLogTool(api, passthroughCtx);

  if (options.enableManagementTools) {
    registerGovernanceExtractSkillTool(api, passthroughCtx);
    registerGovernanceReviewTool(api, passthroughCtx);
  }
}
