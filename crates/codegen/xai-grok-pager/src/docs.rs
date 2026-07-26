//! In-app how-to documentation data (embedded markdown).
//!
//! Locale-specific user-guide tables embed every guide explicitly. Reference
//! docs remain a separate canonical-English table. `DocEntry` exists only for
//! backward compatibility with the TUI doc picker.

/// A compile-time document entry. All fields are `&'static str`.
#[derive(Debug)]
pub struct Doc {
    pub filename: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub content: &'static str,
}

/// Owned variant for the TUI doc picker (backward compat).
#[derive(Debug, Clone)]
pub struct DocEntry {
    pub title: String,
    pub description: String,
    /// Embedded markdown content.
    pub content: &'static str,
}

impl From<&Doc> for DocEntry {
    fn from(d: &Doc) -> Self {
        Self {
            title: d.title.into(),
            description: d.description.into(),
            content: d.content,
        }
    }
}

// ── Static doc tables ────────────────────────────────────────────────────────

macro_rules! guide {
    ($file:literal, $title:literal, $desc:literal, $content:expr) => {
        Doc {
            filename: $file,
            title: $title,
            description: $desc,
            content: $content,
        }
    };
}

pub static USER_GUIDE_EN_US: &[Doc] = &[
    guide!(
        "01-getting-started.md",
        "Getting Started",
        "Installation, first launch, and basic interaction",
        include_str!("../docs/user-guide/01-getting-started.md")
    ),
    guide!(
        "02-authentication.md",
        "Authentication",
        "Browser login, API keys, OIDC, external auth providers",
        include_str!("../docs/user-guide/02-authentication.md")
    ),
    guide!(
        "03-keyboard-shortcuts.md",
        "Keyboard Shortcuts",
        "Complete reference for all TUI key bindings",
        include_str!("../docs/user-guide/03-keyboard-shortcuts.md")
    ),
    guide!(
        "04-slash-commands.md",
        "Slash Commands",
        "All / commands for session management, models, memory, hooks",
        include_str!("../docs/user-guide/04-slash-commands.md")
    ),
    guide!(
        "05-configuration.md",
        "Configuration",
        "config.toml, pager.toml, environment variables, file locations",
        include_str!("../docs/user-guide/05-configuration.md")
    ),
    guide!(
        "06-theming.md",
        "Theming and Appearance",
        "Themes, color support, pager.toml customization",
        include_str!("../docs/user-guide/06-theming.md")
    ),
    guide!(
        "07-mcp-servers.md",
        "MCP Servers",
        "Setting up external tool integrations via MCP",
        include_str!("../docs/user-guide/07-mcp-servers.md")
    ),
    guide!(
        "08-skills.md",
        "Skills",
        "Creating and using reusable prompt packages",
        include_str!("../docs/user-guide/08-skills.md")
    ),
    guide!(
        "09-plugins.md",
        "Plugins and Marketplace",
        "Installing, managing, and creating plugin packages",
        include_str!("../docs/user-guide/09-plugins.md")
    ),
    guide!(
        "10-hooks.md",
        "Hooks",
        "Project lifecycle scripts for pre/post tool-use events",
        include_str!("../docs/user-guide/10-hooks.md")
    ),
    guide!(
        "11-custom-models.md",
        "Custom Models",
        "BYOK, Ollama, OpenAI-compatible endpoints",
        include_str!("../docs/user-guide/11-custom-models.md")
    ),
    guide!(
        "12-project-rules.md",
        "Project Rules (AGENTS.md)",
        "Per-directory instructions and precedence rules",
        include_str!("../docs/user-guide/12-project-rules.md")
    ),
    guide!(
        "13-memory.md",
        "Memory",
        "Cross-session knowledge persistence and search",
        include_str!("../docs/user-guide/13-memory.md")
    ),
    guide!(
        "14-headless-mode.md",
        "Headless Mode and Scripting",
        "Non-interactive CLI for automation and CI/CD",
        include_str!("../docs/user-guide/14-headless-mode.md")
    ),
    guide!(
        "15-agent-mode.md",
        "Agent Mode and IDE Integration",
        "ACP stdio transport, WebSocket relay, SDK integration",
        include_str!("../docs/user-guide/15-agent-mode.md")
    ),
    guide!(
        "16-subagents.md",
        "Subagents and Personas",
        "Spawning parallel child agents with specialized roles",
        include_str!("../docs/user-guide/16-subagents.md")
    ),
    guide!(
        "17-sessions.md",
        "Session Management",
        "Save, load, resume, rewind, and compact sessions",
        include_str!("../docs/user-guide/17-sessions.md")
    ),
    guide!(
        "18-sandbox.md",
        "Sandbox Mode",
        "OS-level filesystem and network isolation",
        include_str!("../docs/user-guide/18-sandbox.md")
    ),
    guide!(
        "19-plan-mode.md",
        "Plan Mode",
        "Structured planning with approval dialogs",
        include_str!("../docs/user-guide/19-plan-mode.md")
    ),
    guide!(
        "20-background-tasks.md",
        "Background Tasks and Monitoring",
        "Background commands, /loop, monitor, scheduler",
        include_str!("../docs/user-guide/20-background-tasks.md")
    ),
    guide!(
        "21-terminal-support.md",
        "Terminal Support and Troubleshooting",
        "tmux, Byobu, Zellij, SSH, truecolor, clipboard, and diagnostics",
        include_str!("../docs/user-guide/21-terminal-support.md")
    ),
    guide!(
        "22-permissions-and-safety.md",
        "Permissions and Safety",
        "Tool approval, sandbox, security",
        include_str!("../docs/user-guide/22-permissions-and-safety.md")
    ),
];

