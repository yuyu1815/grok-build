//! `glob` tool — OpenCode architecture (`Tool` trait).
//!
//! File pattern matching using ripgrep's `--files` mode with glob filters.
//! Returns matching file paths sorted by modification time (oldest first),
//! capped at 100 results.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

#[cfg(all(target_os = "macos", bundle_rg))]
use std::sync::OnceLock;

use serde::{Deserialize, de::Error as _};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::implementations::grok_build::grep::ripgrep::{rg_path, uses_bundled_rg};
use crate::types::output::ToolOutput;
#[allow(unused_imports)]
use crate::types::resources::{
    Cwd, DenyReadGlobs, DisplayCwd, Params, SharedResources, display_cwd_or_cwd, resolve_model_path,
};
use crate::types::tool::{ToolKind, ToolNamespace};
use crate::types::tool_io::ToolInput;

// ─── Constants ──────────────────────────────────────────────────────

const DEFAULT_RESULT_LIMIT: usize = 100;
const MAX_BUFFER_SIZE: usize = 20_000_000;

/// Per-tool configuration supplied through the production registry.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlobParams {
    /// Source `globLimits.maxResults`; omitted uses the source default (100).
    pub max_results: Option<usize>,
}
crate::register_resource!("opencode", "Glob", GlobParams);

// ─── Description ────────────────────────────────────────────────────

const DESCRIPTION: &str = r#"- Fast file pattern matching tool that works with any codebase size
- Supports glob patterns like "**/*.js" or "src/**/*.ts"
- Returns matching file paths sorted by modification time
- Use this tool when you need to find files by name patterns
- When you are doing an open ended search that may require multiple rounds of globbing and grepping, use the Agent tool instead"#;

// ─── Input ──────────────────────────────────────────────────────────

/// Input for the `glob` tool.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GlobInput {
    /// The glob pattern to match files against.
    #[schemars(description = "The glob pattern to match files against")]
    pub pattern: String,

    /// The directory to search in. If not specified, the current working directory will be used. IMPORTANT: Omit this field to use the default directory. DO NOT enter "undefined" or "null" - simply omit it for the default behavior. Must be a valid directory path if provided.
    #[serde(
        default,
        deserialize_with = "deserialize_optional_non_null_string",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(schema_with = "optional_string_schema")]
    #[schemars(
        description = "The directory to search in. If not specified, the current working directory will be used. IMPORTANT: Omit this field to use the default directory. DO NOT enter \"undefined\" or \"null\" - simply omit it for the default behavior. Must be a valid directory path if provided."
    )]
    pub path: Option<String>,
}

/// `Option<String>` normally advertises `null` in its generated schema, but
/// the source's `z.string().optional()` accepts omission only. Keep the field
/// optional through its Rust type while exposing a string-only value schema.
fn optional_string_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({ "type": "string" })
}

/// Source `z.string().optional()` accepts an omitted `path`, but rejects an
/// explicit JSON `null`. Serde normally maps both to `None` for
/// `Option<String>`, so preserve that source distinction at schema parsing.
fn deserialize_optional_non_null_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    if value.is_null() {
        return Err(D::Error::custom("path must be a string, not null"));
    }
    serde_json::from_value(value)
        .map(Some)
        .map_err(D::Error::custom)
}

impl TryFrom<ToolInput> for GlobInput {
    type Error = String;
    fn try_from(value: ToolInput) -> Result<Self, Self::Error> {
        match value {
            ToolInput::Dynamic(v) => {
                serde_json::from_value(v).map_err(|e| format!("GlobInput: {e}"))
            }
            _ => Err("expected Dynamic variant for GlobInput".into()),
        }
    }
}

impl From<GlobInput> for ToolInput {
    fn from(value: GlobInput) -> Self {
        ToolInput::Dynamic(serde_json::to_value(value).expect("GlobInput serializes to JSON"))
    }
}

// ─── Output ─────────────────────────────────────────────────────────

/// Structured output for the `glob` tool.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GlobOutput {
    pub filenames: Vec<String>,
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
    #[serde(rename = "numFiles")]
    pub num_files: usize,
    pub truncated: bool,
}

impl xai_tool_runtime::ToolOutput for GlobOutput {}

#[cfg(test)]
impl GlobOutput {
    fn model_text(&self) -> String {
        if self.filenames.is_empty() {
            return "No files found".to_string();
        }
        let mut lines = self.filenames.clone();
        if self.truncated {
            lines.push(
                "(Results are truncated. Consider using a more specific path or pattern.)"
                    .to_string(),
            );
        }
        lines.join("\n")
    }
}

impl From<GlobOutput> for ToolOutput {
    fn from(output: GlobOutput) -> Self {
        let text = if output.filenames.is_empty() {
            "No files found".to_string()
        } else {
            let mut lines = output.filenames;
            if output.truncated {
                lines.push(
                    "(Results are truncated. Consider using a more specific path or pattern.)"
                        .to_string(),
                );
            }
            lines.join("\n")
        };
        ToolOutput::Text(text.into())
    }
}

