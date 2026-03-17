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

describe("sessionStrategy cutover contract", () => {
  it("defaults to systemSessionMemory when neither field is set", () => {
    const parsed = parsePluginConfig(baseConfig());
    assert.equal(parsed.sessionStrategy, "systemSessionMemory");
  });

  it("rejects removed sessionMemory fields", () => {
    assert.throws(
      () =>
        parsePluginConfig({
          ...baseConfig(),
          sessionMemory: { enabled: true },
        }),
      /sessionMemory is no longer supported in 1\.0\.0-beta\.0/
    );
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

  it("rejects removed local reflection-generation fields", () => {
    assert.throws(
      () =>
        parsePluginConfig({
          ...baseConfig(),
          sessionStrategy: "memoryReflection",
          memoryReflection: {
            agentId: "memory-distiller",
          },
        }),
      /memoryReflection\.agentId is no longer supported in 1\.0\.0-beta\.0/
    );
  });
});
