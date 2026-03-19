export function parseBehavioralGuidanceMetadata(metadataRaw: string | undefined): Record<string, unknown> {
  if (!metadataRaw) return {};
  try {
    const parsed = JSON.parse(metadataRaw);
    return parsed && typeof parsed === "object" ? parsed as Record<string, unknown> : {};
  } catch {
    return {};
  }
}

export function isBehavioralGuidanceEntry(entry: { category: string; metadata?: string }): boolean {
  if (entry.category === "reflection") return true;
  const metadata = parseBehavioralGuidanceMetadata(entry.metadata);
  return metadata.type === "memory-reflection-event" ||
    metadata.type === "memory-reflection-item";
}

export function getDisplayCategoryTag(entry: { category: string; scope: string; metadata?: string }): string {
  if (!isBehavioralGuidanceEntry(entry)) return `${entry.category}:${entry.scope}`;
  return `reflection:${entry.scope}`;
}

type RetryClassifierInput = {
  inBehavioralGuidanceScope: boolean;
  retryCount: number;
  usefulOutputChars: number;
  error: unknown;
};

type RetryClassifierResult = {
  retryable: boolean;
  reason:
  | "not_behavioral_guidance_scope"
  | "retry_already_used"
  | "useful_output_present"
  | "non_retry_error"
  | "non_transient_error"
  | "transient_upstream_failure";
  normalizedError: string;
};

type RetryState = { count: number };

type RetryRunnerParams<T> = {
  scope: "behavioral-guidance" | "distiller";
  runner: "direct" | "cli";
  retryState: RetryState;
  execute: () => Promise<T>;
  onLog?: (level: "info" | "warn", message: string) => void;
  random?: () => number;
  sleep?: (ms: number) => Promise<void>;
};

const BEHAVIORAL_GUIDANCE_TRANSIENT_PATTERNS: RegExp[] = [
  /unexpected eof/i,
  /\beconnreset\b/i,
  /\beconnaborted\b/i,
  /\betimedout\b/i,
  /\bepipe\b/i,
  /connection reset/i,
  /socket hang up/i,
  /socket (?:closed|disconnected)/i,
  /connection (?:closed|aborted|dropped)/i,
  /early close/i,
  /stream (?:ended|closed) unexpectedly/i,
  /temporar(?:y|ily).*unavailable/i,
  /upstream.*unavailable/i,
  /service unavailable/i,
  /bad gateway/i,
  /gateway timeout/i,
  /\b(?:http|status)\s*(?:502|503|504)\b/i,
  /\btimed out\b/i,
  /\btimeout\b/i,
  /\bund_err_(?:socket|headers_timeout|body_timeout)\b/i,
  /network error/i,
  /fetch failed/i,
];

const BEHAVIORAL_GUIDANCE_NON_RETRY_PATTERNS: RegExp[] = [
  /\b401\b/i,
  /\bunauthorized\b/i,
  /invalid api key/i,
  /invalid[_ -]?token/i,
  /\bauth(?:entication)?_?unavailable\b/i,
  /insufficient (?:credit|credits|balance)/i,
  /\bbilling\b/i,
  /\bquota exceeded\b/i,
  /payment required/i,
  /model .*not found/i,
  /no such model/i,
  /unknown model/i,
  /context length/i,
  /context window/i,
  /request too large/i,
  /payload too large/i,
  /too many tokens/i,
  /token limit/i,
  /prompt too long/i,
  /session expired/i,
  /invalid session/i,
  /refusal/i,
  /content policy/i,
  /safety policy/i,
  /content filter/i,
  /disallowed/i,
];

const DEFAULT_SLEEP = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

function toErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    const msg = `${error.name}: ${error.message}`.trim();
    return msg || "Error";
  }
  if (typeof error === "string") return error;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

function clipSingleLine(text: string, maxLen = 260): string {
  const oneLine = text.replace(/\s+/g, " ").trim();
  if (oneLine.length <= maxLen) return oneLine;
  return `${oneLine.slice(0, maxLen - 3)}...`;
}

export function isTransientBehavioralGuidanceUpstreamError(error: unknown): boolean {
  const msg = toErrorMessage(error);
  return BEHAVIORAL_GUIDANCE_TRANSIENT_PATTERNS.some((pattern) => pattern.test(msg));
}

export function isBehavioralGuidanceNonRetryError(error: unknown): boolean {
  const msg = toErrorMessage(error);
  return BEHAVIORAL_GUIDANCE_NON_RETRY_PATTERNS.some((pattern) => pattern.test(msg));
}