fn env_flag(name: &str) -> bool {
    match std::env::var(name).ok().as_deref().map(str::trim) {
        None | Some("") => true,
        Some(value) => matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
    }
}

fn timeout_duration() -> Duration {
    // JavaScript `parseInt(value || '', 10) || 0`: consume a leading integer,
    // then fall back for zero, negative, NaN, empty, and unset values.
    if let Some(seconds) = std::env::var("CLAUDE_CODE_GLOB_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| parse_js_int(&value))
        .filter(|seconds| *seconds > 0)
    {
        return Duration::from_secs(seconds as u64);
    }
    if std::env::var_os("WSL_DISTRO_NAME").is_some() {
        Duration::from_secs(60)
    } else {
        Duration::from_secs(20)
    }
}

fn parse_js_int(value: &str) -> Option<i64> {
    let value = value.trim_start();
    let (negative, digits) = match value.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, value.strip_prefix('+').unwrap_or(value)),
    };
    let prefix: String = digits
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    if prefix.is_empty() {
        return None;
    }
    prefix
        .parse::<i64>()
        .ok()
        .map(|number| if negative { -number } else { number })
}

/// Source `codesignRipgrepIfNecessary()` may mutate the bundled macOS binary
/// before every search attempt, but performs the check at most once per process.
async fn codesign_ripgrep_if_necessary() {
    #[cfg(all(target_os = "macos", bundle_rg))]
    {
        static CHECKED: OnceLock<()> = OnceLock::new();
        if CHECKED.set(()).is_err() || !uses_bundled_rg() {
            return;
        }
        let binary = rg_path();
        let check = Command::new("codesign")
            .args(["-vv", "-d"])
            .arg(&binary)
            .output()
            .await;
        let Ok(check) = check else {
            tracing::debug!(path = %binary.display(), "ripgrep codesign check failed");
            return;
        };
        if !String::from_utf8_lossy(&check.stdout).contains("linker-signed") {
            return;
        }
        if let Err(error) = Command::new("codesign")
            .args([
                "--sign",
                "-",
                "--force",
                "--preserve-metadata=entitlements,requirements,flags,runtime",
            ])
            .arg(&binary)
            .status()
            .await
        {
            tracing::debug!(%error, path = %binary.display(), "ripgrep codesign repair failed");
        }
        if let Err(error) = Command::new("xattr")
            .args(["-d", "com.apple.quarantine"])
            .arg(&binary)
            .status()
            .await
        {
            tracing::debug!(%error, path = %binary.display(), "ripgrep quarantine removal failed");
        }
    }
    #[cfg(not(all(target_os = "macos", bundle_rg)))]
    {
        let _ = uses_bundled_rg;
    }
}

/// `GlobTool.validateInput` checks the result of `expandPath`, not the raw
/// JSON value.  A POSIX `//server/share` therefore remains a normal POSIX
/// path for validation; only Windows UNC paths skip filesystem metadata.
fn is_windows_unc_path(path: &Path) -> bool {
    #[cfg(windows)]
    {
        let path = path.to_string_lossy();
        path.starts_with("\\\\") || path.starts_with("//")
    }

    #[cfg(not(windows))]
    {
        let _ = path;
        false
    }
}

/// Apply the POSIX-relevant part of source `expandPath` before Glob's UNC
/// guard. Node's POSIX normalization makes `//server/share` a normal
/// `/server/share` path; only Windows retains UNC semantics.
pub fn expand_glob_path(cwd: &Path, display_cwd: Option<&Path>, input: &str) -> PathBuf {
    let path = resolve_model_path(cwd, display_cwd, input);

    #[cfg(not(windows))]
    {
        let path = path.as_os_str().to_string_lossy();
        if let Some(without_first_slash) = path.strip_prefix("//") {
            return PathBuf::from(format!("/{without_first_slash}"));
        }
    }

    path
}

/// Source Glob validation runs after schema parsing and before permission
/// resolution. Kept public so the Claude-facing dispatcher can preserve that
/// ordering; `run` repeats it for lowercase/direct registry callers.
pub async fn validate_path_metadata(
    permission_path: &Path,
    supplied_path: Option<&str>,
    cwd: &Path,
) -> Result<(), xai_tool_runtime::ToolError> {
    let Some(path) = supplied_path.filter(|path| !path.is_empty()) else {
        return Ok(());
    };
    // Claude Code deliberately avoids filesystem operations for expanded
    // Windows UNC paths to prevent credential leakage; permission handling
    // owns the next decision. POSIX `//...` still reaches `metadata`.
    if is_windows_unc_path(permission_path) {
        return Ok(());
    }
    let metadata = match tokio::fs::metadata(permission_path).await {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let mut message = format!(
                "Directory does not exist: {path}. Note: your current working directory is {}.",
                cwd.display()
            );
            if let Some(suggestion) = suggest_path_under_cwd(permission_path, cwd).await {
                message.push_str(&format!(" Did you mean {}?", suggestion.display()));
            }
            return Err(xai_tool_runtime::ToolError::invalid_arguments(message)
                .with_details(serde_json::json!({"errorCode": 1})));
        }
        Err(err) => {
            return Err(xai_tool_runtime::ToolError::execution(
                tool_id(),
                err.to_string(),
            ));
        }
    };
    if !metadata.is_dir() {
        return Err(xai_tool_runtime::ToolError::invalid_arguments(format!(
            "Path is not a directory: {path}"
        ))
        .with_details(serde_json::json!({"errorCode": 2})));
    }
    Ok(())
}

