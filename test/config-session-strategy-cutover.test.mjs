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

  it("rejects removed generic auto-recall selection mode setwise-v2", () => {
    assert.throws(
      () =>
        parsePluginConfig({
          ...baseConfig(),
          autoRecallSelectionMode: "setwise-v2",
        }),
      /autoRecallSelectionMode=setwise-v2 is no longer supported; use mmr/
    );
  });

  it("defaults generic auto-recall to exclude behavioral guidance rows", () => {
    const parsed = parsePluginConfig(baseConfig());
    assert.equal(parsed.autoRecallExcludeBehavioral, true);
  });

  it("parses canonical autoRecall behavioral config", () => {
    const parsed = parsePluginConfig({
      ...baseConfig(),
      sessionStrategy: "autoRecall",
      autoRecallBehavioral: {
        injectMode: "durable+adaptive",
        recall: {
          mode: "dynamic",
          includeKinds: ["durable", "adaptive"],
        },
      },
    });
    assert.equal(parsed.sessionStrategy, "autoRecall");
    assert.equal(parsed.autoRecallBehavioral.enabled, true);
    assert.equal(parsed.autoRecallBehavioral.injectMode, "durable+adaptive");
    assert.equal(parsed.autoRecallBehavioral.recall.mode, "dynamic");
    assert.deepEqual(parsed.autoRecallBehavioral.recall.includeKinds, ["durable", "adaptive"]);
  });

  it("rejects removed legacy sessionStrategy value memoryReflection", () => {
    assert.throws(
      () =>
        parsePluginConfig({
          ...baseConfig(),
          sessionStrategy: "memoryReflection",
        }),
      /sessionStrategy=memoryReflection is no longer supported in 1\.0\.0-beta\.0; use sessionStrategy=autoRecall/
    );
  });

  it("rejects removed legacy memoryReflection config namespace", () => {
    assert.throws(
      () =>
        parsePluginConfig({
          ...baseConfig(),
          memoryReflection: {
            recall: {
              mode: "dynamic",
            },
          },
        }),
      /memoryReflection is no longer supported in 1\.0\.0-beta\.0; use autoRecallBehavioral/
    );
  });

  it("rejects removed legacy selfImprovement config namespace", () => {
    assert.throws(
      () =>
        parsePluginConfig({
          ...baseConfig(),
          selfImprovement: {
            enabled: true,
          },
        }),
      /selfImprovement is no longer supported in 1\.0\.0-beta\.0; use governance or autoRecallBehavioral/
    );
  });
});