pub static USER_GUIDE_JA_JP: &[Doc] = &[
    guide!(
        "01-getting-started.md",
        "Getting Started",
        "Installation, first launch, and basic interaction",
        include_str!("../docs/user-guide/ja-JP/01-getting-started.md")
    ),
    guide!(
        "02-authentication.md",
        "Authentication",
        "Browser login, API keys, OIDC, external auth providers",
        include_str!("../docs/user-guide/ja-JP/02-authentication.md")
    ),
    guide!(
        "03-keyboard-shortcuts.md",
        "Keyboard Shortcuts",
        "Complete reference for all TUI key bindings",
        include_str!("../docs/user-guide/ja-JP/03-keyboard-shortcuts.md")
    ),
    guide!(
        "04-slash-commands.md",
        "Slash Commands",
        "All / commands for session management, models, memory, hooks",
        include_str!("../docs/user-guide/ja-JP/04-slash-commands.md")
    ),
    guide!(
        "05-configuration.md",
        "Configuration",
        "config.toml, pager.toml, environment variables, file locations",
        include_str!("../docs/user-guide/ja-JP/05-configuration.md")
    ),
    guide!(
        "06-theming.md",
        "Theming and Appearance",
        "Themes, color support, pager.toml customization",
        include_str!("../docs/user-guide/ja-JP/06-theming.md")
    ),
    guide!(
        "07-mcp-servers.md",
        "MCP Servers",
        "Setting up external tool integrations via MCP",
        include_str!("../docs/user-guide/ja-JP/07-mcp-servers.md")
    ),
    guide!(
        "08-skills.md",
        "Skills",
        "Creating and using reusable prompt packages",
        include_str!("../docs/user-guide/ja-JP/08-skills.md")
    ),
    guide!(
        "09-plugins.md",
        "Plugins and Marketplace",
        "Installing, managing, and creating plugin packages",
        include_str!("../docs/user-guide/ja-JP/09-plugins.md")
    ),
    guide!(
        "10-hooks.md",
        "Hooks",
        "Project lifecycle scripts for pre/post tool-use events",
        include_str!("../docs/user-guide/ja-JP/10-hooks.md")
    ),
    guide!(
        "11-custom-models.md",
        "Custom Models",
        "BYOK, Ollama, OpenAI-compatible endpoints",
        include_str!("../docs/user-guide/ja-JP/11-custom-models.md")
    ),
    guide!(
        "12-project-rules.md",
        "Project Rules (AGENTS.md)",
        "Per-directory instructions and precedence rules",
        include_str!("../docs/user-guide/ja-JP/12-project-rules.md")
    ),
    guide!(
        "13-memory.md",
        "Memory",
        "Cross-session knowledge persistence and search",
        include_str!("../docs/user-guide/ja-JP/13-memory.md")
    ),
    guide!(
        "14-headless-mode.md",
        "Headless Mode and Scripting",
        "Non-interactive CLI for automation and CI/CD",
        include_str!("../docs/user-guide/ja-JP/14-headless-mode.md")
    ),
    guide!(
        "15-agent-mode.md",
        "Agent Mode and IDE Integration",
        "ACP stdio transport, WebSocket relay, SDK integration",
        include_str!("../docs/user-guide/ja-JP/15-agent-mode.md")
    ),
    guide!(
        "16-subagents.md",
        "Subagents and Personas",
        "Spawning parallel child agents with specialized roles",
        include_str!("../docs/user-guide/ja-JP/16-subagents.md")
    ),
    guide!(
        "17-sessions.md",
        "Session Management",
        "Save, load, resume, rewind, and compact sessions",
        include_str!("../docs/user-guide/ja-JP/17-sessions.md")
    ),
    guide!(
        "18-sandbox.md",
        "Sandbox Mode",
        "OS-level filesystem and network isolation",
        include_str!("../docs/user-guide/ja-JP/18-sandbox.md")
    ),
    guide!(
        "19-plan-mode.md",
        "Plan Mode",
        "Structured planning with approval dialogs",
        include_str!("../docs/user-guide/ja-JP/19-plan-mode.md")
    ),
    guide!(
        "20-background-tasks.md",
        "Background Tasks and Monitoring",
        "Background commands, /loop, monitor, scheduler",
        include_str!("../docs/user-guide/ja-JP/20-background-tasks.md")
    ),
    guide!(
        "21-terminal-support.md",
        "Terminal Support and Troubleshooting",
        "tmux, Byobu, Zellij, SSH, truecolor, clipboard, and diagnostics",
        include_str!("../docs/user-guide/ja-JP/21-terminal-support.md")
    ),
    guide!(
        "22-permissions-and-safety.md",
        "Permissions and Safety",
        "Tool approval, sandbox, security",
        include_str!("../docs/user-guide/ja-JP/22-permissions-and-safety.md")
    ),
];

