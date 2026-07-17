# Claude Read contract capsule

## approved-29

Source anchor: local `claude-code` commit `6f6f12b`,
`src/tools/FileReadTool/FileReadTool.ts` and `src/utils/readFileInRange.ts`.

### Approved observable contract

`Read` is published with the strict source schema: required `file_path`, optional
non-negative integer `offset`, positive integer `limit`, and optional string
`pages`. Validation (including unknown keys) happens before permission; pages
validation happens after schema validation and before permission. Exact `Read`
dispatches internally to lowercase `read` only.

For text, `offset: 0` is valid; UTF-8 BOM is removed, CRLF is rendered as LF,
and output uses Claude's compact `LINE<TAB>content` form. Empty files and
offset-past-end use the source system-reminder text. Directories fail with the
source `EISDIR` error rather than producing a listing. Text reads retain the
source 256 KiB whole-file limit when no line limit is provided and add the
source cyber-risk reminder to non-empty text output.

### Explicit unsupported / non-scope

PDF/page rendering and notebook rendering return explicit unsupported failures.
Read deduplication/read-state, in-flight cancellation, image behavior, dynamic
source feature flags and token-counter parity are not implemented. They are not
reported as successful Claude-equivalent behavior.

### Implementation record (separate from approved contract)

The OpenCode `read` backend is retained as the lowercase internal dispatch
target. Its text path implements the approved Claude-facing text presentation,
directory error, BOM/CRLF normalization and size bound; this shared backend is
necessary because ACP dispatch only carries the registered target and payload.
