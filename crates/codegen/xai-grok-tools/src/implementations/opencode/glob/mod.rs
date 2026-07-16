//! `glob` tool — OpenCode architecture (`Tool` trait).
//!
//! File pattern matching using ripgrep's `--files` mode with glob filters.
//! Returns matching file paths sorted by modification time (oldest first),
//! capped at 100 results.

use std::path::PathBuf;
use std::process::Stdio;

use tokio::io::AsyncReadExt;
use tokio::process::Command;

use crate::implementations::grok_build::grep::ripgrep::rg_path;
use crate::types::output::ToolOutput;
#[allow(unused_imports)]
use crate::types::resources::{
    Cwd, DenyReadGlobs, DisplayCwd, SharedResources, display_cwd_or_cwd, resolve_model_path,
};
use crate::types::tool::{ToolKind, ToolNamespace};
use crate::types::tool_io::ToolInput;

// ─── Constants ──────────────────────────────────────────────────────

const RESULT_LIMIT: usize = 100;

/// Hard cap on bytes read from ripgrep's stdout (5 MB).
const MAX_STDOUT_BYTES: usize = 5_000_000;

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
    /// Pre-formatted text for the model prompt.
    pub tool_output_for_prompt: String,
    /// Number of files included in `entries` (capped at `RESULT_LIMIT`).
    pub count: usize,
    /// Total files matched by ripgrep before the cap. May exceed `count`.
    /// When `truncated_by_bytes` was hit this is a lower bound.
    pub total_count: usize,
    /// Whether results were truncated at the limit.
    pub truncated: bool,
    /// Paths relative to cwd when possible, included in `count` and sorted by
    /// modification time. Empty when `count == 0`.
    pub entries: Vec<String>,
    /// The model-facing workspace root used to resolve `path` -- equal to
    /// `display_cwd_or_cwd(cwd, display_cwd)`. Adapters that re-format the
    /// output use this as the relativization base when
    /// the model omits `path`, instead of re-resolving cwd themselves.
    pub cwd_for_display: String,
}

impl xai_tool_runtime::ToolOutput for GlobOutput {}