/// Backward-compatible canonical English table.
pub static USER_GUIDE: &[Doc] = USER_GUIDE_EN_US;

/// Non-user-guide reference docs. Separate from USER_GUIDE because they
/// live under `docs/` (not `docs/user-guide/`), are not extracted to disk,
/// and do not follow the NN-*.md managed naming pattern. Bundled via
/// `include_str!` so they are available at runtime without a docs path.
static REFERENCE_DOCS: &[Doc] = &[
    Doc {
        filename: "hooks-and-plugins.md",
        title: "Hooks & Plugins Guide",
        description: "Using hooks, plugins, and marketplace",
        content: include_str!("../docs/hooks-and-plugins.md"),
    },
    Doc {
        filename: "custom-hooks.md",
        title: "Creating Custom Hooks",
        description: "Writing your own hooks and matchers",
        content: include_str!("../docs/custom-hooks.md"),
    },
];

// ── Public API ───────────────────────────────────────────────────────────────

pub fn user_guide_for(locale: crate::i18n::Locale) -> &'static [Doc] {
    match locale {
        crate::i18n::Locale::EnUs => USER_GUIDE_EN_US,
        crate::i18n::Locale::JaJp => USER_GUIDE_JA_JP,
    }
}

fn active_user_guide() -> &'static [Doc] {
    user_guide_for(crate::i18n::locale())
}

/// Find a doc by canonical English title (case-insensitive).
pub fn find_doc(title: &str) -> Option<&'static Doc> {
    if let Some(index) = USER_GUIDE_EN_US
        .iter()
        .position(|doc| doc.title.eq_ignore_ascii_case(title))
    {
        return active_user_guide().get(index);
    }
    REFERENCE_DOCS
        .iter()
        .find(|doc| doc.title.eq_ignore_ascii_case(title))
}

