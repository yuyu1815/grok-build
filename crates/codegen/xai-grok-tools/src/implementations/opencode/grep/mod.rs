//! `grep` tool — OpenCode namespace.
//!
//! Shells out to the ripgrep (`rg`) binary, parses the output, sorts
//! matches by file modification time (most recent first), caps at 100
//! results, truncates long lines, and formats as grouped output.

use std::collections::HashMap;
use std::process::Stdio;

use serde::de::Error as _;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use crate::implementations::grok_build::grep::ripgrep::rg_path;
use crate::types::output::{GrepFileMatch, GrepLineMatch, GrepSearchOutput};
use crate::types::requirements::{Expr, ToolRequirement};
#[allow(unused_imports)]
use crate::types::resources::{Cwd, SharedResources};
use crate::types::tool::{ToolKind, ToolNamespace};

// ───────────────────────────────────────────────────────────────────────────
// Constants
// ───────────────────────────────────────────────────────────────────────────

const RESULT_LIMIT: usize = 100;
const MAX_LINE_LENGTH: usize = 2000;
/// Claude Code's embedded-rg capture limit.  This is deliberately expressed
/// in UTF-16 code units, not bytes; JavaScript `String#length` uses this unit.
const EMBEDDED_CAPTURE_UTF16_CAP: usize = 20_000_000;

// ───────────────────────────────────────────────────────────────────────────
// Description
// ───────────────────────────────────────────────────────────────────────────

const DESCRIPTION: &str = r#"A powerful search tool built on ripgrep

Usage:
- Prefer ${{ tools.by_kind.search }} for exact symbol/string searches. Whenever possible, use this instead of terminal grep/rg. This tool is faster and respects .gitignore
- Supports full regex syntax, e.g. `log.*Error`, `function\s+\w+`. Ensure you escape special chars to get exact matches, e.g. `functionCall\(`
- Avoid overly broad glob patterns (e.g., '--glob *') as they bypass .gitignore rules and may be slow
- The pattern field is a raw regex string: do NOT wrap it in quotes or add trailing quote characters unnecessarily
- Only use 'include' when certain of the file type needed. Note: import paths may not match source file types (.js vs .ts)
- Results are capped for responsiveness; truncated results show "at least" counts.
- Filter files by pattern with the include parameter (e.g. "*.js", "*.{ts,tsx}")
- Returns file paths and line numbers with at least one match sorted by modification time
- Use this tool when you need to find files containing specific patterns"#;

// ───────────────────────────────────────────────────────────────────────────
// Input
// ───────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct GrepInput {
    /// The regex pattern to search for.
    pub pattern: String,

    /// Directory to search in. Defaults to the current working directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// File glob filter (e.g. "*.ts").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include: Option<String>,
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
struct SemanticNumber(f64);

impl SemanticNumber {
    fn ripgrep_arg(self) -> String {
        self.0.to_string()
    }

    fn javascript_integer(self) -> isize {
        self.0.trunc() as isize
    }
}

impl<'de> serde::Deserialize<'de> for SemanticNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let number = match value {
            serde_json::Value::Number(number) => number.as_f64(),
            serde_json::Value::String(text)
                if text
                    .strip_prefix('-')
                    .unwrap_or(&text)
                    .split_once('.')
                    .map_or_else(
                        || !text.is_empty() && text.chars().all(|c| c.is_ascii_digit() || c == '-'),
                        |(whole, fraction)| {
                            !whole.is_empty()
                                && !fraction.is_empty()
                                && whole
                                    .strip_prefix('-')
                                    .unwrap_or(whole)
                                    .chars()
                                    .all(|c| c.is_ascii_digit())
                                && fraction.chars().all(|c| c.is_ascii_digit())
                        },
                    ) =>
            {
                text.parse::<f64>().ok()
            }
            _ => None,
        };
        number
            .filter(|number| number.is_finite())
            .map(Self)
            .ok_or_else(|| D::Error::custom("expected a finite number or decimal numeric string"))
    }
}

impl schemars::JsonSchema for SemanticNumber {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "source_grep_number".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "anyOf": [{ "type": "number" }, { "type": "string" }] })
    }
}

/// Claude Code's public uppercase `Grep` input.  This is deliberately a
/// separate tool from OpenCode's lowercase `grep`: their schemas and result
/// projections are different public contracts.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceGrepInput {
    pub pattern: String,
    pub path: Option<String>,
    pub glob: Option<String>,
    pub output_mode: Option<String>,
    #[serde(rename = "-B")]
    before: Option<SemanticNumber>,
    #[serde(rename = "-A")]
    after: Option<SemanticNumber>,
    #[serde(rename = "-C")]
    context_short: Option<SemanticNumber>,
    context: Option<SemanticNumber>,
    #[serde(rename = "-n")]
    line_numbers: Option<bool>,
    #[serde(rename = "-i")]
    case_insensitive: Option<bool>,
    #[serde(rename = "type")]
    file_type: Option<String>,
    head_limit: Option<SemanticNumber>,
    offset: Option<SemanticNumber>,
    multiline: Option<bool>,
}

fn timeout_override_secs(value: Option<&str>) -> Option<u64> {
    // JavaScript parseInt(value, 10): consume an optional sign and decimal
    // prefix only.  A positive result is the only effective override.
    let text = value.unwrap_or_default().trim_start();
    let digits = text
        .strip_prefix('+')
        .or_else(|| text.strip_prefix('-'))
        .unwrap_or(text)
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    let parsed = digits.parse::<u64>().ok()?;
    if text.starts_with('-') || parsed == 0 {
        None
    } else {
        Some(parsed)
    }
}