export function classifyBehavioralGuidanceRetry(input: RetryClassifierInput): RetryClassifierResult {
  const normalizedError = clipSingleLine(toErrorMessage(input.error), 260);

  if (!input.inBehavioralGuidanceScope) {
    return { retryable: false, reason: "not_behavioral_guidance_scope", normalizedError };
  }
  if (input.retryCount > 0) {
    return { retryable: false, reason: "retry_already_used", normalizedError };
  }
  if (input.usefulOutputChars > 0) {
    return { retryable: false, reason: "useful_output_present", normalizedError };
  }
  if (isBehavioralGuidanceNonRetryError(input.error)) {
    return { retryable: false, reason: "non_retry_error", normalizedError };
  }
  if (isTransientBehavioralGuidanceUpstreamError(input.error)) {
    return { retryable: true, reason: "transient_upstream_failure", normalizedError };
  }
  return { retryable: false, reason: "non_transient_error", normalizedError };
}

export function computeBehavioralGuidanceRetryDelayMs(random: () => number = Math.random): number {
  const raw = random();
  const clamped = Number.isFinite(raw) ? Math.min(1, Math.max(0, raw)) : 0;
  return 1000 + Math.floor(clamped * 2000);
}

export async function runWithBehavioralGuidanceTransientRetryOnce<T>(
  params: RetryRunnerParams<T>
): Promise<T> {
  try {
    return await params.execute();
  } catch (error) {
    const decision = classifyBehavioralGuidanceRetry({
      inBehavioralGuidanceScope: params.scope === "behavioral-guidance" || params.scope === "distiller",
      retryCount: params.retryState.count,
      usefulOutputChars: 0,
      error,
    });
    if (!decision.retryable) throw error;

    const delayMs = computeBehavioralGuidanceRetryDelayMs(params.random);
    params.retryState.count += 1;
    params.onLog?.(
      "warn",
      `memory-${params.scope}: transient upstream failure detected (${params.runner}); ` +
      `retrying once in ${delayMs}ms (${decision.reason}). error=${decision.normalizedError}`
    );
    await (params.sleep ?? DEFAULT_SLEEP)(delayMs);

    try {
      const result = await params.execute();
      params.onLog?.("info", `memory-${params.scope}: retry succeeded (${params.runner})`);
      return result;
    } catch (retryError) {
      params.onLog?.(
        "warn",
        `memory-${params.scope}: retry exhausted (${params.runner}). ` +
        `error=${clipSingleLine(toErrorMessage(retryError), 260)}`
      );
      throw retryError;
    }
  }
}

export interface BehavioralGuidanceSlices {
  invariants: string[];
  derived: string[];
}

export interface BehavioralGuidanceMappedMemory {
  text: string;
  category: "preference" | "fact" | "decision";
  heading: string;
}

export type BehavioralGuidanceMappedKind = "user-model" | "agent-model" | "lesson" | "decision";

export interface BehavioralGuidanceMappedMemoryItem extends BehavioralGuidanceMappedMemory {
  mappedKind: BehavioralGuidanceMappedKind;
  ordinal: number;
  groupSize: number;
}

export interface BehavioralGuidanceSliceItem {
  text: string;
  itemKind: "invariant" | "derived";
  section: "Invariants" | "Derived";
  ordinal: number;
  groupSize: number;
}

export interface BehavioralGuidanceGovernanceEntry {
  priority?: string;
  status?: string;
  area?: string;
  summary: string;
  details?: string;
  suggestedAction?: string;
}

export function extractSectionMarkdown(markdown: string, heading: string): string {
  const lines = markdown.split(/\r?\n/);
  const headingNeedle = `## ${heading}`.toLowerCase();
  let inSection = false;
  const collected: string[] = [];
  for (const raw of lines) {
    const line = raw.trim();
    const lower = line.toLowerCase();
    if (lower.startsWith("## ")) {
      if (inSection && lower !== headingNeedle) break;
      inSection = lower === headingNeedle;
      continue;
    }
    if (!inSection) continue;
    collected.push(raw);
  }
  return collected.join("\n").trim();
}

export function parseSectionBullets(markdown: string, heading: string): string[] {
  const lines = extractSectionMarkdown(markdown, heading).split(/\r?\n/);
  const collected: string[] = [];
  for (const raw of lines) {
    const line = raw.trim();
    if (line.startsWith("- ") || line.startsWith("* ")) {
      const normalized = line.slice(2).trim();
      if (normalized) collected.push(normalized);
    }
  }
  return collected;
}

function parseSectionBulletsAny(markdown: string, headings: string[]): string[] {
  for (const heading of headings) {
    const parsed = parseSectionBullets(markdown, heading);
    if (parsed.length > 0) return parsed;
  }
  return [];
}