/// All canonical English docs, including reference docs, zero allocation.
pub fn all_docs() -> impl Iterator<Item = &'static Doc> {
    USER_GUIDE_EN_US.iter().chain(REFERENCE_DOCS.iter())
}

/// All canonical English doc titles, zero allocation.
pub fn all_titles() -> impl Iterator<Item = &'static str> {
    all_docs().map(|doc| doc.title)
}

/// Returns the content of a how-to document by canonical English title.
pub fn get_howto_doc(title: &str) -> Option<&'static str> {
    find_doc(title).map(|doc| doc.content)
}

/// Returns a list of available canonical English titles for model lookup.
pub fn list_howto_titles() -> Vec<String> {
    all_titles().map(String::from).collect()
}

/// Returns all docs as owned `DocEntry` values for the TUI doc picker.
pub fn default_howto_entries() -> Vec<DocEntry> {
    active_user_guide()
        .iter()
        .map(|doc| DocEntry {
            title: crate::i18n::localized_doc_title(doc.title).into_owned(),
            description: crate::i18n::localized_doc_description(doc.description).into_owned(),
            content: doc.content,
        })
        .chain(REFERENCE_DOCS.iter().map(|doc| DocEntry {
            title: crate::i18n::localized_doc_title(doc.title).into_owned(),
            description: crate::i18n::localized_doc_description(doc.description).into_owned(),
            content: doc.content,
        }))
        .collect()
}

/// Extract user-guide docs to `<grok_home>/docs/user-guide/`.
///
/// Called from the pager binary startup so the model can read them from disk.
pub fn extract_user_guide_docs(grok_home: &std::path::Path) {
    extract_user_guide_docs_for_locale(grok_home, crate::i18n::locale());
}

