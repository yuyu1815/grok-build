//! Runtime localization for user-facing pager and CLI text.
//!
//! Canonical protocol/configuration values never pass through this module. Only
//! display strings are translated. The locale is resolved once at process
//! startup; changing `[ui].language` therefore takes effect on the next launch.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{LazyLock, OnceLock};

pub const FALLBACK_LOCALE: &str = "en-US";
pub const JAPANESE_LOCALE: &str = "ja-JP";
pub const GROK_LANG_ENV: &str = "GROK_LANG";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    EnUs,
    JaJp,
}

impl Locale {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EnUs => FALLBACK_LOCALE,
            Self::JaJp => JAPANESE_LOCALE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocaleSource {
    Environment,
    Config,
    Os,
    Fallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedLocale {
    pub locale: Locale,
    pub source: LocaleSource,
}

static LOCALE: OnceLock<ResolvedLocale> = OnceLock::new();

static EN_US: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../i18n/en-US.json"))
        .expect("embedded en-US localization catalog must be valid JSON")
});
static JA_JP: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../i18n/ja-JP.json"))
        .expect("embedded ja-JP localization catalog must be valid JSON")
});

/// Normalize a locale identifier to one of the supported locales.
///
/// POSIX suffixes (`.UTF-8`, `@modifier`) and `_` separators are accepted.
pub fn normalize_locale(raw: &str) -> Option<Locale> {
    let normalized = raw
        .trim()
        .split(['.', '@'])
        .next()
        .unwrap_or_default()
        .replace('_', "-")
        .to_ascii_lowercase();
    match normalized.as_str() {
        "en" | "en-us" => Some(Locale::EnUs),
        "ja" | "ja-jp" => Some(Locale::JaJp),
        _ => None,
    }
}

/// Pure locale resolver used by startup and tests.
///
/// Precedence: `GROK_LANG` > `[ui].language` > OS locale > `en-US`.
pub fn resolve_locale_values(
    grok_lang: Option<&str>,
    configured: Option<&str>,
    os_locale: Option<&str>,
) -> ResolvedLocale {
    for (value, source) in [
        (grok_lang, LocaleSource::Environment),
        (configured, LocaleSource::Config),
        (os_locale, LocaleSource::Os),
    ] {
        if let Some(locale) = value.and_then(normalize_locale) {
            return ResolvedLocale { locale, source };
        }
    }
    ResolvedLocale {
        locale: Locale::EnUs,
        source: LocaleSource::Fallback,
    }
}

/// Resolve and install the process locale. Safe to call more than once; the
/// first result wins so every UI surface uses one coherent language.
pub fn init(configured: Option<&str>) -> ResolvedLocale {
    *LOCALE.get_or_init(|| {
        let env_locale = std::env::var(GROK_LANG_ENV).ok();
        let os_locale = sys_locale::get_locale();
        resolve_locale_values(env_locale.as_deref(), configured, os_locale.as_deref())
    })
}

pub fn current() -> ResolvedLocale {
    LOCALE.get().copied().unwrap_or_else(|| init(None))
}

pub fn locale() -> Locale {
    current().locale
}

pub fn locale_name() -> &'static str {
    locale().as_str()
}

pub fn localized_doc_title(english: &str) -> Cow<'_, str> {
    if locale() == Locale::JaJp {
        let japanese = match english {
            "Getting Started" => "はじめに",
            "Authentication" => "認証",
            "Keyboard Shortcuts" => "キーボードショートカット",
            "Slash Commands" => "スラッシュコマンド",
            "Configuration" => "設定",
            "Theming and Appearance" => "テーマと外観",
            "MCP Servers" => "MCPサーバー",
            "Skills" => "スキル",
            "Plugins and Marketplace" => "プラグインとマーケットプレイス",
            "Hooks" => "フック",
            "Custom Models" => "カスタムモデル",
            "Project Rules (AGENTS.md)" => "プロジェクトルール（AGENTS.md）",
            "Memory" => "メモリー",
            "Headless Mode and Scripting" => "ヘッドレスモードとスクリプト",
            "Agent Mode and IDE Integration" => "エージェントモードとIDE連携",
            "Subagents and Personas" => "サブエージェントとペルソナ",
            "Session Management" => "セッション管理",
            "Sandbox Mode" => "サンドボックスモード",
            "Plan Mode" => "プランモード",
            "Background Tasks and Monitoring" => "バックグラウンドタスクと監視",
            "Terminal Support and Troubleshooting" => "ターミナル対応とトラブルシューティング",
            "Permissions and Safety" => "権限と安全性",
            "Hooks & Plugins Guide" => "フックとプラグイン",
            "Creating Custom Hooks" => "カスタムフックの作成",
            _ => return text(english),
        };
        return Cow::Borrowed(japanese);
    }
    text(english)
}