async fn suggest_path_under_cwd(path: &Path, cwd: &Path) -> Option<PathBuf> {
    let parent = cwd.parent()?;
    if !path.starts_with(parent) || path.starts_with(cwd) || path == cwd {
        return None;
    }
    let candidate = cwd.join(path.strip_prefix(parent).ok()?);
    tokio::fs::metadata(&candidate)
        .await
        .ok()
        .map(|_| candidate)
}

fn absolute_pattern_root(pattern: &str) -> Option<(PathBuf, String)> {
    let path = Path::new(pattern);
    if !path.is_absolute() {
        return None;
    }

    let special = pattern.find(['*', '?', '[', '{']);
    let prefix = special.map_or(pattern, |index| &pattern[..index]);
    let separator = prefix.rfind(['/', '\\']);
    let (base, relative) = match separator {
        Some(index) if index == 0 => ("/", &pattern[1..]),
        Some(index) => (&prefix[..index], &pattern[index + 1..]),
        None => (pattern, ""),
    };
    if relative.is_empty() {
        let path = Path::new(base);
        return Some((
            path.parent()?.to_path_buf(),
            path.file_name()?.to_string_lossy().into_owned(),
        ));
    }
    Some((PathBuf::from(base), relative.to_string()))
}

struct RipgrepOutput {
    stdout: String,
    stderr: String,
    status: std::process::ExitStatus,
    /// `true` when timeout/cancellation already applied the source partial-line
    /// transform.  The normal non-zero-exit handling must not drop a second line.
    partial_failure_handled: bool,
}

fn tool_id() -> xai_tool_protocol::ToolId {
    xai_tool_protocol::ToolId::new("glob").expect("valid tool id")
}

fn timeout_error() -> xai_tool_runtime::ToolError {
    let seconds = if std::env::var_os("WSL_DISTRO_NAME").is_some() {
        60
    } else {
        20
    };
    xai_tool_runtime::ToolError::timeout(
        tool_id(),
        format!(
            "Ripgrep search timed out after {seconds} seconds. The search may have matched files but did not complete in time. Try searching a more specific path or pattern."
        ),
    )
}

/// Source `ripGrep()` parses nonempty stdout with `trim().split('\\n')`,
/// removes CR/empty lines, then unconditionally drops the final parsed line
/// for timeout/cancellation failures. It does not inspect newline completeness.
fn retained_timeout_lines(stdout: &[u8]) -> Vec<String> {
    let mut lines = source_lines(&String::from_utf8_lossy(stdout));
    if !lines.is_empty() {
        lines.pop();
    }
    lines
}

/// Match `stdout.trim().split('\\n')`, preserving whitespace that belongs to
/// a filename while dropping only empty records and a trailing carriage return.
fn source_lines(stdout: &str) -> Vec<String> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    trimmed
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line).to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

