# Claude Code `Grep` contract capsule

## approved-26

| Item | Value |
| --- | --- |
| Status | `approved` |
| Revision | `approved-26` |
| Approval | User explicitly approved all four draft-26 recommended decisions. |
| Source oracle | `claude-code @ 6f6f12b37f529488b10e53928dd5508bb93535c7` only |
| Scope | Exact public uppercase `Grep`; lowercase `grep` is preserved unchanged. |

This is the approved, branch-local fixation of draft-26. The draft itself
remains unmodified in the source workspace contract record; this document does
not reinterpret any earlier approved revision.

```text
strict schema
  -> supplied-path validation
  -> read permission
  -> Darwin preparation + availability probe
  -> configured rg (+ one qualifying EAGAIN retry)
  -> source result/error projection
```

Required observations: strict 14-field schema and `InputValidationError`
envelope; empty/omitted path uses cwd; missing supplied path fails before
permission; denied/cancelled permission reaches no rg lifecycle; content,
files-with-matches, and count projection; offset-before-limit with default 250
and zero unlimited; source sorting; configured rg/probe/telemetry; cancellation,
timeout and UTF-16 capture rules; exit code 1 is an empty success; other call
errors use the source error envelope.

Non-scope: lowercase `grep`; Glob, Read, Write, Bash; pager; shared test
infrastructure; source files; fake success or unapproved unsupported behavior.

## Implementation record

This section is intentionally separate from the approved contract. It is
updated only with implementation facts and cannot alter the approved meaning.

Implemented in this revision:

- Public uppercase `Grep` is registered separately from lowercase `grep` and
  validates the approved strict source envelope before normal dispatch.
- The supplied-path → `AccessKind::Grep` permission → source rg lifecycle →
  source result projection path is live. The pre-existing source lifecycle now
  owns its timeout/cancellation, one EAGAIN retry, UTF-16 capture cap,
  pagination, and all three output modes.
- Lowercase `grep` remains registered and routed through its existing tool.

Remaining source-only gaps: Darwin's source binary preparation/probe and its
telemetry implementation have no corresponding source-backed runtime service
in this workspace. They are not emulated as successful behavior.