pub fn localized_doc_description(english: &str) -> Cow<'_, str> {
    if locale() == Locale::JaJp {
        let japanese = match english {
            "Installation, first launch, and basic interaction" => {
                "インストール、初回起動、基本操作"
            }
            "Browser login, API keys, OIDC, external auth providers" => {
                "ブラウザログイン、APIキー、OIDC、外部認証プロバイダー"
            }
            "Complete reference for all TUI key bindings" => {
                "TUIの全キーバインドの完全リファレンス"
            }
            "All / commands for session management, models, memory, hooks" => {
                "セッション管理、モデル、メモリー、フックの全スラッシュコマンド"
            }
            "config.toml, pager.toml, environment variables, file locations" => {
                "config.toml、pager.toml、環境変数、ファイルの場所"
            }
            "Themes, color support, pager.toml customization" => {
                "テーマ、色対応、pager.tomlのカスタマイズ"
            }
            "Setting up external tool integrations via MCP" => "MCPによる外部ツール連携の設定",
            "Creating and using reusable prompt packages" => {
                "再利用可能なプロンプトパッケージの作成と利用"
            }
            "Installing, managing, and creating plugin packages" => {
                "プラグインパッケージのインストール、管理、作成"
            }
            "Project lifecycle scripts for pre/post tool-use events" => {
                "ツール使用前後のプロジェクトライフサイクルスクリプト"
            }
            "BYOK, Ollama, OpenAI-compatible endpoints" => "BYOK、Ollama、OpenAI互換エンドポイント",
            "Per-directory instructions and precedence rules" => {
                "ディレクトリ単位の指示と優先順位ルール"
            }
            "Cross-session knowledge persistence and search" => "セッション間の知識保持と検索",
            "Non-interactive CLI for automation and CI/CD" => "自動化とCI/CD向けの非対話CLI",
            "ACP stdio transport, WebSocket relay, SDK integration" => {
                "ACP stdioトランスポート、WebSocketリレー、SDK連携"
            }
            "Spawning parallel child agents with specialized roles" => {
                "専門ロールを持つ子エージェントの並列起動"
            }
            "Save, load, resume, rewind, and compact sessions" => {
                "セッションの保存、読み込み、再開、巻き戻し、圧縮"
            }
            "OS-level filesystem and network isolation" => {
                "OSレベルのファイルシステムとネットワーク分離"
            }
            "Structured planning with approval dialogs" => {
                "承認ダイアログを使った構造化プランニング"
            }
            "Background commands, /loop, monitor, scheduler" => {
                "バックグラウンドコマンド、/loop、monitor、scheduler"
            }
            "tmux, Byobu, Zellij, SSH, truecolor, clipboard, and diagnostics" => {
                "tmux、Byobu、Zellij、SSH、truecolor、クリップボード、診断"
            }
            "Tool approval, sandbox, security" => "ツール承認、サンドボックス、セキュリティ",
            "Using hooks, plugins, and marketplace" => {
                "フック、プラグイン、マーケットプレイスの利用"
            }
            "Writing your own hooks and matchers" => "独自のフックとマッチャーの作成",
            _ => return text(english),
        };
        return Cow::Borrowed(japanese);
    }
    text(english)
}

pub(crate) fn text_for_locale(locale: Locale, english: &str) -> Cow<'_, str> {
    if locale == Locale::JaJp
        && let Some(value) = JA_JP.get(english)
    {
        return Cow::Owned(value.clone());
    }
    if let Some(value) = EN_US.get(english) {
        return Cow::Owned(value.clone());
    }
    Cow::Borrowed(english)
}

/// Translate a format template and replace `{name}` placeholders with display
/// values. Placeholder names are canonical only inside this presentation
/// helper; protocol and machine-readable values never pass through it.
pub fn format(english: &str, values: &[(&str, String)]) -> String {
    let mut rendered = text(english).into_owned();
    for (name, value) in values {
        rendered = rendered.replace(&format!("{{{name}}}"), value);
    }
    rendered
}

#[macro_export]
macro_rules! tr {
    ($english:literal) => {
        $crate::i18n::text($english)
    };
    ($english:literal, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::format(
            $english,
            &[$((stringify!($name), ($value).to_string())),+],
        )
    };
    ($english:literal, $($name:ident),+ $(,)?) => {
        $crate::i18n::format(
            $english,
            &[$((stringify!($name), $name.to_string())),+],
        )
    };
}

/// Apply the active locale to every clap command/argument description without
/// changing command names, option names, value names, aliases, or values.
pub fn localize_clap_command(command: &mut clap::Command) {
    localize_clap_command_for_locale(command, locale());
}

