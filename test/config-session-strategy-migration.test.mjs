import { describe, it } from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";
import jitiFactory from "jiti";

const testDir = path.dirname(fileURLToPath(import.meta.url));
const pluginSdkStubPath = path.resolve(testDir, "helpers", "openclaw-plugin-sdk-stub.mjs");
const jiti = jitiFactory(import.meta.url, {
  interopDefault: true,
  alias: {
    "openclaw/plugin-sdk": pluginSdkStubPath,
  },
});
const { parsePluginConfig } = jiti("../index.ts");

function baseConfig() {
  return {
    remoteBackend: {
      enabled: true,
      baseURL: "http://backend.test",
      authToken: "token-test",
    },
  };
}

describe("sessionStrategy legacy compatibility mapping", () => {
  it("maps legacy sessionMemory.enabled=true to systemSessionMemory", () => {
    const parsed = parsePluginConfig({
      ...baseConfig(),
      sessionMemory: { enabled: true },
    });
    assert.equal(parsed.sessionStrategy, "systemSessionMemory");
  });

  it("maps legacy sessionMemory.enabled=false to none", () => {
    const parsed = parsePluginConfig({
      ...baseConfig(),
      sessionMemory: { enabled: false },
    });
    assert.equal(parsed.sessionStrategy, "none");
  });

  it("prefers explicit sessionStrategy over legacy sessionMemory.enabled", () => {
    const parsed = parsePluginConfig({
      ...baseConfig(),
      sessionStrategy: "memoryReflection",
      sessionMemory: { enabled: false },
    });
    assert.equal(parsed.sessionStrategy, "memoryReflection");
  });

  it("defaults to systemSessionMemory when neither field is set", () => {
    const parsed = parsePluginConfig(baseConfig());
    assert.equal(parsed.sessionStrategy, "systemSessionMemory");
  });

  it("defaults generic auto-recall selection mode to mmr", () => {
    const parsed = parsePluginConfig(baseConfig());
    assert.equal(parsed.autoRecallSelectionMode, "mmr");
  });

  it("accepts explicit generic auto-recall selection mode mmr", () => {
    const parsed = parsePluginConfig({
      ...baseConfig(),
      autoRecallSelectionMode: "mmr",
    });
    assert.equal(parsed.autoRecallSelectionMode, "mmr");
  });

  it("parses explicit generic auto-recall selection mode setwise-v2", () => {
    const parsed = parsePluginConfig({
      ...baseConfig(),
      autoRecallSelectionMode: "setwise-v2",
    });
    assert.equal(parsed.autoRecallSelectionMode, "setwise-v2");
  });

  it("keeps deprecated local reflection-generation fields parseable but marks them ignored", () => {
    const parsed = parsePluginConfig({
      ...baseConfig(),
      sessionStrategy: "memoryReflection",
      memoryReflection: {
        agentId: "memory-distiller",
        maxInputChars: 16000,
        timeoutMs: 15000,
        thinkLevel: "high",
      },
    });

    assert.equal(parsed.memoryReflection.agentId, "memory-distiller");
    assert.equal(parsed.memoryReflection.maxInputChars, 16000);
    assert.equal(parsed.memoryReflection.timeoutMs, 15000);
    assert.equal(parsed.memoryReflection.thinkLevel, "high");
    assert.deepEqual(parsed.memoryReflection.deprecatedIgnoredFields, [
      "agentId",
      "maxInputChars",
      "timeoutMs",
      "thinkLevel",
    ]);
  });
});
