# Changelog

## 0.2.0 — 2026-07-18

### Breaking Changes

- `TaskToolInput` is now `#[non_exhaustive]` so future optional Tool fields do
  not force another exhaustive-construction break. Downstream Rust callers can
  no longer construct it with a struct literal.

### Migration

Replace exhaustive construction:

```rust,ignore
let input = TaskToolInput {
    prompt,
    description,
    // every remaining field
};
```

with the default-preserving constructor and explicit overrides:

```rust,ignore
let mut input = TaskToolInput::new(prompt, description);
input.run_in_background = false;
input.reasoning_effort = Some(SubagentReasoningEffort::High);
```

`TaskToolInput::new(prompt, description)` uses the same defaults as omitted
JSON fields: `subagent_type = "general-purpose"`, background execution enabled,
and all optional/server-injected fields unset. JSON/serde callers do not need to
migrate; the Tool schema and wire format are unchanged.