async fn run_ripgrep(
    args: &[String],
    target: &Path,
    cancellation: &CancellationToken,
) -> Result<RipgrepOutput, xai_tool_runtime::ToolError> {
    let mut command = Command::new(rg_path());
    command
        .args(args)
        .arg(target)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::util::detach_command(&mut command);
    command.stdin(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|error| xai_tool_runtime::ToolError::execution(tool_id(), error.to_string()))?;
    let mut stdout_pipe = child.stdout.take().ok_or_else(|| {
        xai_tool_runtime::ToolError::execution(tool_id(), "ripgrep stdout was not piped")
    })?;
    let mut stderr_pipe = child.stderr.take().ok_or_else(|| {
        xai_tool_runtime::ToolError::execution(tool_id(), "ripgrep stderr was not piped")
    })?;
    let stdout_task = tokio::spawn(async move { read_capped(&mut stdout_pipe).await });
    let stderr_task = tokio::spawn(async move { read_capped(&mut stderr_pipe).await });

    let deadline = Instant::now() + timeout_duration();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| xai_tool_runtime::ToolError::execution(tool_id(), error.to_string()))?
        {
            break status;
        }
        if cancellation.is_cancelled() {
            let _ = child.kill().await;
            let stdout = stdout_task
                .await
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default();
            let _ = stderr_task.await;
            let lines = retained_timeout_lines(&stdout);
            if lines.is_empty() {
                return Err(timeout_error());
            }
            return Ok(RipgrepOutput {
                stdout: lines.join("\n"),
                stderr: String::new(),
                status: child.wait().await.map_err(|error| {
                    xai_tool_runtime::ToolError::execution(tool_id(), error.to_string())
                })?,
                partial_failure_handled: true,
            });
        }
        if Instant::now() >= deadline {
            let _ = child.kill().await;
            let stdout = stdout_task
                .await
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default();
            let _ = stderr_task.await;
            let lines = retained_timeout_lines(&stdout);
            if lines.is_empty() {
                return Err(timeout_error());
            }
            return Ok(RipgrepOutput {
                stdout: lines.join("\n"),
                stderr: String::new(),
                status: child.wait().await.map_err(|error| {
                    xai_tool_runtime::ToolError::execution(tool_id(), error.to_string())
                })?,
                partial_failure_handled: true,
            });
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    };

    let stdout = stdout_task
        .await
        .map_err(|error| xai_tool_runtime::ToolError::execution(tool_id(), error.to_string()))?
        .map_err(|error| xai_tool_runtime::ToolError::execution(tool_id(), error.to_string()))?;
    let stderr = stderr_task
        .await
        .map_err(|error| xai_tool_runtime::ToolError::execution(tool_id(), error.to_string()))?
        .map_err(|error| xai_tool_runtime::ToolError::execution(tool_id(), error.to_string()))?;
    Ok(RipgrepOutput {
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        status,
        partial_failure_handled: false,
    })
}

/// Source embedded ripgrep caps stdout and stderr independently, retains each
/// prefix, and lets the process finish. The cap is not a kill condition.
async fn read_capped<R: tokio::io::AsyncRead + Unpin>(reader: &mut R) -> std::io::Result<Vec<u8>> {
    let mut captured = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            return Ok(captured);
        }
        if captured.len() < MAX_BUFFER_SIZE {
            let remaining = MAX_BUFFER_SIZE - captured.len();
            captured.extend_from_slice(&chunk[..read.min(remaining)]);
        }
    }
}

// ─── Tool ───────────────────────────────────────────────────────────

/// Glob tool — lists files matching a glob pattern, sorted by mtime.
#[derive(Debug, Default)]
pub struct GlobTool;

impl crate::types::tool_metadata::ToolMetadata for GlobTool {
    fn kind(&self) -> ToolKind {
        ToolKind::List
    }

    fn tool_namespace(&self) -> ToolNamespace {
        ToolNamespace::OpenCode
    }

    fn description_template(&self) -> &str {
        DESCRIPTION
    }
}

impl xai_tool_runtime::Tool for GlobTool {
    type Args = GlobInput;
    type Output = GlobOutput;

    fn id(&self) -> xai_tool_protocol::ToolId {
        xai_tool_protocol::ToolId::new("glob").expect("valid tool id")
    }