fn extract_user_guide_docs_for_locale(grok_home: &std::path::Path, locale: crate::i18n::Locale) {
    let docs_dir = grok_home.join("docs").join("user-guide");
    if let Err(e) = std::fs::create_dir_all(&docs_dir) {
        tracing::warn!(error = %e, "Failed to create user-guide docs directory");
        return;
    }
    let guide = user_guide_for(locale);
    for doc in guide {
        if let Err(e) = std::fs::write(docs_dir.join(doc.filename), doc.content) {
            tracing::debug!(error = %e, filename = doc.filename, "Failed to extract user-guide doc");
        }
    }
    // Clean up stale managed docs (files removed from the active guide since last run).
    // Only remove files matching the managed naming pattern (NN-*.md).
    if let Ok(entries) = std::fs::read_dir(&docs_dir) {
        let valid: std::collections::HashSet<&str> = guide.iter().map(|d| d.filename).collect();
        for dir_entry in entries.flatten() {
            if let Some(name) = dir_entry.file_name().to_str() {
                let is_managed = name.len() > 3
                    && name.as_bytes()[0].is_ascii_digit()
                    && name.as_bytes()[1].is_ascii_digit()
                    && name.as_bytes()[2] == b'-'
                    && name.ends_with(".md");
                if is_managed
                    && !valid.contains(name)
                    && let Err(e) = std::fs::remove_file(dir_entry.path())
                {
                    tracing::debug!(error = %e, filename = name, "Failed to remove stale user-guide doc");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_guide_entries_are_valid() {
        for doc in USER_GUIDE {
            assert!(!doc.content.is_empty(), "Doc {} is empty", doc.filename);
            assert!(
                !doc.title.is_empty(),
                "Doc {} has empty title",
                doc.filename
            );
            assert!(
                !doc.description.is_empty(),
                "Doc {} has empty description",
                doc.filename
            );
            assert!(
                doc.content.starts_with('#'),
                "Doc {} should start with a markdown header",
                doc.filename
            );
        }
    }

    #[test]
    fn user_guide_entries_have_no_duplicates() {
        let mut seen = std::collections::HashSet::new();
        for doc in USER_GUIDE {
            assert!(
                seen.insert(doc.filename),
                "Duplicate doc in list: {}",
                doc.filename
            );
        }
    }

    #[test]
    fn default_howto_entries_includes_all_user_guide_docs() {
        let entries = default_howto_entries();
        assert_eq!(entries.len(), USER_GUIDE.len() + REFERENCE_DOCS.len());
        for (i, doc) in USER_GUIDE.iter().enumerate() {
            assert_eq!(entries[i].title, doc.title, "Entry {} title mismatch", i);
        }
    }

    #[test]
    fn find_doc_is_case_insensitive() {
        let doc = find_doc("getting started").expect("should find Getting Started");
        assert_eq!(doc.title, "Getting Started");
        assert!(find_doc("nonexistent guide").is_none());
    }

    #[test]
    fn all_titles_covers_both_tables() {
        let titles: Vec<_> = all_titles().collect();
        assert_eq!(titles.len(), USER_GUIDE.len() + REFERENCE_DOCS.len());
    }

    #[test]
    fn get_howto_doc_delegates_to_find_doc() {
        assert!(get_howto_doc("Getting Started").is_some());
        assert!(get_howto_doc("Hooks & Plugins Guide").is_some());
        assert!(get_howto_doc("no such doc").is_none());
    }

    #[test]
    fn list_howto_titles_returns_all() {
        let titles = list_howto_titles();
        assert_eq!(titles.len(), USER_GUIDE.len() + REFERENCE_DOCS.len());
    }

    #[test]
    fn model_readable_user_guide_path_tracks_locale_and_preserves_user_files() {
        let tmp = tempfile::tempdir().unwrap();
        // Startup exposes this exact path to the model; locale switches replace
        // its managed NN-*.md files in place rather than changing the path.
        let docs_dir = tmp.path().join("docs").join("user-guide");
        let representative = docs_dir.join("01-getting-started.md");
        let stale_managed = docs_dir.join("99-removed.md");
        let user_file = docs_dir.join("notes.md");

        std::fs::create_dir_all(&docs_dir).unwrap();
        std::fs::write(&stale_managed, "stale").unwrap();
        std::fs::write(&user_file, "user notes").unwrap();

        extract_user_guide_docs_for_locale(tmp.path(), crate::i18n::Locale::EnUs);
        assert_eq!(
            std::fs::read_to_string(&representative).unwrap(),
            USER_GUIDE_EN_US[0].content
        );
        assert!(!stale_managed.exists(), "stale managed doc must be removed");
        assert_eq!(
            std::fs::read_to_string(&user_file).unwrap(),
            "user notes",
            "non-managed files must be preserved"
        );

        extract_user_guide_docs_for_locale(tmp.path(), crate::i18n::Locale::JaJp);
        assert_eq!(
            std::fs::read_to_string(&representative).unwrap(),
            USER_GUIDE_JA_JP[0].content
        );
        assert_eq!(
            managed_filenames(&docs_dir),
            guide_filenames(USER_GUIDE_JA_JP),
            "Japanese extraction must expose exactly the active managed manifest"
        );

        extract_user_guide_docs_for_locale(tmp.path(), crate::i18n::Locale::EnUs);
        assert_eq!(
            std::fs::read_to_string(&representative).unwrap(),
            USER_GUIDE_EN_US[0].content
        );
        assert_eq!(
            managed_filenames(&docs_dir),
            guide_filenames(USER_GUIDE_EN_US)
        );
        assert_eq!(std::fs::read_to_string(user_file).unwrap(), "user notes");
    }

    fn guide_filenames(guide: &[Doc]) -> std::collections::BTreeSet<String> {
        guide.iter().map(|doc| doc.filename.to_owned()).collect()
    }

    fn managed_filenames(docs_dir: &std::path::Path) -> std::collections::BTreeSet<String> {
        std::fs::read_dir(docs_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| {
                name.len() > 3
                    && name.as_bytes()[0].is_ascii_digit()
                    && name.as_bytes()[1].is_ascii_digit()
                    && name.as_bytes()[2] == b'-'
                    && name.ends_with(".md")
            })
            .collect()
    }
}