pub fn localize_clap_command_for_locale(command: &mut clap::Command, locale: Locale) {
    if locale == Locale::JaJp {
        *command = std::mem::take(command).help_template(
            "{before-help}{about-with-newline}\n\
使用法: {usage}\n\
\n\
引数:\n\
{positionals}\n\
\n\
オプション:\n\
{options}\n\
\n\
コマンド:\n\
{subcommands}{after-help}",
        );
    }
    if let Some(about) = command.get_about().map(ToString::to_string) {
        *command = std::mem::take(command).about(text_for_locale(locale, &about).into_owned());
    }
    if let Some(long_about) = command.get_long_about().map(ToString::to_string) {
        *command =
            std::mem::take(command).long_about(text_for_locale(locale, &long_about).into_owned());
    }
    if let Some(before_help) = command.get_before_help().map(ToString::to_string) {
        *command =
            std::mem::take(command).before_help(text_for_locale(locale, &before_help).into_owned());
    }
    if let Some(after_help) = command.get_after_help().map(ToString::to_string) {
        *command =
            std::mem::take(command).after_help(text_for_locale(locale, &after_help).into_owned());
    }
    *command = std::mem::take(command).mut_args(|arg| {
        let help = arg.get_help().map(ToString::to_string);
        let long_help = arg.get_long_help().map(ToString::to_string);
        let mut arg = arg;
        if let Some(help) = help {
            arg = arg.help(text_for_locale(locale, &help).into_owned());
        }
        if let Some(long_help) = long_help {
            arg = arg.long_help(text_for_locale(locale, &long_help).into_owned());
        }
        arg
    });
    for subcommand in command.get_subcommands_mut() {
        localize_clap_command_for_locale(subcommand, locale);
    }
}

/// Translate a display string. Missing Japanese entries intentionally fall
/// back to the embedded English catalog, then to the caller-provided English.
pub fn text(english: &str) -> Cow<'_, str> {
    text_for_locale(locale(), english)
}

/// Static-lifetime adapter for metadata registries built once at startup.
/// Translated strings are intentionally leaked once per process.
pub fn static_text(english: &'static str) -> &'static str {
    match text(english) {
        Cow::Borrowed(value) => value,
        Cow::Owned(value) => Box::leak(value.into_boxed_str()),
    }
}

pub fn catalog_keys(locale: Locale) -> std::collections::HashSet<&'static str> {
    match locale {
        Locale::EnUs => EN_US.keys().map(String::as_str).collect(),
        Locale::JaJp => JA_JP.keys().map(String::as_str).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_precedence_and_fallback() {
        assert_eq!(
            resolve_locale_values(Some("ja_JP.UTF-8"), Some("en-US"), Some("en-US")),
            ResolvedLocale {
                locale: Locale::JaJp,
                source: LocaleSource::Environment,
            }
        );
        assert_eq!(
            resolve_locale_values(Some("unsupported"), Some("ja-JP"), Some("en-US")),
            ResolvedLocale {
                locale: Locale::JaJp,
                source: LocaleSource::Config,
            }
        );
        assert_eq!(
            resolve_locale_values(None, None, Some("ja")),
            ResolvedLocale {
                locale: Locale::JaJp,
                source: LocaleSource::Os,
            }
        );
        assert_eq!(
            resolve_locale_values(None, None, Some("fr-FR")),
            ResolvedLocale {
                locale: Locale::EnUs,
                source: LocaleSource::Fallback,
            }
        );
    }

    #[test]
    fn catalogs_have_identical_nonempty_keys() {
        assert_eq!(catalog_keys(Locale::EnUs), catalog_keys(Locale::JaJp));
        assert!(
            EN_US
                .iter()
                .all(|(key, value)| !key.is_empty() && !value.is_empty())
        );
        assert!(
            JA_JP
                .iter()
                .all(|(key, value)| !key.is_empty() && !value.is_empty())
        );
    }

    #[test]
    fn japanese_translation_and_english_fallback_work() {
        assert_eq!(text_for_locale(Locale::JaJp, "Settings"), "設定");
        assert_eq!(
            text_for_locale(Locale::JaJp, "unregistered fallback text"),
            "unregistered fallback text"
        );
    }

    #[test]
    fn catalogs_do_not_contain_mojibake_keys_or_values() {
        let mojibake = ['窶', '繧', '竊', '檀', '繝'];
        for (locale, catalog) in [("en-US", &*EN_US), ("ja-JP", &*JA_JP)] {
            let broken: Vec<_> = catalog
                .iter()
                .filter(|(key, value)| {
                    key.chars()
                        .chain(value.chars())
                        .any(|ch| mojibake.contains(&ch))
                })
                .collect();
            assert!(
                broken.is_empty(),
                "mojibake in {locale} catalog:\n{broken:#?}"
            );
        }
    }

    #[test]
    fn cjk_translation_uses_terminal_cell_width() {
        use unicode_width::UnicodeWidthStr;
        let translated = text_for_locale(Locale::JaJp, "Settings");
        assert_eq!(translated.width(), 4, "設定 is two double-width CJK glyphs");
        let clipped = crate::render::line_utils::truncate_str(&translated, 3);
        assert!(clipped.width() <= 3);
    }
}