impl From<GlobOutput> for ToolOutput {
    fn from(output: GlobOutput) -> Self {
        ToolOutput::Text(output.tool_output_for_prompt.into())
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

        // ── Resolve search directory ────────────────────────────
        let search_dir = resolve_model_path(
            &cwd,
            display_cwd.as_deref(),
            &input.path.clone().unwrap_or_default(),
        );

        if let Some(path) = input.path.as_deref().filter(|path| !path.is_empty()) {
            let metadata = tokio::fs::metadata(&search_dir).await.map_err(|err| {
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
        // ── Build ripgrep command ───────────────────────────────
        //   rg --files --glob='!.git/*' --hidden --glob=<pattern> <search_dir>
        let rg_exec = rg_path();
        let mut cmd = Command::new(rg_exec);
        cmd.arg("--files")
            .arg("--glob=!.git/*")
            .arg("--hidden")
            .arg("--no-ignore")
            .arg("--sort=modified")
            .arg("--glob")
            .arg(&input.pattern);
        for deny in &deny_read_globs {
            cmd.arg("--glob").arg(format!("!{deny}"));
        }
        cmd.arg(&search_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        crate::util::detach_command(&mut cmd);
        cmd.stdin(Stdio::null());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return Err(xai_tool_runtime::ToolError::execution(
                    xai_tool_protocol::ToolId::new("glob").expect("valid tool id"),
                    e.to_string(),
                ));
            }
        };

        // ── Read stdout with byte cap ───────────────────────────
        let mut stdout_buf = Vec::with_capacity(MAX_STDOUT_BYTES.min(65_536));
        let mut truncated_by_bytes = false;
        if let Some(mut stdout_pipe) = child.stdout.take() {
            let mut tmp = [0u8; 8192];
            loop {
                match stdout_pipe.read(&mut tmp).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if stdout_buf.len() + n <= MAX_STDOUT_BYTES {
                            stdout_buf.extend_from_slice(&tmp[..n]);
                        } else {
                            let remaining = MAX_STDOUT_BYTES.saturating_sub(stdout_buf.len());
                            if remaining > 0 {
                                stdout_buf.extend_from_slice(&tmp[..remaining]);
                            }
                            truncated_by_bytes = true;
                            let _ = child.start_kill();
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        }

        // Consume stderr to avoid deadlocks.
        if let Some(stderr_pipe) = child.stderr.take() {
            let _ = stderr_pipe
                .take(1_000_000)
                .read_to_end(&mut Vec::new())
                .await;
        }

        let status = child.wait().await.map_err(|err| {
            xai_tool_runtime::ToolError::execution(
                xai_tool_protocol::ToolId::new("glob").expect("valid tool id"),
                err.to_string(),
            )
        })?;
        if !status.success() && status.code() != Some(1) {
            return Err(xai_tool_runtime::ToolError::execution(
                xai_tool_protocol::ToolId::new("glob").expect("valid tool id"),
                format!("ripgrep failed with status {status}"),
            ));
        }

        // ── Parse file paths from stdout ────────────────────────
        let stdout = String::from_utf8_lossy(&stdout_buf);
        let mut truncated = truncated_by_bytes;

        // Collect every match so total_count is accurate. Cap stat()s and
        // the returned entry list at RESULT_LIMIT so we don't pay the syscall
        // cost on huge result sets, but keep counting lines past the cap so
        // the truncation marker can report the real overflow.
        let mut entries: Vec<PathBuf> = Vec::new();
        let mut total_count: usize = 0;
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            total_count += 1;

            if entries.len() >= RESULT_LIMIT {
                truncated = true;
                continue;
            }

            let full_path = search_dir.join(line);
            entries.push(full_path);
        }

        // ── Format output ───────────────────────────────────────
        let count = entries.len();
        let cwd_for_display = display_cwd_or_cwd(&cwd, display_cwd.as_deref());
        let entry_paths: Vec<String> = entries
            .iter()
            .map(|path| {
                path.strip_prefix(&cwd)
                    .map(|relative| relative.display().to_string())
                    .unwrap_or_else(|_| path.display().to_string())
            })
            .collect();

        let tool_output_for_prompt = if entry_paths.is_empty() {
            "No files found".to_string()
        } else {
            let mut lines: Vec<String> = entry_paths.clone();

            if truncated {
                lines.push(
                    "(Results are truncated. Consider using a more specific path or pattern.)"
                        .to_string(),
                );
            }
            lines.join("\n")
        };

        Ok(GlobOutput {
            tool_output_for_prompt,
            count,
            total_count,
            truncated,
            entries: entry_paths,
            cwd_for_display: cwd_for_display.display().to_string(),
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

        assert_eq!(output.count, 2);
        assert!(!output.truncated);
        assert!(output.tool_output_for_prompt.contains(".ts"));
        assert!(!output.tool_output_for_prompt.contains("readme.md"));
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

        assert_eq!(output.count, 0);
        assert!(!output.truncated);
        assert!(output.tool_output_for_prompt.contains("No files found"));
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

        assert_eq!(output.count, 1);
        assert!(output.tool_output_for_prompt.contains("main.rs"));
        assert!(!output.tool_output_for_prompt.contains("root.txt"));
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

        assert_eq!(output.count, 2);
        // ripgrep --sort=modified lists the oldest file first.
        let new_pos = output.tool_output_for_prompt.find("new.txt").unwrap();
        let old_pos = output.tool_output_for_prompt.find("old.txt").unwrap();
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

        assert_eq!(output.count, 2);
        assert!(output.tool_output_for_prompt.contains("top.rs"));
        assert!(output.tool_output_for_prompt.contains("deep.rs"));
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

        assert_eq!(output.count, 1);
        assert!(output.tool_output_for_prompt.contains("found.txt"));
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

        assert_eq!(output.count, 0);
        assert!(!output.truncated);
        assert!(output.tool_output_for_prompt.contains("No files found"));
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

        assert_eq!(output.count, 1);
        assert!(output.tool_output_for_prompt.contains("root.rs"));
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

        assert_eq!(output.count, 100);
        assert!(output.truncated);
        assert!(
            output
                .tool_output_for_prompt
                .contains("Results are truncated. Consider using a more specific path or pattern."),
            "expected truncation message, got: {}",
            output.tool_output_for_prompt
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
            output.count >= 2,
            "expected at least 2 files, got {}",
            output.count
        );
        assert!(
            output.tool_output_for_prompt.contains(".hidden.ts"),
            "hidden file should appear in output: {}",
            output.tool_output_for_prompt
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

        assert!(output.entries.iter().any(|entry| entry == "visible.rs"));
        assert!(!output.entries.iter().any(|entry| entry == "secret.rs"));
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
            output.tool_output_for_prompt.contains("visible.txt"),
            "visible.txt should be in output: {}",
            output.tool_output_for_prompt
        );
        assert!(
            output.tool_output_for_prompt.contains("secret.txt"),
            "secret.txt inside ignored_dir/ should be included with --no-ignore: {}",
            output.tool_output_for_prompt
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

        assert_eq!(output.tool_output_for_prompt, "example.rs");
        assert_eq!(output.entries, vec!["example.rs"]);
    }
}
