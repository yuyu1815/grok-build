//! `glob` tool — OpenCode architecture (`Tool` trait).
//!
//! File pattern matching using ripgrep's `--files` mode with glob filters.
//! Returns matching file paths sorted by modification time (oldest first),
//! capped at 100 results.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::implementations::grok_build::grep::ripgrep::rg_path;
use crate::types::output::ToolOutput;
#[allow(unused_imports)]
use crate::types::resources::{
    Cwd, DenyReadGlobs, DisplayCwd, SharedResources, display_cwd_or_cwd, resolve_model_path,
};
use crate::types::tool::{ToolKind, ToolNamespace};
use crate::types::tool_io::ToolInput;

// ─── Constants ──────────────────────────────────────────────────────

const DEFAULT_RESULT_LIMIT: usize = 100;

// ─── Description ────────────────────────────────────────────────────

const DESCRIPTION: &str = r#"Lists files and directories in a given path.

Other details:
    - The result does not display dot-files and dot-directories.
    - Respects .gitignore patterns (files/directories ignored by git are not shown).
    - Large directories are summarized with file counts and extension breakdowns instead of listing all files."#;

// ─── Input ──────────────────────────────────────────────────────────

/// Input for the `glob` tool.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GlobInput {
    /// Glob pattern to match files against (e.g. "**/*.ts", "src/**/*.tsx").
    pub pattern: String,

    /// Directory to search in. Defaults to the current working directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
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
    if let Ok(value) = std::env::var("CLAUDE_CODE_GLOB_TIMEOUT_SECONDS") {
        if let Ok(seconds) = value.parse::<u64>() {
            if seconds > 0 {
                return Duration::from_secs(seconds);
            }
        }
    }
    if std::env::var_os("WSL_DISTRO_NAME").is_some() {
        Duration::from_secs(60)
    } else {
        Duration::from_secs(20)
    }
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
    let mut child = command.spawn().map_err(|error| {
        xai_tool_runtime::ToolError::execution(
            xai_tool_protocol::ToolId::new("glob").expect("valid tool id"),
            error.to_string(),
        )
    })?;
    let mut stdout_pipe = child.stdout.take().ok_or_else(|| {
        xai_tool_runtime::ToolError::execution(
            xai_tool_protocol::ToolId::new("glob").expect("valid tool id"),
            "ripgrep stdout was not piped",
        )
    })?;
    let mut stderr_pipe = child.stderr.take().ok_or_else(|| {
        xai_tool_runtime::ToolError::execution(
            xai_tool_protocol::ToolId::new("glob").expect("valid tool id"),
            "ripgrep stderr was not piped",
        )
    })?;
    let stdout_task = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stdout_pipe.read_to_end(&mut bytes).await.map(|_| bytes)
    });
    let stderr_task = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stderr_pipe.read_to_end(&mut bytes).await.map(|_| bytes)
    });

    let deadline = Instant::now() + timeout_duration();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|error| {
            xai_tool_runtime::ToolError::execution(
                xai_tool_protocol::ToolId::new("glob").expect("valid tool id"),
                error.to_string(),
            )
        })? {
            break status;
        }
        if cancellation.is_cancelled() {
            let _ = child.kill().await;
            let _ = stdout_task.await;
            let _ = stderr_task.await;
            return Err(xai_tool_runtime::ToolError::cancelled(
                xai_tool_protocol::ToolId::new("glob").expect("valid tool id"),
                "Glob search was cancelled",
            ));
        }
        if Instant::now() >= deadline {
            let _ = child.kill().await;
            let stdout = stdout_task
                .await
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default();
            let _ = stderr_task.await;
            let text = String::from_utf8_lossy(&stdout);
            let mut lines: Vec<&str> = text.lines().collect();
            if !text.ends_with('\n') {
                lines.pop();
            }
            if lines.is_empty() {
                return Err(xai_tool_runtime::ToolError::timeout(
                    xai_tool_protocol::ToolId::new("glob").expect("valid tool id"),
                    "Glob search timed out",
                ));
            }
            return Ok(RipgrepOutput {
                stdout: lines.join("\n"),
                stderr: String::new(),
                status: child.wait().await.map_err(|error| {
                    xai_tool_runtime::ToolError::execution(
                        xai_tool_protocol::ToolId::new("glob").expect("valid tool id"),
                        error.to_string(),
                    )
                })?,
            });
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    };

    let stdout = stdout_task
        .await
        .map_err(|error| {
            xai_tool_runtime::ToolError::execution(
                xai_tool_protocol::ToolId::new("glob").expect("valid tool id"),
                error.to_string(),
            )
        })?
        .map_err(|error| {
            xai_tool_runtime::ToolError::execution(
                xai_tool_protocol::ToolId::new("glob").expect("valid tool id"),
                error.to_string(),
            )
        })?;
    let stderr = stderr_task
        .await
        .map_err(|error| {
            xai_tool_runtime::ToolError::execution(
                xai_tool_protocol::ToolId::new("glob").expect("valid tool id"),
                error.to_string(),
            )
        })?
        .map_err(|error| {
            xai_tool_runtime::ToolError::execution(
                xai_tool_protocol::ToolId::new("glob").expect("valid tool id"),
                error.to_string(),
            )
        })?;
    Ok(RipgrepOutput {
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        status,
    })
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
            "glob",
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

        let permission_path = resolve_model_path(
            &cwd,
            display_cwd.as_deref(),
            &input.path.clone().unwrap_or_default(),
        );

        if let Some(path) = input.path.as_deref().filter(|path| !path.is_empty()) {
            let metadata = tokio::fs::metadata(&permission_path).await.map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    xai_tool_runtime::ToolError::invalid_arguments(format!(
                        "Directory does not exist: {path}."
                    ))
                } else {
                    xai_tool_runtime::ToolError::execution(
                        xai_tool_protocol::ToolId::new("glob").expect("valid tool id"),
                        err.to_string(),
                    )
                }
            })?;
            if !metadata.is_dir() {
                return Err(xai_tool_runtime::ToolError::invalid_arguments(format!(
                    "Path is not a directory: {path}"
                )));
            }
        }

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
        let mut result = run_ripgrep(&args, &search_dir, &cancellation).await?;
        if !result.status.success()
            && result.status.code() != Some(1)
            && (result.stderr.contains("os error 11")
                || result.stderr.contains("Resource temporarily unavailable"))
        {
            args.splice(0..0, ["-j".to_string(), "1".to_string()]);
            result = run_ripgrep(&args, &search_dir, &cancellation).await?;
        }
        if !result.status.success() && result.status.code() != Some(1) {
            let critical = result.stderr.contains("ENOENT")
                || result.stderr.contains("EACCES")
                || result.stderr.contains("EPERM")
                || result.stdout.trim().is_empty();
            if critical {
                return Err(xai_tool_runtime::ToolError::execution(
                    xai_tool_protocol::ToolId::new("glob").expect("valid tool id"),
                    format!(
                        "ripgrep failed with status {}: {}",
                        result.status,
                        result.stderr.trim()
                    ),
                ));
            }
            let mut lines: Vec<&str> = result.stdout.lines().collect();
            if !result.stdout.ends_with('\n') {
                lines.pop();
            }
            result.stdout = lines.join("\n");
        }

        // ── Parse file paths from stdout ────────────────────────
        let stdout = result.stdout;
        let mut truncated = false;

        // Keep the source order and apply the default limit after rg sorting.
        let mut entries: Vec<PathBuf> = Vec::new();
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if entries.len() >= DEFAULT_RESULT_LIMIT {
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

        // Round-trip through serde_json::Value
        let value = serde_json::to_value(&input).unwrap();
        let back: GlobInput = serde_json::from_value(value).unwrap();
        assert_eq!(back.pattern, "**/*.ts");
        assert_eq!(back.path.as_deref(), Some("src"));

        assert!(serde_json::from_str::<GlobInput>(r#"{"pattern":"*.rs","extra":true}"#).is_err());
        assert!(serde_json::from_str::<GlobInput>(r#"{"path":"src"}"#).is_err());
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
