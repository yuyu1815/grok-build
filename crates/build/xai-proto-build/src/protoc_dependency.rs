use anyhow::Context;
use std::process::{Command, Output};

pub(crate) struct DependencyOutput {
    #[cfg(windows)]
    _temp_dir: tempfile::TempDir,
    #[cfg(windows)]
    path: std::path::PathBuf,
}

#[cfg(unix)]
pub(crate) fn configure(command: &mut Command) -> anyhow::Result<DependencyOutput> {
    command
        .arg("--dependency_out=/dev/stdout")
        .arg("--descriptor_set_out=/dev/null");
    Ok(DependencyOutput {})
}

#[cfg(windows)]
pub(crate) fn configure(command: &mut Command) -> anyhow::Result<DependencyOutput> {
    let temp_dir = tempfile::tempdir()?;
    let path = temp_dir.path().join("protoc-dependencies.d");
    command
        .arg(format!("--dependency_out={}", path.display()))
        .arg("--descriptor_set_out=NUL");
    Ok(DependencyOutput {
        _temp_dir: temp_dir,
        path,
    })
}

#[cfg(unix)]
pub(crate) fn read(
    _dependency_output: &DependencyOutput,
    output: Output,
) -> anyhow::Result<String> {
    String::from_utf8(output.stdout).context("protoc command output not UTF-8")
}

#[cfg(windows)]
pub(crate) fn read(
    dependency_output: &DependencyOutput,
    _output: Output,
) -> anyhow::Result<String> {
    std::fs::read_to_string(&dependency_output.path)
        .context("failed to read protoc dependency file")
}

#[cfg(unix)]
pub(crate) fn prefix() -> &'static str {
    "/dev/null:"
}

#[cfg(windows)]
pub(crate) fn prefix() -> &'static str {
    "NUL:"
}