    fn description(
        &self,
        _ctx: &::xai_tool_runtime::ListToolsContext,
    ) -> xai_tool_types::ToolDescription {
        xai_tool_types::ToolDescription::new(
            "Glob",
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

    #[tracing::instrument(name = "tool.glob", skip_all)]
    async fn run(
        &self,
        ctx: xai_tool_runtime::ToolCallContext,
        input: GlobInput,
    ) -> Result<GlobOutput, xai_tool_runtime::ToolError> {
        use crate::types::tool_metadata::shared_resources;
        let resources = shared_resources(&ctx)?;

        let cwd = crate::types::tool_metadata::resolve_cwd(&ctx, &resources).await?;
        let display_cwd = resources
            .lock()
            .await
            .get::<DisplayCwd>()
            .map(|d| d.0.clone());
        let deny_read_globs = resources
            .lock()
            .await
            .get::<DenyReadGlobs>()
            .map(|d| d.0.clone())
            .unwrap_or_default();
        let result_limit = resources
            .lock()
            .await
            .get::<Params<GlobParams>>()
            .and_then(|params| params.max_results)
            .unwrap_or(DEFAULT_RESULT_LIMIT);

        let permission_path = expand_glob_path(
            &cwd,
            display_cwd.as_deref(),
            &input.path.clone().unwrap_or_default(),
        );

        validate_path_metadata(&permission_path, input.path.as_deref(), &cwd).await?;

        let (search_dir, search_pattern) = absolute_pattern_root(&input.pattern).map_or(
            (permission_path.clone(), input.pattern.clone()),
            |(base, pattern)| (base, pattern),
        );
        let mut args = vec![
            "--files".to_string(),
            "--glob".to_string(),
            search_pattern,
            "--sort=modified".to_string(),
        ];
        if env_flag("CLAUDE_CODE_GLOB_NO_IGNORE") {
            args.push("--no-ignore".to_string());
        }
        if env_flag("CLAUDE_CODE_GLOB_HIDDEN") {
            args.push("--hidden".to_string());
        }
        for deny in &deny_read_globs {
            args.push("--glob".to_string());
            args.push(format!("!{deny}"));
        }
        let cancellation = ctx
            .get::<xai_tool_runtime::Cancellation>()
            .map(|value| value.0.clone())
            .unwrap_or_default();
        let started = Instant::now();
        codesign_ripgrep_if_necessary().await;
        let mut result = run_ripgrep(&args, &search_dir, &cancellation).await?;
        let is_critical = |result: &RipgrepOutput| {
            result.stderr.contains("ENOENT")
                || result.stderr.contains("EACCES")
                || result.stderr.contains("EPERM")
        };
        // Source checks critical process errors before the EAGAIN retry branch.
        if !result.status.success() && result.status.code() != Some(1) && is_critical(&result) {
            return Err(xai_tool_runtime::ToolError::execution(
                tool_id(),
                format!(
                    "ripgrep failed with status {}: {}",
                    result.status,
                    result.stderr.trim()
                ),
            ));
        }
        if !result.status.success()
            && result.status.code() != Some(1)
            && (result.stderr.contains("os error 11")
                || result.stderr.contains("Resource temporarily unavailable"))
        {
            tracing::info!(target: "tools.glob", "tengu_ripgrep_eagain_retry");
            args.splice(0..0, ["-j".to_string(), "1".to_string()]);
            result = run_ripgrep(&args, &search_dir, &cancellation).await?;
        }
        if !result.status.success() && result.status.code() != Some(1) {
            if is_critical(&result) {
                return Err(xai_tool_runtime::ToolError::execution(
                    tool_id(),
                    format!(
                        "ripgrep failed with status {}: {}",
                        result.status,
                        result.stderr.trim()
                    ),
                ));
            }
            if !result.partial_failure_handled {
                // A normal noncritical rg failure may still expose all complete
                // partial lines. Timeout/cancellation were transformed above.
                result.stdout = source_lines(&result.stdout).join("\n");
            }
        }

        // ── Parse file paths from stdout ────────────────────────
        let stdout = result.stdout;
        let mut truncated = false;

        // Keep the source order and apply the default limit after rg sorting.
        let mut entries: Vec<PathBuf> = Vec::new();
        for line in source_lines(&stdout) {
            if entries.len() >= result_limit {
                truncated = true;
                continue;
            }

            let full_path = search_dir.join(line);
            entries.push(full_path);
        }

        // ── Format output ───────────────────────────────────────
        let count = entries.len();
        let entry_paths: Vec<String> = entries
            .iter()
            .map(|path| {
                path.strip_prefix(&cwd)
                    .map(|relative| relative.display().to_string())
                    .unwrap_or_else(|_| path.display().to_string())
            })
            .collect();

        Ok(GlobOutput {
            filenames: entry_paths,
            duration_ms: started.elapsed().as_millis() as u64,
            num_files: count,
            truncated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::tool_metadata::test_ctx;

    use crate::types::resources::Resources;
    use tempfile::TempDir;

    fn test_resources(cwd: &std::path::Path) -> Resources {
        let mut resources = Resources::new();
        resources.insert(Cwd(cwd.to_path_buf()));
        resources
    }

    #[tokio::test]
    async fn glob_finds_matching_files() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("hello.ts"), "console.log('hi');\n").unwrap();
        std::fs::write(tmp.path().join("world.ts"), "export {};\n").unwrap();
        std::fs::write(tmp.path().join("readme.md"), "# readme\n").unwrap();

        let tool = GlobTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GlobInput {
                pattern: "*.ts".to_string(),
                path: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(output.num_files, 2);
        assert!(!output.truncated);
        assert!(output.model_text().contains(".ts"));
        assert!(!output.model_text().contains("readme.md"));
    }

    #[tokio::test]
    async fn glob_no_matches_returns_empty() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("readme.md"), "# readme\n").unwrap();

        let tool = GlobTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GlobInput {
                pattern: "*.xyz_nonexistent".to_string(),
                path: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(output.num_files, 0);
        assert!(!output.truncated);
        assert!(output.model_text().contains("No files found"));
    }

    #[tokio::test]
    async fn glob_with_subdirectory_path() {
        let tmp = TempDir::new().unwrap();
        let sub = tmp.path().join("src");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(tmp.path().join("root.txt"), "root\n").unwrap();

        let tool = GlobTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GlobInput {
                pattern: "*.rs".to_string(),
                path: Some("src".to_string()),
            },
        )
        .await
        .unwrap();

        assert_eq!(output.num_files, 1);
        assert!(output.model_text().contains("main.rs"));
        assert!(!output.model_text().contains("root.txt"));
    }

    #[tokio::test]
    async fn glob_sorts_by_mtime_oldest_first() {
        let tmp = TempDir::new().unwrap();

        // Create files with slight time gaps so mtime differs.
        std::fs::write(tmp.path().join("old.txt"), "old\n").unwrap();
        // Touch a second file after a small delay.
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(tmp.path().join("new.txt"), "new\n").unwrap();

        let tool = GlobTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GlobInput {
                pattern: "*.txt".to_string(),
                path: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(output.num_files, 2);
        // ripgrep --sort=modified lists the oldest file first.
        let new_pos = output.model_text().find("new.txt").unwrap();
        let old_pos = output.model_text().find("old.txt").unwrap();
        assert!(
            old_pos < new_pos,
            "old.txt should appear before new.txt (mtime sort), got old@{} new@{}",
            old_pos,
            new_pos,
        );
    }

    #[tokio::test]
    async fn glob_recursive_pattern() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("a").join("b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(tmp.path().join("top.rs"), "top\n").unwrap();
        std::fs::write(nested.join("deep.rs"), "deep\n").unwrap();

        let tool = GlobTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GlobInput {
                pattern: "**/*.rs".to_string(),
                path: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(output.num_files, 2);
        assert!(output.model_text().contains("top.rs"));
        assert!(output.model_text().contains("deep.rs"));
    }

    #[test]
    fn tool_metadata() {
        use crate::types::tool_metadata::ToolMetadata;
        let tool = GlobTool;
        assert_eq!(xai_tool_runtime::Tool::id(&tool).as_str(), "glob");
        assert!(matches!(tool.kind(), ToolKind::List));
        assert!(matches!(tool.tool_namespace(), ToolNamespace::OpenCode));
    }

    #[test]
    fn serde_roundtrip() {
        let json = r#"{"pattern":"**/*.ts","path":"src"}"#;
        let input: GlobInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.pattern, "**/*.ts");
        assert_eq!(input.path.as_deref(), Some("src"));

        // Minimal — path omitted
        let json_min = r#"{"pattern":"*.rs"}"#;
        let input_min: GlobInput = serde_json::from_str(json_min).unwrap();
        assert_eq!(input_min.pattern, "*.rs");
        assert!(input_min.path.is_none());

        let error = serde_json::from_str::<GlobInput>(r#"{"pattern":"*.rs","path":null}"#)
            .expect_err("an explicit null path must not be treated as omitted");
        assert!(
            error
                .to_string()
                .contains("path must be a string, not null")
        );

        // Round-trip through serde_json::Value
        let value = serde_json::to_value(&input).unwrap();
        let back: GlobInput = serde_json::from_value(value).unwrap();
        assert_eq!(back.pattern, "**/*.ts");
        assert_eq!(back.path.as_deref(), Some("src"));

        assert!(serde_json::from_str::<GlobInput>(r#"{"pattern":"*.rs","extra":true}"#).is_err());
        assert!(serde_json::from_str::<GlobInput>(r#"{"path":"src"}"#).is_err());
    }

    #[test]
    fn registry_schema_exposes_path_as_optional_string_not_nullable() {
        let schema = crate::registry::types::generate_schema::<GlobInput>();
        let properties = schema["properties"]
            .as_object()
            .expect("Glob schema must expose properties");
        let path = properties
            .get("path")
            .expect("Glob schema must expose path");

        assert_eq!(path["type"], "string");
        assert_ne!(
            path["type"],
            serde_json::json!(["string", "null"]),
            "model-facing Glob schema must not permit path: null: {path}"
        );
        assert!(
            !schema["required"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("path")),
            "path remains optional"
        );
    }

    #[test]
    fn public_definition_matches_the_source_descriptions() {
        assert_eq!(
            DESCRIPTION,
            r#"- Fast file pattern matching tool that works with any codebase size
- Supports glob patterns like "**/*.js" or "src/**/*.ts"
- Returns matching file paths sorted by modification time
- Use this tool when you need to find files by name patterns
- When you are doing an open ended search that may require multiple rounds of globbing and grepping, use the Agent tool instead"#,
        );
        let schema = crate::registry::types::generate_schema::<GlobInput>();
        assert_eq!(
            schema["properties"]["pattern"]["description"],
            "The glob pattern to match files against"
        );
        assert_eq!(
            schema["properties"]["path"]["description"],
            "The directory to search in. If not specified, the current working directory will be used. IMPORTANT: Omit this field to use the default directory. DO NOT enter \"undefined\" or \"null\" - simply omit it for the default behavior. Must be a valid directory path if provided."
        );
    }

    #[tokio::test]
    async fn missing_path_includes_source_cwd_note_and_dropped_repo_suggestion() {
        let parent = TempDir::new().unwrap();
        let cwd = parent.path().join("repo");
        std::fs::create_dir_all(cwd.join("src")).unwrap();
        let requested = parent.path().join("src");

        let error = validate_path_metadata(&requested, Some(requested.to_str().unwrap()), &cwd)
            .await
            .expect_err("missing path must fail validation");
        let message = error.to_string();
        assert!(message.contains(&format!(
            "Note: your current working directory is {}.",
            cwd.display()
        )));
        assert!(message.contains(&format!("Did you mean {}?", cwd.join("src").display())));
    }

    #[tokio::test]
    async fn absolute_path_parameter() {
        let tmp = TempDir::new().unwrap();
        let sub = tmp.path().join("abs_target");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("found.txt"), "data\n").unwrap();

        let tool = GlobTool;
        // cwd is the tmp root, but we pass the absolute path to the sub dir
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GlobInput {
                pattern: "*.txt".to_string(),
                path: Some(sub.to_string_lossy().to_string()),
            },
        )
        .await
        .unwrap();

        assert_eq!(output.num_files, 1);
        assert!(output.model_text().contains("found.txt"));
    }

    #[tokio::test]
    async fn empty_directory() {
        let tmp = TempDir::new().unwrap();
        // Directory exists but contains no files.

        let tool = GlobTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GlobInput {
                pattern: "*".to_string(),
                path: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(output.num_files, 0);
        assert!(!output.truncated);
        assert!(output.model_text().contains("No files found"));
    }

    #[tokio::test]
    async fn path_empty_string_defaults_to_cwd() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("root.rs"), "fn main() {}\n").unwrap();

        let tool = GlobTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GlobInput {
                pattern: "*.rs".to_string(),
                path: Some(String::new()),
            },
        )
        .await
        .unwrap();

        assert_eq!(output.num_files, 1);
        assert!(output.model_text().contains("root.rs"));
    }

    #[tokio::test]
    async fn supplied_missing_or_non_directory_path_fails_before_glob() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("not-a-directory.txt");
        std::fs::write(&file, "data\n").unwrap();

        let tool = GlobTool;
        let missing = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(test_resources(tmp.path()).into_shared()),
            GlobInput {
                pattern: "*.rs".to_string(),
                path: Some(tmp.path().join("missing").display().to_string()),
            },
        )
        .await
        .unwrap_err();
        assert!(missing.to_string().contains("Directory does not exist:"));

        let not_directory = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(test_resources(tmp.path()).into_shared()),
            GlobInput {
                pattern: "*.rs".to_string(),
                path: Some(file.display().to_string()),
            },
        )
        .await
        .unwrap_err();
        assert!(
            not_directory
                .to_string()
                .contains("Path is not a directory:")
        );
    }

    #[tokio::test]
    async fn missing_cwd_resource() {
        let tool = GlobTool;
        let resources = Resources::new(); // No Cwd inserted

        let result = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GlobInput {
                pattern: "*.rs".to_string(),
                path: None,
            },
        )
        .await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Cwd not available")
        );
    }

    #[tokio::test]
    async fn result_cap_100() {
        let tmp = TempDir::new().unwrap();
        for i in 0..110 {
            std::fs::write(tmp.path().join(format!("file_{:03}.txt", i)), "data\n").unwrap();
        }

        let tool = GlobTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GlobInput {
                pattern: "*.txt".to_string(),
                path: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(output.num_files, 100);
        assert!(output.truncated);
        assert!(
            output
                .model_text()
                .contains("Results are truncated. Consider using a more specific path or pattern."),
            "expected truncation message, got: {}",
            output.model_text()
        );
    }

    #[tokio::test]
    async fn configured_result_limit_overrides_default() {
        let tmp = TempDir::new().unwrap();
        for i in 0..4 {
            std::fs::write(tmp.path().join(format!("file_{i}.txt")), "data\n").unwrap();
        }

        let mut resources = test_resources(tmp.path());
        resources.insert(Params(GlobParams {
            max_results: Some(2),
        }));
        let output = xai_tool_runtime::Tool::run(
            &GlobTool,
            test_ctx(resources.into_shared()),
            GlobInput {
                pattern: "*.txt".to_string(),
                path: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(output.num_files, 2);
        assert!(output.truncated);
    }

    #[test]
    #[cfg(windows)]
    #[test]
    fn expanded_windows_unc_paths_skip_validation_filesystem_access() {
        assert!(is_windows_unc_path(Path::new(r"\\server\share")));
        assert!(is_windows_unc_path(Path::new("//server/share")));
        assert!(!is_windows_unc_path(Path::new(r"C:\\project")));
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn posix_double_slash_path_runs_filesystem_validation() {
        let path = expand_glob_path(
            Path::new("/workspace"),
            None,
            "//glob-validation-missing-path",
        );
        assert_eq!(path, PathBuf::from("/glob-validation-missing-path"));
        let error = validate_path_metadata(
            &path,
            Some("//glob-validation-missing-path"),
            Path::new("/workspace"),
        )
        .await
        .expect_err("POSIX double-slash paths must not skip validation");

        assert!(error.to_string().contains("Directory does not exist:"));
    }

    #[test]
    fn timeout_partial_result_drops_the_last_parsed_line_even_when_complete() {
        assert_eq!(
            retained_timeout_lines(b"first.rs\nlast.rs\n"),
            vec!["first.rs"]
        );
        assert_eq!(retained_timeout_lines(b"only.rs\n"), Vec::<String>::new());
        assert_eq!(retained_timeout_lines(b"\n\r\n"), Vec::<String>::new());
    }

    #[test]
    fn source_line_parsing_trims_the_stream_not_each_filename() {
        assert_eq!(
            source_lines("first.rs\n  name.rs  \r\nlast.rs\n"),
            vec!["first.rs", "  name.rs  ", "last.rs"]
        );
        assert_eq!(source_lines(" \n "), Vec::<String>::new());
    }

    #[test]
    fn timeout_error_uses_the_source_fixed_message() {
        let message = timeout_error().to_string();
        assert!(message.contains("Ripgrep search timed out after "));
        assert!(message.contains("Try searching a more specific path or pattern."));
    }

    #[test]
    fn timeout_uses_javascript_parse_int_semantics() {
        assert_eq!(parse_js_int("12x"), Some(12));
        assert_eq!(parse_js_int("  +12x"), Some(12));
        assert_eq!(parse_js_int("-1"), Some(-1));
        assert_eq!(parse_js_int("x12"), None);
        assert_eq!(parse_js_int(""), None);
    }

    #[test]
    fn glob_description_matches_the_file_matching_tool_contract() {
        assert!(DESCRIPTION.contains("Fast file pattern matching"));
        assert!(DESCRIPTION.contains("use the Agent tool instead"));
        assert!(!DESCRIPTION.contains("Large directories are summarized"));
    }

    #[tokio::test]
    async fn hidden_files_included() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".hidden.ts"), "hidden\n").unwrap();
        std::fs::write(tmp.path().join("visible.ts"), "visible\n").unwrap();

        let tool = GlobTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GlobInput {
                pattern: "*.ts".to_string(),
                path: None,
            },
        )
        .await
        .unwrap();

        assert!(
            output.num_files >= 2,
            "expected at least 2 files, got {}",
            output.num_files
        );
        assert!(
            output.model_text().contains(".hidden.ts"),
            "hidden file should appear in output: {}",
            output.model_text()
        );
    }

    #[tokio::test]
    async fn deny_read_globs_excludes_confirmed_match() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("visible.rs"), "visible\n").unwrap();
        std::fs::write(tmp.path().join("secret.rs"), "secret\n").unwrap();
        let mut resources = test_resources(tmp.path());
        resources.insert(DenyReadGlobs(vec!["secret.rs".to_string()]));

        let output = xai_tool_runtime::Tool::run(
            &GlobTool,
            test_ctx(resources.into_shared()),
            GlobInput {
                pattern: "*.rs".to_string(),
                path: None,
            },
        )
        .await
        .unwrap();

        assert!(output.filenames.iter().any(|entry| entry == "visible.rs"));
        assert!(!output.filenames.iter().any(|entry| entry == "secret.rs"));
    }

    #[tokio::test]
    async fn no_ignore_includes_gitignored_files() {
        let tmp = TempDir::new().unwrap();

        // Initialize a git repo so --no-ignore has an ignore file to bypass.
        xai_test_utils::git::ensure_hermetic_git_on_path();
        let status = std::process::Command::new("git")
            .args(["init"])
            .current_dir(tmp.path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("git must be available");
        assert!(status.success(), "git init failed");

        // Ignore an entire directory via .gitignore.
        std::fs::write(tmp.path().join(".gitignore"), "ignored_dir/\n").unwrap();

        // Create an ignored directory with a .txt file inside it.
        let ignored = tmp.path().join("ignored_dir");
        std::fs::create_dir(&ignored).unwrap();
        std::fs::write(ignored.join("secret.txt"), "should be ignored\n").unwrap();

        // Create a non-ignored .txt file.
        std::fs::write(tmp.path().join("visible.txt"), "should be found\n").unwrap();

        let tool = GlobTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GlobInput {
                pattern: "**/*.txt".to_string(),
                path: None,
            },
        )
        .await
        .unwrap();

        assert!(
            output.model_text().contains("visible.txt"),
            "visible.txt should be in output: {}",
            output.model_text()
        );
        assert!(
            output.model_text().contains("secret.txt"),
            "secret.txt inside ignored_dir/ should be included with --no-ignore: {}",
            output.model_text()
        );
    }

    #[tokio::test]
    async fn output_is_plain_cwd_relative_paths() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("example.rs"), "fn main() {}\n").unwrap();

        let tool = GlobTool;
        let resources = test_resources(tmp.path());

        let output = xai_tool_runtime::Tool::run(
            &tool,
            test_ctx(resources.into_shared()),
            GlobInput {
                pattern: "*.rs".to_string(),
                path: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(output.model_text(), "example.rs");
        assert_eq!(output.filenames, vec!["example.rs"]);
    }
}