fn source_default_timeout_secs() -> u64 {
    #[cfg(target_os = "linux")]
    if std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|release| release.to_ascii_lowercase().contains("microsoft"))
        .unwrap_or(false)
    {
        return 60;
    }
    20
}

fn truncate_utf16(text: String, cap: usize) -> String {
    if text.encode_utf16().count() <= cap {
        return text;
    }
    let mut units = text.encode_utf16().take(cap).collect::<Vec<_>>();
    // JS slice may leave an unpaired surrogate. Rust strings cannot represent
    // one, so retain the largest valid UTF-16 prefix instead.
    if matches!(units.last(), Some(unit) if (0xD800..=0xDBFF).contains(unit)) {
        units.pop();
    }
    String::from_utf16_lossy(&units)
}

fn source_slice<T: Clone>(
    items: &[T],
    offset: SemanticNumber,
    limit: Option<SemanticNumber>,
) -> Vec<T> {
    let len = items.len() as isize;
    let normalize = |index: isize| {
        if index < 0 {
            (len + index).max(0)
        } else {
            index.min(len)
        }
    };
    let raw_start = offset.javascript_integer();
    let start = normalize(raw_start) as usize;
    let end = match limit {
        Some(limit) if limit.javascript_integer() == 0 => len as usize,
        Some(limit) => normalize(raw_start.saturating_add(limit.javascript_integer())) as usize,
        None => normalize(raw_start.saturating_add(250)) as usize,
    };
    if end <= start {
        Vec::new()
    } else {
        items[start..end].to_vec()
    }
}