export function isPlaceholderBehavioralGuidanceSliceLine(line: string): boolean {
  const normalized = line.replace(/\*\*/g, "").trim();
  if (!normalized) return true;
  if (/^\(none( captured)?\)$/i.test(normalized)) return true;
  if (/^(invariants?|reflections?|derived)[:：]$/i.test(normalized)) return true;
  if (/apply this session'?s deltas next run/i.test(normalized)) return true;
  if (/apply this session'?s distilled changes next run/i.test(normalized)) return true;
  if (/investigate why direct trajectory-derived generation failed/i.test(normalized)) return true;
  return false;
}

export function normalizeBehavioralGuidanceSliceLine(line: string): string {
  return line
    .replace(/\*\*/g, "")
    .replace(/^(invariants?|reflections?|derived)[:：]\s*/i, "")
    .trim();
}

export function sanitizeBehavioralGuidanceSliceLines(lines: string[]): string[] {
  return lines
    .map(normalizeBehavioralGuidanceSliceLine)
    .filter((line) => !isPlaceholderBehavioralGuidanceSliceLine(line));
}

function isInvariantRuleLike(line: string): boolean {
  return /^(always|never|when\b|if\b|before\b|after\b|prefer\b|avoid\b|require\b|only\b|do not\b|must\b|should\b)/i.test(line) ||
    /\b(must|should|never|always|prefer|avoid|required?)\b/i.test(line);
}

function isDerivedDeltaLike(line: string): boolean {
  return /^(this run|next run|going forward|follow-up|re-check|retest|verify|confirm|avoid repeating|adjust|change|update|retry|keep|watch)\b/i.test(line) ||
    /\b(this run|next run|delta|change|adjust|retry|re-check|retest|verify|confirm|avoid repeating|follow-up)\b/i.test(line);
}

function isOpenLoopAction(line: string): boolean {
  return /^(investigate|verify|confirm|re-check|retest|update|add|remove|fix|avoid|keep|watch|document)\b/i.test(line);
}

export function extractBehavioralGuidanceOpenLoops(behavioralGuidanceText: string): string[] {
  return sanitizeBehavioralGuidanceSliceLines(parseSectionBullets(behavioralGuidanceText, "Open loops / next actions"))
    .filter(isOpenLoopAction)
    .slice(0, 8);
}

export function extractBehavioralGuidanceLessons(behavioralGuidanceText: string): string[] {
  return sanitizeBehavioralGuidanceSliceLines(parseSectionBullets(behavioralGuidanceText, "Lessons & pitfalls (symptom / cause / fix / prevention)"));
}

export function extractBehavioralGuidanceGovernanceCandidates(behavioralGuidanceText: string): BehavioralGuidanceGovernanceEntry[] {
  const section = extractSectionMarkdown(behavioralGuidanceText, "Learning governance candidates (.governance / promotion / skill extraction)");
  if (!section) return [];

  const entryBlocks = section
    .split(/(?=^###\s+Entry\b)/gim)
    .map((block) => block.trim())
    .filter(Boolean);

  const parsed = entryBlocks
    .map(parseBehavioralGuidanceGovernanceEntry)
    .filter((entry): entry is BehavioralGuidanceGovernanceEntry => entry !== null);

  if (parsed.length > 0) return parsed;

  const fallbackBullets = sanitizeBehavioralGuidanceSliceLines(
    parseSectionBulletsAny(behavioralGuidanceText, [
      "Learning governance candidates (.governance / promotion / skill extraction)",
    ])
  );
  if (fallbackBullets.length === 0) return [];

  return [{
    priority: "medium",
    status: "pending",
    area: "config",
    summary: "Behavioral guidance governance candidates",
    details: fallbackBullets.map((line) => `- ${line}`).join("\n"),
    suggestedAction: "Review the governance candidates, promote durable rules to AGENTS.md / SOUL.md / TOOLS.md when stable, and extract a skill if the pattern becomes reusable.",
  }];
}

function parseBehavioralGuidanceGovernanceEntry(block: string): BehavioralGuidanceGovernanceEntry | null {
  const body = block.replace(/^###\s+Entry\b[^\n]*\n?/i, "").trim();
  if (!body) return null;

  const readField = (label: string): string | undefined => {
    const match = body.match(new RegExp(`^\\*\\*${label}\\*\\*:\\s*(.+)$`, "im"));
    const value = match?.[1]?.trim();
    return value ? value : undefined;
  };

  const readSection = (label: string): string | undefined => {
    const escaped = label.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const match = body.match(new RegExp(`^###\\s+${escaped}\\s*\\n([\\s\\S]*?)(?=^###\\s+|$)`, "im"));
    const value = match?.[1]?.trim();
    return value ? value : undefined;
  };

  const summary = readSection("Summary");
  if (!summary) return null;

  return {
    priority: readField("Priority"),
    status: readField("Status"),
    area: readField("Area"),
    summary,
    details: readSection("Details"),
    suggestedAction: readSection("Suggested Action"),
  };
}

export function extractBehavioralGuidanceMappedMemories(behavioralGuidanceText: string): BehavioralGuidanceMappedMemory[] {
  return extractBehavioralGuidanceMappedMemoryItems(behavioralGuidanceText).map(({ text, category, heading }) => ({ text, category, heading }));
}

export function extractBehavioralGuidanceMappedMemoryItems(behavioralGuidanceText: string): BehavioralGuidanceMappedMemoryItem[] {
  const mappedSections: Array<{
    heading: string;
    category: "preference" | "fact" | "decision";
    mappedKind: BehavioralGuidanceMappedKind;
  }> = [
    {
      heading: "User model deltas (about the human)",
      category: "preference",
      mappedKind: "user-model",
    },
    {
      heading: "Agent model deltas (about the assistant/system)",
      category: "preference",
      mappedKind: "agent-model",
    },
    {
      heading: "Lessons & pitfalls (symptom / cause / fix / prevention)",
      category: "fact",
      mappedKind: "lesson",
    },
    {
      heading: "Decisions (durable)",
      category: "decision",
      mappedKind: "decision",
    },
  ];

  return mappedSections.flatMap(({ heading, category, mappedKind }) => {
    const lines = sanitizeBehavioralGuidanceSliceLines(parseSectionBullets(behavioralGuidanceText, heading));
    const groupSize = lines.length;
    return lines.map((text, ordinal) => ({ text, category, heading, mappedKind, ordinal, groupSize }));
  });
}

export function extractBehavioralGuidanceSlices(behavioralGuidanceText: string): BehavioralGuidanceSlices {
  const invariantSection = parseSectionBulletsAny(behavioralGuidanceText, ["Durable guidance", "Invariants"]);
  const derivedSection = parseSectionBulletsAny(behavioralGuidanceText, ["Adaptive guidance", "Derived"]);
  const mergedSection = parseSectionBullets(behavioralGuidanceText, "Invariants & Reflections");

  const invariantsPrimary = sanitizeBehavioralGuidanceSliceLines(invariantSection).filter(isInvariantRuleLike);
  const derivedPrimary = sanitizeBehavioralGuidanceSliceLines(derivedSection).filter(isDerivedDeltaLike);

  const invariantLinesLegacy = sanitizeBehavioralGuidanceSliceLines(
    mergedSection.filter((line) => /invariant|stable|policy|rule/i.test(line))
  ).filter(isInvariantRuleLike);
  const reflectionLinesLegacy = sanitizeBehavioralGuidanceSliceLines(
    mergedSection.filter((line) => /reflect|inherit|derive|change|apply/i.test(line))
  ).filter(isDerivedDeltaLike);
  const durableDecisionLines = sanitizeBehavioralGuidanceSliceLines(parseSectionBullets(behavioralGuidanceText, "Decisions (durable)"))
    .filter(isInvariantRuleLike);

  const invariants = invariantsPrimary.length > 0
    ? invariantsPrimary
    : (invariantLinesLegacy.length > 0 ? invariantLinesLegacy : durableDecisionLines);
  const derived = derivedPrimary.length > 0
    ? derivedPrimary
    : reflectionLinesLegacy;

  return {
    invariants: invariants.slice(0, 8),
    derived: derived.slice(0, 10),
  };
}

export function extractBehavioralGuidanceSliceItems(behavioralGuidanceText: string): BehavioralGuidanceSliceItem[] {
  const slices = extractBehavioralGuidanceSlices(behavioralGuidanceText);
  const invariantGroupSize = slices.invariants.length;
  const derivedGroupSize = slices.derived.length;

  const invariantItems = slices.invariants.map((text, ordinal) => ({
    text,
    itemKind: "invariant" as const,
    section: "Invariants" as const,
    ordinal,
    groupSize: invariantGroupSize,
  }));
  const derivedItems = slices.derived.map((text, ordinal) => ({
    text,
    itemKind: "derived" as const,
    section: "Derived" as const,
    ordinal,
    groupSize: derivedGroupSize,
  }));

  return [...invariantItems, ...derivedItems];
}
