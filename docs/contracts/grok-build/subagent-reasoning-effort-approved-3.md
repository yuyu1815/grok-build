# Subagent reasoning-effort — approved-3 implementation record

Status: `approved-3` implementation record

This record preserves the approved public contract from
`docs/contracts/grok-build/subagent-reasoning-effort.md`:

```text
preflight (pure validation)
        ↓ no resource / DB / worktree / child side effect (failure notice may emit)
materialize (worktree, rehydrate, context, persistence)
        ↓
run/promote (child execution; active tracker owns lifecycle)
```

## Decisions

- `SubagentSpawnContext::available_models` is the immutable catalogue used for
  model and effort capability decisions.
- Catalog lookup is key-first and then `entry.info.model` slug fallback through
  `config::find_model_by_id`.
- Explicit Tool effort unsupported by the effective model fails before
  worktree creation with the existing `invalid_arguments` message. Harness,
  role, persona, and agent-definition effort remains best effort.
- TaskTool canonicalizes the public `max` alias to `xhigh`; the handler uses
  sampling `ReasoningEffort::FromStr` and has no public-to-sampling adapter.
- A freshly created worktree is owned by an explicit asynchronous cleanup seam
  until active promotion. The seam never performs async work in `Drop`.
- Resume rehydrate failures remove only a partial destination; a source-owned
  existing worktree remains intact. Foreground/background failure and finish
  notification semantics remain unchanged; preflight failure notifications use
  the effective `request.run_in_background || definition.background` mode.

## Known P2 constraint

If the entire spawn future is externally aborted or panics between worktree
materialization and the next explicit cleanup checkpoint, Rust cannot await the
worktree removal from `Drop`. The guard therefore does not hide asynchronous
cleanup in `Drop`; normal cancellation and every handled pre-promote error use
the explicit cleanup future. An externally aborted/panicking future may leave a
fresh worktree for the existing startup/orphan cleanup sweep to reconcile.

## Implementation evidence

- `cargo check -p xai-grok-shell --lib` — PASS.
- `cargo test -p xai-grok-tools task` — PASS (271 tests).
- Shell integration-test compilation remains baseline-blocked by the existing
  29 unrelated test-only diagnostics. No shell integration-test PASS is
  claimed; production compile and focused tool tests are the available
  evidence.

## Scope boundary

No public schema, model-selection, permission, persistence-format, notification
ordering, or unsupported-behavior changes are authorized by this record. Such a
change requires a new contract revision and user approval.