async fn run_source_grep(
    ctx: xai_tool_runtime::ToolCallContext,
    input: SourceGrepInput,
) -> Result<GrepSearchOutput, xai_tool_runtime::ToolError> {
    let resources = crate::types::tool_metadata::shared_resources(&ctx)?;
    let cwd = crate::types::tool_metadata::resolve_cwd(&ctx, &resources).await?;
    let path = input
        .path
        .clone()
        .unwrap_or_else(|| cwd.to_string_lossy().into_owned());
    let mode = input.output_mode.as_deref().unwrap_or("files_with_matches");
    let mut args = vec!["--hidden".to_string()];
    for excluded in ["!.git", "!.svn", "!.hg", "!.bzr", "!.jj", "!.sl"] {
        args.push("--glob".into());
        args.push(excluded.into());
    }
    args.extend(["--max-columns".into(), "500".into()]);
    if input.multiline.unwrap_or(false) {
        args.extend(["-U".into(), "--multiline-dotall".into()]);
    }
    if input.case_insensitive.unwrap_or(false) {
        args.push("-i".into());
    }
    match mode {
        "content" => {
            if input.line_numbers.unwrap_or(true) {
                args.push("-n".into());
            }
            let c = input.context.or(input.context_short);
            if let Some(n) = c {
                args.extend(["-C".into(), n.ripgrep_arg()]);
            } else {
                if let Some(n) = input.before {
                    args.extend(["-B".into(), n.ripgrep_arg()]);
                }
                if let Some(n) = input.after {
                    args.extend(["-A".into(), n.ripgrep_arg()]);
                }
            }
        }
        "count" => args.extend(["-c".into()]),
        "files_with_matches" => args.push("-l".into()),
        other => {
            return Err(xai_tool_runtime::ToolError::custom(
                "invalid_source_grep_input",
                format!("invalid output_mode: {other}"),
            ));
        }
    }
    if input.pattern.starts_with('-') {
        args.extend(["-e".into(), input.pattern.clone()]);
    } else {
        args.push(input.pattern.clone());
    }
    if let Some(t) = input.file_type.as_deref() {
        args.extend(["--type".into(), t.into()]);
    }
    if let Some(glob) = input.glob.as_deref() {
        for token in glob.split_whitespace().flat_map(|s| {
            if s.contains('{') && s.contains('}') {
                vec![s]
            } else {
                s.split(',').collect()
            }
        }) {
            if !token.is_empty() {
                args.extend(["--glob".into(), token.into()]);
            }
        }
    }
    args.push(path.clone());
    let displayed_timeout_secs = source_default_timeout_secs();
    let timeout_secs = timeout_override_secs(
        std::env::var("CLAUDE_CODE_GLOB_TIMEOUT_SECONDS")
            .ok()
            .as_deref(),
    )
    .unwrap_or(displayed_timeout_secs);
    let cancellation = ctx.get::<xai_tool_runtime::Cancellation>();
    let mut attempt = 0;
    let (stdout, stderr, code) = loop {
        let mut command = Command::new(rg_path());
        command
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());
        crate::util::detach_command(&mut command);
        let mut child = command
            .spawn()
            .map_err(|e| xai_tool_runtime::ToolError::custom("ripgrep_spawn", e.to_string()))?;
        let stdout_reader = child.stdout.take().map(|mut pipe| {
            tokio::spawn(async move {
                let mut buffer = Vec::new();
                pipe.read_to_end(&mut buffer).await.map(|_| buffer)
            })
        });
        let stderr_reader = child.stderr.take().map(|mut pipe| {
            tokio::spawn(async move {
                let mut buffer = Vec::new();
                pipe.read_to_end(&mut buffer).await.map(|_| buffer)
            })
        });
        let mut timed_out = false;
        let mut cancelled = false;
        let status = if let Some(cancellation) = cancellation.as_ref() {
            tokio::select! {
                result = child.wait() => result,
                _ = tokio::time::sleep(std::time::Duration::from_secs(timeout_secs)) => {
                    timed_out = true;
                    let _ = child.kill().await;
                    child.wait().await
                }
                _ = cancellation.0.cancelled() => {
                    cancelled = true;
                    let _ = child.kill().await;
                    child.wait().await
                }
            }
        } else {
            tokio::select! {
                result = child.wait() => result,
                _ = tokio::time::sleep(std::time::Duration::from_secs(timeout_secs)) => {
                    timed_out = true;
                    let _ = child.kill().await;
                    child.wait().await
                }
            }
        }
        .map_err(|e| xai_tool_runtime::ToolError::custom("ripgrep_wait", e.to_string()))?;
        let out = match stdout_reader {
            Some(reader) => reader
                .await
                .map_err(|e| xai_tool_runtime::ToolError::custom("ripgrep_stdout", e.to_string()))?
                .map_err(|e| {
                    xai_tool_runtime::ToolError::custom("ripgrep_stdout", e.to_string())
                })?,
            None => Vec::new(),
        };
        let err = match stderr_reader {
            Some(reader) => reader
                .await
                .map_err(|e| xai_tool_runtime::ToolError::custom("ripgrep_stderr", e.to_string()))?
                .map_err(|e| {
                    xai_tool_runtime::ToolError::custom("ripgrep_stderr", e.to_string())
                })?,
            None => Vec::new(),
        };
        // We do not have a source-proven embedded/system command selector in
        // this Rust path.  Preserve the source embedded capture boundary for
        // the adapter's observable strings without claiming that selection or
        // Darwin preparation is implemented.
        let out = truncate_utf16(
            String::from_utf8_lossy(&out).into_owned(),
            EMBEDDED_CAPTURE_UTF16_CAP,
        )
        .into_bytes();
        let err = truncate_utf16(
            String::from_utf8_lossy(&err).into_owned(),
            EMBEDDED_CAPTURE_UTF16_CAP,
        )
        .into_bytes();
        if cancelled {
            return Err(xai_tool_runtime::ToolError::cancelled(
                xai_tool_protocol::ToolId::new("grep").expect("valid tool id"),
                "Ripgrep search cancelled",
            ));
        }
        if timed_out {
            let mut partial: Vec<_> = String::from_utf8_lossy(&out)
                .lines()
                .map(str::to_owned)
                .collect();
            partial.pop();
            if partial.is_empty() {
                return Err(xai_tool_runtime::ToolError::timeout(
                    xai_tool_protocol::ToolId::new("grep").expect("valid tool id"),
                    format!(
                        "Ripgrep search timed out after {displayed_timeout_secs} seconds. The search may have matched files but did not complete in time. Try searching a more specific path or pattern."
                    ),
                ));
            }
            break (
                partial.join("\n").into_bytes(),
                err,
                status.code().unwrap_or(-1),
            );
        }
        let code = status.code().unwrap_or(-1);
        let stderr_text = String::from_utf8_lossy(&err);
        if attempt == 0
            && (stderr_text.contains("os error 11")
                || stderr_text.contains("Resource temporarily unavailable"))
        {
            args.splice(0..0, ["-j".into(), "1".into()]);
            attempt += 1;
            continue;
        }
        break (out, err, code);
    };
    if code > 1 && stdout.is_empty() {
        return Err(xai_tool_runtime::ToolError::custom(
            "ripgrep_failed",
            String::from_utf8_lossy(&stderr).to_string(),
        ));
    }
    let text = String::from_utf8_lossy(&stdout);
    let all_lines: Vec<String> = text.lines().map(str::to_string).collect();
    let offset = input.offset.unwrap_or(SemanticNumber(0.0));
    let limit = input.head_limit;
    let lines = source_slice(&all_lines, offset, limit);
    let content = lines.join("\n");
    let rendered = match mode {
        "content" => {
            let mut result = if content.is_empty() {
                "No matches found".to_owned()
            } else {
                content
            };
            let effective_limit = limit.unwrap_or(SemanticNumber(250.0));
            let applied_limit = effective_limit.javascript_integer() != 0
                && (all_lines.len() as f64 - offset.0) > effective_limit.0;
            if applied_limit || offset.0 > 0.0 {
                let mut parts = Vec::new();
                if applied_limit {
                    parts.push(format!("limit: {}", effective_limit.ripgrep_arg()));
                }
                if offset.0 > 0.0 {
                    parts.push(format!("offset: {}", offset.ripgrep_arg()));
                }
                result.push_str(&format!(
                    "\n\n[Showing results with pagination = {}]",
                    parts.join(", ")
                ));
            }
            result
        }
        "count" => {
            let matches: usize = lines
                .iter()
                .filter_map(|line| line.rsplit_once(':')?.1.parse::<usize>().ok())
                .sum();
            let raw = if content.is_empty() {
                "No matches found".to_owned()
            } else {
                content
            };
            format!(
                "{raw}\n\nFound {matches} total {} across {} {}.",
                if matches == 1 {
                    "occurrence"
                } else {
                    "occurrences"
                },
                lines.len(),
                if lines.len() == 1 { "file" } else { "files" }
            )
        }
        _ => {
            if lines.is_empty() {
                "No files found".into()
            } else {
                format!(
                    "Found {} {}\n{}",
                    lines.len(),
                    if lines.len() == 1 { "file" } else { "files" },
                    lines.join("\n")
                )
            }
        }
    };
    Ok(GrepSearchOutput {
        stdout: rendered.into_bytes(),
        stderr,
        exit_code: code,
        match_count: lines.len(),
        file_matches: Vec::new(),
    })
}

/// Source-compatible uppercase Claude Code `Grep`.
#[derive(Debug, Default)]
pub struct SourceGrepTool;

