export async function runEmbeddedPiAgent() {
  return {
    payloads: [
      {
        text: [
          "## Context (session background)",
          "- Current session behavioral-guidance test fixture.",
          "",
          "## Decisions (durable)",
          "- Keep behavioral guidance prompt planning localized to the prompt builder integration.",
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
          "## Learning governance candidates (.governance / promotion / skill extraction)",
          "- (none captured)",
          "",
          "## Open loops / next actions",
          "- Verify current behavioral guidance handoff after reset.",
          "",
          "## Retrieval tags / keywords",
          "- auto-recall-behavioral",
          "",
          "## Durable guidance",
          "- Keep behavioral-guidance in before_prompt_build only.",
          "",
          "## Adaptive guidance",
          "- Fresh adaptive line from this run.",
        ].join("\n"),
      },
    ],
  };
}
