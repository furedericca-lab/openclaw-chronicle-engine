import { createHash } from "node:crypto";
import type { ReflectionErrorSignal } from "./session-exposure-state.js";

export interface ReflectionToolCallEvent {
  toolName?: string;
  error?: unknown;
  result?: unknown;
}

export function extractReflectionErrorSignalFromToolCall(
  event: ReflectionToolCallEvent,
  maxScanChars: number
): ReflectionErrorSignal | null {
  const toolName = typeof event.toolName === "string" && event.toolName.trim() ? event.toolName : "unknown";

  if (typeof event.error === "string" && event.error.trim().length > 0) {
    const signature = normalizeErrorSignature(event.error);
    return {
      at: Date.now(),
      toolName,
      summary: summarizeErrorText(event.error),
      source: "tool_error",
      signature,
      signatureHash: sha256Hex(signature).slice(0, 16),
    };
  }

  const raw = extractTextFromToolResult(event.result);
  const clipped = raw.length > maxScanChars ? raw.slice(0, maxScanChars) : raw;
  if (!clipped || !containsErrorSignal(clipped)) return null;
  const signature = normalizeErrorSignature(clipped);
  return {
    at: Date.now(),
    toolName,
    summary: summarizeErrorText(clipped),
    source: "tool_output",
    signature,
    signatureHash: sha256Hex(signature).slice(0, 16),
  };
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

function containsErrorSignal(text: string): boolean {
  const normalized = text.toLowerCase();
  return (
    /\[error\]|error:|exception:|fatal:|traceback|syntaxerror|typeerror|referenceerror|npm err!/.test(normalized) ||
    /command not found|no such file|permission denied|non-zero|exit code/.test(normalized) ||
    /"status"\s*:\s*"error"|"status"\s*:\s*"failed"|\biserror\b/.test(normalized) ||
    /错误\s*[：:]|异常\s*[：:]|报错\s*[：:]|失败\s*[：:]/.test(normalized)
  );
}

function summarizeErrorText(text: string, maxLen = 220): string {
  const oneLine = redactSecrets(text).replace(/\s+/g, " ").trim();
  if (!oneLine) return "(empty tool error)";
  return oneLine.length <= maxLen ? oneLine : `${oneLine.slice(0, maxLen - 3)}...`;
}

function normalizeErrorSignature(text: string): string {
  return redactSecrets(String(text || ""))
    .toLowerCase()
    .replace(/[a-z]:\\[^ \n\r\t]+/gi, "<path>")
    .replace(/\/[^ \n\r\t]+/g, "<path>")
    .replace(/\b0x[0-9a-f]+\b/gi, "<hex>")
    .replace(/\b\d+\b/g, "<n>")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 240);
}

function extractTextFromToolResult(result: unknown): string {
  if (result == null) return "";
  if (typeof result === "string") return result;
  if (typeof result === "object") {
    const obj = result as Record<string, unknown>;
    const content = obj.content;
    if (Array.isArray(content)) {
      const textParts = content
        .filter((block) => block && typeof block === "object")
        .map((block) => (block as Record<string, unknown>).text)
        .filter((text): text is string => typeof text === "string");
      if (textParts.length > 0) return textParts.join("\n");
    }
    if (typeof obj.text === "string") return obj.text;
    if (typeof obj.error === "string") return obj.error;
    if (typeof obj.details === "string") return obj.details;
  }
  try {
    return JSON.stringify(result);
  } catch {
    return "";
  }
}

function sha256Hex(text: string): string {
  return createHash("sha256").update(text, "utf8").digest("hex");
}