impl crate::types::tool_metadata::ToolMetadata for SourceGrepTool {
    fn kind(&self) -> ToolKind {
        ToolKind::Search
    }
    fn tool_namespace(&self) -> ToolNamespace {
        ToolNamespace::OpenCode
    }
    fn description_template(&self) -> &str {
        "Search file contents with ripgrep."
    }
    fn requires_expr(&self) -> Expr<ToolRequirement> {
        Expr::True
    }
}

impl xai_tool_runtime::Tool for SourceGrepTool {
    type Args = SourceGrepInput;
    type Output = GrepSearchOutput;

    fn id(&self) -> xai_tool_protocol::ToolId {
        xai_tool_protocol::ToolId::new("Grep").expect("valid tool id")
    }

    fn description(
        &self,
        _ctx: &xai_tool_runtime::ListToolsContext,
    ) -> xai_tool_types::ToolDescription {
        xai_tool_types::ToolDescription::new(
            "Grep",
            crate::types::tool_metadata::ToolMetadata::description_template(self),
        )
    }

    fn capabilities(&self) -> xai_tool_protocol::ToolCapabilities {
        xai_tool_protocol::ToolCapabilities {
            is_read_only: true,
            tool_scope: Some(xai_tool_protocol::ToolScope::Read),
            ..Default::default()
        }
    }

    async fn run(
        &self,
        ctx: xai_tool_runtime::ToolCallContext,
        input: SourceGrepInput,
    ) -> Result<GrepSearchOutput, xai_tool_runtime::ToolError> {
        run_source_grep(ctx, input).await
    }
}

// ───────────────────────────────────────────────────────────────────────────
// ToolInput conversions (via Dynamic variant)
// ───────────────────────────────────────────────────────────────────────────

impl TryFrom<crate::types::tool_io::ToolInput> for GrepInput {
    type Error = String;
    fn try_from(value: crate::types::tool_io::ToolInput) -> Result<Self, Self::Error> {
        match value {
            crate::types::tool_io::ToolInput::Dynamic(v) => {
                serde_json::from_value(v).map_err(|e| format!("GrepInput: {e}"))
            }
            _ => Err("expected Dynamic variant for GrepInput".into()),
        }
    }
}

