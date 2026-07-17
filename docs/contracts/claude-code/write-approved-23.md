# Claude Code Write approved-23 implementation record

| Field | Value |
| --- | --- |
| Issue / vertical slice | #4 / Claude Code `Write` (`FileWriteTool`) |
| Status | `approved` |
| Approved revision | `approved-23` |
| Approval | User explicitly approved all four `draft-23` recommendations. |
| Source oracle | `claude-code @ 6f6f12b37f529488b10e53928dd5508bb93535c7` only |
| Scope baseline | `draft-23` in the shared contract capsule; earlier approved revisions are not edited or reinterpreted. |

This file fixes only the approved `draft-23` delta. It is intentionally a new
record because the historical contract capsule is not present in this assigned
worktree. It does not overwrite any earlier approved section.

```text
approved-23
  ├─ exact public Claude-facing `Write`, while lowercase `write` remains
  ├─ closed `{ file_path: string, content: string }` input before permission
  ├─ path/read-error/create-vs-update/result behavior where the Rust seam permits
  └─ generic Edit permission is explicitly unsupported, not called parity
```

## Fixed executable path and order

```text
source FileWriteTool
  schema → expandPath / validation → permission → mkdir
  → read (NotFound = create; all other read errors propagate) → write → result

Rust Claude-facing route
  `Write` closed-schema gate → resolver for permission target → generic Edit
  → lowercase `write` implementation → read (NotFound only is create)
  → mkdir → write → FileWritten / OpenCode result
```

The source entry point is `claude-code/src/tools.ts` (`FileWriteTool`), with
the implementation at `src/tools/FileWriteTool/FileWriteTool.ts`. Its strict
schema, `Write` name, `strict: true`, `maxResultSizeChars: 100_000`, search hint,
and create/update behavior are the only behavior oracle.

## Approved scope

- Case-sensitive inbound `Write` routes to the existing lowercase `write`
  implementation without changing lowercase parsing compatibility.
- Invalid source-shaped input is rejected before resolver, permission, prompt,
  mkdir, write, or success result.
- The permission target for exact `Write` uses the resolved model path; the
  dispatched input retains the original path.
- `read_file` treats only `NotFound` as a new file. Any other read error stops
  before parent creation, write, and notification.
- Grep routing, implementation, and tests are excluded from this PR.

## Explicit unsupported / non-scope

```text
generic AccessKind::Edit permission
  ≠ source FileWriteTool permission decision tree
  → explicit unsupported; no parity claim

TEAMMEM / Read-state / stale guard / LSP / VS Code / encoding-atomicity / gitDiff
  → draft-23 non-scope; no fabricated success or source-equivalence claim
```

The source-specific schema diagnostic and exact Claude message/ACP result shape
remain unsupported where the ACP transport has no established lossless mapping.
This record does not authorize Grep, lowercase behavior changes, test-support
changes, or pager changes.

## Implementation record (separate from approved contract)

- Removed pre-existing Grep routing and Grep tests from the Write PR.
- Added a behavioral regression test proving a non-`NotFound` pre-write read
  error reaches the caller and leaves both parent directory and target absent.
- Existing focused session tests cover strict input rejection, permission
  non-reachability, exact `Write → write` routing, and lowercase compatibility.