impl From<GrepInput> for crate::types::tool_io::ToolInput {
    fn from(value: GrepInput) -> Self {
        crate::types::tool_io::ToolInput::Dynamic(
            serde_json::to_value(value).expect("GrepInput serializes to JSON"),
        )
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Tool implementation
// ───────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct GrepTool;

// ─── Tests ──────────────────────────────────────────────────────────

impl crate::types::tool_metadata::ToolMetadata for GrepTool {
    fn kind(&self) -> ToolKind {
        ToolKind::Search
    }

    fn tool_namespace(&self) -> ToolNamespace {
        ToolNamespace::OpenCode
    }

    fn description_template(&self) -> &str {
        DESCRIPTION
    }

    fn requires_expr(&self) -> Expr<ToolRequirement> {
        Expr::True
    }
}

impl xai_tool_runtime::Tool for GrepTool {
    type Args = GrepInput;
    type Output = GrepSearchOutput;

    fn id(&self) -> xai_tool_protocol::ToolId {
        xai_tool_protocol::ToolId::new("grep").expect("valid tool id")
    }

    fn description(
        &self,
        _ctx: &::xai_tool_runtime::ListToolsContext,
    ) -> xai_tool_types::ToolDescription {
        xai_tool_types::ToolDescription::new(
            "grep",
            crate::types::tool_metadata::ToolMetadata::description_template(self),
        )
    }

    fn capabilities(&self) -> xai_tool_protocol::ToolCapabilities {
        xai_tool_protocol::ToolCapabilities {
            is_read_only: true,
            tool_scope: Some(xai_tool_protocol::ToolScope::Read),
            ..Default::default()
        }
    }

    #[tracing::instrument(name = "tool.opencode.grep", skip_all)]
    async fn run(
        &self,
        ctx: xai_tool_runtime::ToolCallContext,
        input: GrepInput,
    ) -> Result<GrepSearchOutput, xai_tool_runtime::ToolError> {
        use crate::types::tool_metadata::shared_resources;
        let resources = shared_resources(&ctx)?;

        let cwd = crate::types::tool_metadata::resolve_cwd(&ctx, &resources).await?;

        // Resolve search path.
        let search_path = match &input.path {
            Some(p) if !p.is_empty() => {
                let candidate = std::path::Path::new(p);
                if candidate.is_absolute() {
                    candidate.to_path_buf()
                } else {
                    cwd.join(candidate)
                }
            }
            _ => cwd,
        };

        // Build rg command.
        let rg_exec = rg_path();
        let mut cmd = Command::new(rg_exec);
        cmd.args([
            "-n",
            "-H",
            "--hidden",
            "--no-messages",
            "--field-match-separator=|",
            "--regexp",
        ]);
        cmd.arg(&input.pattern);

        if let Some(ref include) = input.include
            && !include.is_empty()
        {
            cmd.arg("--glob").arg(include);
        }

        cmd.arg(search_path.to_string_lossy().as_ref());
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        crate::util::detach_command(&mut cmd);
        cmd.stdin(Stdio::null());

        // Spawn.
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return Ok(GrepSearchOutput {
                    stdout: Vec::new(),
                    stderr: format!("Error spawning rg: {e}").into_bytes(),
                    exit_code: -1,
                    match_count: 0,
                    file_matches: Vec::new(),
                });
            }
        };

        // Read stdout + stderr.
        let mut stdout_buf = Vec::new();
        if let Some(mut pipe) = child.stdout.take() {
            let _ = pipe.read_to_end(&mut stdout_buf).await;
        }
        let mut stderr_buf = Vec::new();
        if let Some(mut pipe) = child.stderr.take() {
            let _ = pipe.read_to_end(&mut stderr_buf).await;
        }

        let status = child.wait().await.ok();
        let exit_code = status.and_then(|s| s.code()).unwrap_or(-1);

        // Exit code 1 = no matches, exit code 2 with no output = errors only.
        let stdout_str = String::from_utf8_lossy(&stdout_buf);
        if exit_code == 1 || (exit_code == 2 && stdout_str.trim().is_empty()) {
            let formatted = "No files found".to_string();
            return Ok(GrepSearchOutput {
                stdout: formatted.into_bytes(),
                stderr: stderr_buf,
                exit_code,
                match_count: 0,
                file_matches: Vec::new(),
            });
        }

        // ── Parse ripgrep output (format: filepath|linenum|linetext) ────
        struct RawMatch {
            path: String,
            line_num: usize,
            line_text: String,
            mtime_ms: u64,
        }

        let mut matches: Vec<RawMatch> = Vec::new();
        let mut mtime_cache: HashMap<String, u64> = HashMap::new();

        for line in stdout_str.lines() {
            if line.is_empty() {
                continue;
            }
            // Split on first two `|` separators.
            let mut parts = line.splitn(3, '|');
            let file_path = match parts.next() {
                Some(p) => p,
                None => continue,
            };
            let line_num_str = match parts.next() {
                Some(n) => n,
                None => continue,
            };
            let line_text = parts.next().unwrap_or("");
            let line_num = match line_num_str.parse::<usize>() {
                Ok(n) => n,
                Err(_) => continue,
            };

            // Cache mtime per file.
            let mtime_ms = if let Some(&cached) = mtime_cache.get(file_path) {
                cached
            } else {
                let mtime = tokio::fs::metadata(file_path)
                    .await
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                mtime_cache.insert(file_path.to_string(), mtime);
                mtime
            };

            matches.push(RawMatch {
                path: file_path.to_string(),
                line_num,
                line_text: line_text.to_string(),
                mtime_ms,
            });
        }

        // Sort by mtime (most recent first).
        matches.sort_by(|a, b| b.mtime_ms.cmp(&a.mtime_ms));

        let total_matches = matches.len();
        let truncated = total_matches > RESULT_LIMIT;
        let final_matches = if truncated {
            &matches[..RESULT_LIMIT]
        } else {
            &matches[..]
        };

        if final_matches.is_empty() {
            let formatted = "No files found".to_string();
            return Ok(GrepSearchOutput {
                stdout: formatted.into_bytes(),
                stderr: stderr_buf,
                exit_code,
                match_count: 0,
                file_matches: Vec::new(),
            });
        }

        // ── Format output grouped by file ───────────────────────────────
        let mut output_lines: Vec<String> = Vec::new();
        if truncated {
            output_lines.push(format!(
                "Found {total_matches} matches (showing first {RESULT_LIMIT})"
            ));
        } else {
            output_lines.push(format!("Found {total_matches} matches"));
        }

        // Build file_matches for structured output + formatted text.
        let mut file_matches: Vec<GrepFileMatch> = Vec::new();
        let mut current_path = String::new();
        let mut current_file_match_lines: Vec<GrepLineMatch> = Vec::new();

        for m in final_matches {
            if m.path != current_path {
                if !current_path.is_empty() {
                    file_matches.push(GrepFileMatch {
                        path: current_path.clone(),
                        matches: std::mem::take(&mut current_file_match_lines),
                    });
                    output_lines.push(String::new()); // blank line between files
                }
                current_path = m.path.clone();
                output_lines.push(format!("{}:", m.path));
            }
            let display_text = if m.line_text.len() > MAX_LINE_LENGTH {
                format!("{}...", &m.line_text[..MAX_LINE_LENGTH])
            } else {
                m.line_text.clone()
            };
            output_lines.push(format!("  Line {}: {}", m.line_num, display_text));
            current_file_match_lines.push(GrepLineMatch {
                line_number: m.line_num,
                content: m.line_text.clone(),
            });
        }
        // Flush last file.
        if !current_path.is_empty() {
            file_matches.push(GrepFileMatch {
                path: current_path,
                matches: current_file_match_lines,
            });
        }

        if truncated {
            output_lines.push(String::new());
            output_lines.push(format!(
                "(Results truncated: showing {RESULT_LIMIT} of {total_matches} matches. Consider using a more specific path or pattern.)"
            ));
        }

        if exit_code == 2 {
            output_lines.push(String::new());
            output_lines.push("(Some paths were inaccessible and skipped)".to_string());
        }

        let formatted = output_lines.join("\n");

        Ok(GrepSearchOutput {
            stdout: formatted.into_bytes(),
            stderr: stderr_buf,
            exit_code: 0, // normalized — we have results
            match_count: total_matches,
            file_matches,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::tool_metadata::test_ctx;

    use crate::types::resources::Resources;
    use tempfile::TempDir;

    /// Build a `Resources` bag with only `Cwd` inserted.
    fn test_resources(cwd: &std::path::Path) -> Resources {
        let mut resources = Resources::new();
        resources.insert(Cwd(cwd.to_path_buf()));
        resources
    }

    // ── tool_metadata ────────────────────────────────────────────────

    #[test]
    fn tool_metadata() {
        use crate::types::tool_metadata::ToolMetadata;
        let tool = GrepTool;
        assert_eq!(xai_tool_runtime::Tool::id(&tool).as_str(), "grep");
        assert_eq!(tool.kind(), ToolKind::Search);
        assert!(
            matches!(tool.tool_namespace(), ToolNamespace::OpenCode),
            "expected OpenCode namespace"
        );
    }

    // ── serde_roundtrip ──────────────────────────────────────────────

    #[test]
    fn serde_roundtrip() {
        let input = GrepInput {
            pattern: "foo.*bar".to_string(),
            path: Some("/tmp/dir".to_string()),
            include: Some("*.rs".to_string()),
        };
        let json = serde_json::to_value(&input).unwrap();
        let back: GrepInput = serde_json::from_value(json).unwrap();
        assert_eq!(back.pattern, "foo.*bar");
        assert_eq!(back.path.as_deref(), Some("/tmp/dir"));
        assert_eq!(back.include.as_deref(), Some("*.rs"));

        // Minimal input — only pattern, optional fields absent.
        let minimal = serde_json::json!({ "pattern": "hello" });
        let parsed: GrepInput = serde_json::from_value(minimal).unwrap();
        assert_eq!(parsed.pattern, "hello");
        assert!(parsed.path.is_none());
        assert!(parsed.include.is_none());
    }

    #[test]
    fn source_numeric_fields_accept_finite_numbers_and_decimal_strings() {
        let input: SourceGrepInput = serde_json::from_value(serde_json::json!({
            "pattern": "needle",
            "-B": "-5",
            "-A": 3.14,
            "-C": "0",
            "context": -2.5,
            "head_limit": "1.25",
            "offset": -1,
        }))
        .unwrap();
        assert_eq!(input.before.unwrap().0, -5.0);
        assert_eq!(input.after.unwrap().0, 3.14);
        assert_eq!(input.context_short.unwrap().0, 0.0);
        assert_eq!(input.context.unwrap().0, -2.5);
        assert_eq!(input.head_limit.unwrap().0, 1.25);
        assert_eq!(input.offset.unwrap().0, -1.0);

        for rejected in ["\"1e3\"", "\"Infinity\"", "\"\"", "true"] {
            let value = format!(r#"{{"pattern":"needle","offset":{rejected}}}"#);
            assert!(
                serde_json::from_str::<SourceGrepInput>(&value).is_err(),
                "{value}"
            );
        }
    }

    #[test]
    fn timeout_override_uses_parse_int_prefix_rules() {
        assert_eq!(timeout_override_secs(Some("5.9")), Some(5));
        assert_eq!(timeout_override_secs(Some("+12seconds")), Some(12));
        assert_eq!(timeout_override_secs(Some("-5")), None);
        assert_eq!(timeout_override_secs(Some("0")), None);
        assert_eq!(timeout_override_secs(Some("nope")), None);
    }

    #[test]
    fn embedded_capture_cap_counts_utf16_units() {
        assert_eq!(truncate_utf16("a😀b".to_owned(), 3), "a😀");
        assert_eq!(truncate_utf16("a😀b".to_owned(), 4), "a😀b");
    }

    // ── basic_match ──────────────────────────────────────────────────

    #[tokio::test]
    async fn lowercase_grep_searches_a_pattern_with_the_source_marker_prefix() {
        let tmp = TempDir::new().unwrap();
        let marker = "__CLAUDE_CODE_GREP__needle";
        std::fs::write(tmp.path().join("marker.txt"), format!("{marker}\n")).unwrap();

        let output = xai_tool_runtime::Tool::run(
            &GrepTool,
            test_ctx(test_resources(tmp.path()).into_shared()),
            GrepInput {
                pattern: marker.to_owned(),
                path: None,
                include: None,
            },
        )
        .await
        .expect("lowercase grep marker pattern should remain searchable");

        assert_eq!(output.match_count, 1);
        assert!(String::from_utf8_lossy(&output.stdout).contains(marker));
    }

    #[tokio::test]
    async fn basic_match() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "hello world\ngoodbye world\n").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "no match here\n").unwrap();

        let tool = GrepTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GrepInput {
                pattern: "hello".to_string(),
                path: None,
                include: None,
            },
        )
        .await
        .unwrap();

        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.contains("Found 1 matches"), "header missing: {text}");
        assert!(text.contains("a.txt:"), "file header missing: {text}");
        assert!(
            text.contains("Line 1: hello world"),
            "line format wrong: {text}"
        );
        assert!(!text.contains("b.txt"), "non-matching file listed: {text}");
        assert_eq!(output.match_count, 1);
        assert_eq!(output.exit_code, 0);
    }

    // ── no_matches ───────────────────────────────────────────────────

    #[tokio::test]
    async fn no_matches() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "hello world\n").unwrap();

        let tool = GrepTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GrepInput {
                pattern: "zzz_nonexistent_zzz".to_string(),
                path: None,
                include: None,
            },
        )
        .await
        .unwrap();

        let text = String::from_utf8_lossy(&output.stdout);
        assert!(
            text.contains("No files found"),
            "expected 'No files found': {text}"
        );
        assert_eq!(output.match_count, 0);
        assert!(output.file_matches.is_empty());
    }

    // ── multiple_files_grouped ───────────────────────────────────────

    #[tokio::test]
    async fn multiple_files_grouped() {
        let tmp = TempDir::new().unwrap();
        // Create two files, both containing the pattern.
        std::fs::write(
            tmp.path().join("first.txt"),
            "match_me line1\nmatch_me line2\n",
        )
        .unwrap();
        // Small delay so mtime differs and order is deterministic.
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(tmp.path().join("second.txt"), "match_me line1\n").unwrap();

        let tool = GrepTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GrepInput {
                pattern: "match_me".to_string(),
                path: None,
                include: None,
            },
        )
        .await
        .unwrap();

        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.contains("Found 3 matches"), "header: {text}");

        // Verify blank line separates the two file groups.
        // Output format: header\n{path1}:\n  Line ...\n  Line ...\n\n{path2}:\n  Line ...
        let lines: Vec<&str> = text.lines().collect();
        // Find the blank separator line between groups.
        let blank_positions: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.is_empty())
            .map(|(i, _)| i)
            .collect();
        assert!(
            !blank_positions.is_empty(),
            "expected blank line between file groups: {text}"
        );

        // Both files should appear.
        assert!(text.contains("first.txt:"), "first.txt missing: {text}");
        assert!(text.contains("second.txt:"), "second.txt missing: {text}");
        assert_eq!(output.match_count, 3);
    }

    // ── include_glob_filter ──────────────────────────────────────────

    #[tokio::test]
    async fn include_glob_filter() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("code.rs"), "fn hello() {}\n").unwrap();
        std::fs::write(tmp.path().join("code.py"), "def hello(): pass\n").unwrap();

        let tool = GrepTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GrepInput {
                pattern: "hello".to_string(),
                path: None,
                include: Some("*.rs".to_string()),
            },
        )
        .await
        .unwrap();

        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.contains("code.rs"), "expected code.rs match: {text}");
        assert!(
            !text.contains("code.py"),
            "code.py should be excluded: {text}"
        );
        assert_eq!(output.match_count, 1);
    }

    // ── path_parameter_absolute ──────────────────────────────────────

    #[tokio::test]
    async fn path_parameter_absolute() {
        let tmp = TempDir::new().unwrap();
        let sub = tmp.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("target.txt"), "needle\n").unwrap();
        std::fs::write(tmp.path().join("root.txt"), "needle\n").unwrap();

        let tool = GrepTool;
        let resources = test_resources(tmp.path());

        // Pass an absolute path to the subdirectory.
        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GrepInput {
                pattern: "needle".to_string(),
                path: Some(sub.to_string_lossy().into_owned()),
                include: None,
            },
        )
        .await
        .unwrap();

        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.contains("target.txt"), "expected target.txt: {text}");
        assert!(
            !text.contains("root.txt"),
            "root.txt should be excluded: {text}"
        );
        assert_eq!(output.match_count, 1);
    }

    // ── path_parameter_relative ──────────────────────────────────────

    #[tokio::test]
    async fn path_parameter_relative() {
        let tmp = TempDir::new().unwrap();
        let sub = tmp.path().join("mydir");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("inner.txt"), "needle\n").unwrap();
        std::fs::write(tmp.path().join("outer.txt"), "needle\n").unwrap();

        let tool = GrepTool;
        let resources = test_resources(tmp.path());

        // Pass a relative path — should resolve against Cwd.
        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GrepInput {
                pattern: "needle".to_string(),
                path: Some("mydir".to_string()),
                include: None,
            },
        )
        .await
        .unwrap();

        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.contains("inner.txt"), "expected inner.txt: {text}");
        assert!(
            !text.contains("outer.txt"),
            "outer.txt should be excluded: {text}"
        );
        assert_eq!(output.match_count, 1);
    }

    // ── match_count_field ────────────────────────────────────────────

    #[tokio::test]
    async fn match_count_field() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("multi.txt"),
            "alpha\nbeta\nalpha again\nalpha third\n",
        )
        .unwrap();

        let tool = GrepTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GrepInput {
                pattern: "alpha".to_string(),
                path: None,
                include: None,
            },
        )
        .await
        .unwrap();

        // Three lines contain "alpha".
        assert_eq!(output.match_count, 3);
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.contains("Found 3 matches"), "header: {text}");
    }

    // ── file_matches_populated ───────────────────────────────────────

    #[tokio::test]
    async fn file_matches_populated() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("f.txt"),
            "line1 target\nline2\nline3 target\n",
        )
        .unwrap();

        let tool = GrepTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GrepInput {
                pattern: "target".to_string(),
                path: None,
                include: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(output.file_matches.len(), 1);
        let fm = &output.file_matches[0];
        assert!(
            fm.path.contains("f.txt"),
            "path should contain f.txt: {}",
            fm.path
        );
        assert_eq!(fm.matches.len(), 2);

        // First match: line 1.
        assert_eq!(fm.matches[0].line_number, 1);
        assert!(fm.matches[0].content.contains("target"));
        // Second match: line 3.
        assert_eq!(fm.matches[1].line_number, 3);
        assert!(fm.matches[1].content.contains("target"));
    }

    // ── missing_cwd_resource ─────────────────────────────────────────

    #[tokio::test]
    async fn missing_cwd_resource() {
        let tool = GrepTool;
        let resources = Resources::new(); // No Cwd inserted.

        let result = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GrepInput {
                pattern: "anything".to_string(),
                path: None,
                include: None,
            },
        )
        .await;

        assert!(result.is_err(), "expected error for missing Cwd");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Cwd not available"),
            "wrong error message: {err:?}"
        );
    }

    // ── mtime_sorting ───────────────────────────────────────────────

    #[tokio::test]
    async fn mtime_sorting() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("old.txt"), "match\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(tmp.path().join("new.txt"), "match\n").unwrap();

        let tool = GrepTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GrepInput {
                pattern: "match".to_string(),
                path: None,
                include: None,
            },
        )
        .await
        .unwrap();

        let text = String::from_utf8_lossy(&output.stdout);
        let new_pos = text.find("new.txt").expect("new.txt not in output");
        let old_pos = text.find("old.txt").expect("old.txt not in output");
        assert!(
            new_pos < old_pos,
            "expected new.txt before old.txt (mtime sort): {text}"
        );
    }

    // ── match_cap_100 ───────────────────────────────────────────────

    #[tokio::test]
    async fn match_cap_100() {
        let tmp = TempDir::new().unwrap();
        let content: String = (0..150).map(|_| "pattern\n").collect();
        std::fs::write(tmp.path().join("big.txt"), &content).unwrap();

        let tool = GrepTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GrepInput {
                pattern: "pattern".to_string(),
                path: None,
                include: None,
            },
        )
        .await
        .unwrap();

        let text = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            output.match_count, 150,
            "match_count should be total: {text}"
        );
        assert!(
            text.contains("showing 100 of 150"),
            "expected truncation message: {text}"
        );
        // Count "  Line " prefixes — exactly 100 displayed lines.
        let displayed = text.matches("  Line ").count();
        assert_eq!(
            displayed, 100,
            "expected 100 displayed lines, got {displayed}: {text}"
        );
    }

    // ── line_truncation_2000_chars ──────────────────────────────────

    #[tokio::test]
    async fn line_truncation_2000_chars() {
        let tmp = TempDir::new().unwrap();
        let long_line = format!("MATCH{}", "x".repeat(2500));
        std::fs::write(tmp.path().join("long.txt"), format!("{long_line}\n")).unwrap();

        let tool = GrepTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GrepInput {
                pattern: "MATCH".to_string(),
                path: None,
                include: None,
            },
        )
        .await
        .unwrap();

        let text = String::from_utf8_lossy(&output.stdout);
        // Find the formatted "  Line N: ..." output line.
        let match_line = text
            .lines()
            .find(|l| l.trim_start().starts_with("Line "))
            .expect("no match line in output");
        assert!(
            match_line.ends_with("..."),
            "expected truncated line ending with '...': {match_line}"
        );
        // The displayed content after "  Line N: " should be at most 2003 chars (2000 + "...").
        let content_start = match_line.find(": ").unwrap() + 2;
        let display_content = &match_line[content_start..];
        assert!(
            display_content.len() <= 2003,
            "display content length {} exceeds 2003: {display_content}",
            display_content.len()
        );
    }

    // ── exit_code_1_no_matches ──────────────────────────────────────

    #[tokio::test]
    async fn exit_code_1_no_matches() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "hello world\n").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "goodbye world\n").unwrap();

        let tool = GrepTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GrepInput {
                pattern: "zzz_absolutely_nothing_zzz".to_string(),
                path: None,
                include: None,
            },
        )
        .await
        .unwrap();

        let text = String::from_utf8_lossy(&output.stdout);
        assert_eq!(text.as_ref(), "No files found", "stdout: {text}");
        assert_eq!(output.match_count, 0);
    }

    // ── special_regex_chars ─────────────────────────────────────────

    #[tokio::test]
    async fn special_regex_chars() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("code.txt"), "log(err)\nlogout\n").unwrap();

        let tool = GrepTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GrepInput {
                pattern: r"log\(".to_string(),
                path: None,
                include: None,
            },
        )
        .await
        .unwrap();

        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.contains("log(err)"), "should match log(err): {text}");
        assert!(
            !text.contains("logout"),
            "should NOT match 'logout': {text}"
        );
        assert_eq!(output.match_count, 1);
    }

    // ── pipe_in_match_text ──────────────────────────────────────────

    #[tokio::test]
    async fn pipe_in_match_text() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("pipes.txt"), "a | b | c\n").unwrap();

        let tool = GrepTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GrepInput {
                pattern: "a".to_string(),
                path: None,
                include: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(output.file_matches.len(), 1);
        let line_match = &output.file_matches[0].matches[0];
        assert!(
            line_match.content.contains("a | b | c"),
            "pipe chars must survive parsing; got: {}",
            line_match.content
        );
    }

    // ── empty_pattern ───────────────────────────────────────────────

    #[tokio::test]
    async fn empty_pattern() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("any.txt"), "some content\n").unwrap();

        let tool = GrepTool;
        let resources = test_resources(tmp.path());

        let result = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GrepInput {
                pattern: "".to_string(),
                path: None,
                include: None,
            },
        )
        .await;

        // Empty pattern may match everything or produce an error from rg.
        // The key assertion: no panic, returns Ok.
        assert!(result.is_ok(), "empty pattern should not panic: {result:?}");
    }

    // ── exit_code_2_with_output ─────────────────────────────────────

    #[tokio::test]
    #[cfg(unix)]
    async fn exit_code_2_with_output() {
        // ripgrep returns exit code 2 when some paths are inaccessible
        // but valid matches exist in other files.
        // We trigger this by creating a broken symlink alongside a real file.
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("real.txt"), "findme\n").unwrap();
        // Create a broken symlink: link -> nonexistent_target
        std::os::unix::fs::symlink(
            tmp.path().join("nonexistent_target"),
            tmp.path().join("broken_link"),
        )
        .unwrap();

        let tool = GrepTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GrepInput {
                pattern: "findme".to_string(),
                path: None,
                include: None,
            },
        )
        .await
        .unwrap();

        let text = String::from_utf8_lossy(&output.stdout);
        // We expect the match from real.txt to be present.
        assert!(
            text.contains("real.txt"),
            "expected real.txt match in output: {text}"
        );
        // Note: ripgrep's --no-messages flag suppresses error messages so rg
        // may not return exit code 2 for a broken symlink. If rg returns 0
        // instead, the "(Some paths were inaccessible)" message won't appear.
        // We still verify the match was found; the exit-code-2 path is
        // exercised only when rg actually reports partial errors.
        assert!(output.match_count >= 1, "should have at least 1 match");
    }

    // ── exit_code_2_without_output ──────────────────────────────────

    // Skipped: triggering ripgrep exit code 2 with zero stdout (errors
    // only, no matches) is impractical in a unit test with real `rg`.
    // The code path (line 189) returns "No files found" and is simple
    // enough to verify by inspection. Documented as a known gap.
}
