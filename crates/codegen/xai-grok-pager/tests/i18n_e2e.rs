use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const EN_US_SOURCE: &str = include_str!("../i18n/en-US.json");
const JA_JP_SOURCE: &str = include_str!("../i18n/ja-JP.json");

use xai_grok_pager::i18n::{
    Locale, LocaleSource, ResolvedLocale, catalog_keys, localize_clap_command_for_locale,
    normalize_locale, resolve_locale_values,
};

#[test]
fn locale_resolution_obeys_precedence_and_fallback() {
    assert_eq!(normalize_locale("ja_JP.UTF-8"), Some(Locale::JaJp));
    assert_eq!(normalize_locale("en_US@custom"), Some(Locale::EnUs));
    assert_eq!(normalize_locale("fr-FR"), None);

    assert_eq!(
        resolve_locale_values(Some("ja-JP"), Some("en-US"), Some("en-US")),
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
fn catalogs_have_matching_nonempty_keys() {
    let en = catalog_keys(Locale::EnUs);
    let ja = catalog_keys(Locale::JaJp);
    assert_eq!(en, ja);
    assert!(!en.is_empty());
    assert!(en.iter().all(|key| !key.is_empty()));
}

fn placeholders(text: &str) -> BTreeMap<&str, usize> {
    let mut found = BTreeMap::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find('}') else {
            break;
        };
        let name = &rest[..close];
        if !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
            && name
                .as_bytes()
                .first()
                .is_some_and(|byte| *byte == b'_' || byte.is_ascii_alphabetic())
        {
            *found.entry(name).or_insert(0) += 1;
        }
        rest = &rest[close + 1..];
    }
    found
}

#[test]
fn catalog_translations_preserve_placeholder_sets() {
    let en: BTreeMap<String, String> = serde_json::from_str(EN_US_SOURCE).unwrap();
    let ja: BTreeMap<String, String> = serde_json::from_str(JA_JP_SOURCE).unwrap();
    let mismatches: Vec<_> = en
        .iter()
        .filter_map(|(key, english)| {
            let japanese = ja.get(key).expect("catalog key sets match");
            let english_placeholders: BTreeSet<_> = placeholders(english).into_keys().collect();
            let japanese_placeholders: BTreeSet<_> = placeholders(japanese).into_keys().collect();
            (english_placeholders != japanese_placeholders).then_some((
                key,
                english_placeholders,
                japanese_placeholders,
            ))
        })
        .collect();
    assert!(
        mismatches.is_empty(),
        "catalog placeholder-set mismatches:\n{mismatches:#?}"
    );
}

#[test]
fn japanese_catalog_identities_are_explicitly_allowed() {
    const ALLOWED_IDENTITIES: &[&str] = &[
        "======================",
        "{label}: {pct}%",
        "always-approve",
        "worktree",
        "worktree ",
        "yolo",
    ];

    let en: BTreeMap<String, String> = serde_json::from_str(EN_US_SOURCE).unwrap();
    let ja: BTreeMap<String, String> = serde_json::from_str(JA_JP_SOURCE).unwrap();
    let identities: BTreeSet<_> = en
        .iter()
        .filter_map(|(key, english)| (ja.get(key) == Some(english)).then_some(key.as_str()))
        .collect();
    let allowed: BTreeSet<_> = ALLOWED_IDENTITIES.iter().copied().collect();
    assert_eq!(identities, allowed);
}

fn markdown_link_destinations(markdown: &str) -> Vec<String> {
    let link =
        regex::Regex::new(r"(?m)(?P<image>!)?\[[^\]]*\]\((?P<dest><[^>]+>|[^\s\)]+)").unwrap();
    link.captures_iter(markdown)
        .filter(|captures| captures.name("image").is_none())
        .map(|captures| captures["dest"].trim_matches(['<', '>']).to_string())
        .collect()
}

fn markdown_heading_slug(heading: &str) -> String {
    let mut slug = String::new();
    let mut in_code = false;
    for character in heading.trim().chars() {
        match character {
            '`' => in_code = !in_code,
            '<' | '>' if !in_code => {}
            character if character.is_alphanumeric() || character == '_' || character == '-' => {
                slug.extend(character.to_lowercase());
            }
            character if character.is_whitespace() => slug.push('-'),
            _ => {}
        }
    }
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    slug.trim_matches('-').to_string()
}

fn markdown_anchors(markdown: &str) -> BTreeSet<String> {
    let explicit =
        regex::Regex::new(r#"(?i)<a\s+(?:[^>]*?\s)?(?:id|name)=["']([^"']+)["'][^>]*>"#).unwrap();
    let mut anchors: BTreeSet<_> = explicit
        .captures_iter(markdown)
        .map(|captures| captures[1].to_string())
        .collect();
    let mut heading_counts = BTreeMap::<String, usize>::new();
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        let hashes = trimmed.bytes().take_while(|byte| *byte == b'#').count();
        if !(1..=6).contains(&hashes) || trimmed.as_bytes().get(hashes) != Some(&b' ') {
            continue;
        }
        let base = markdown_heading_slug(&trimmed[hashes + 1..]);
        if base.is_empty() {
            continue;
        }
        let count = heading_counts.entry(base.clone()).or_default();
        let slug = if *count == 0 {
            base
        } else {
            format!("{base}-{count}")
        };
        *count += 1;
        anchors.insert(slug);
    }
    anchors
}

fn internal_fragment_target(source: &Path, destination: &str) -> Option<(PathBuf, String)> {
    let (path, fragment) = destination.split_once('#')?;
    if fragment.is_empty()
        || destination.contains("://")
        || destination.starts_with("mailto:")
        || destination.starts_with("data:")
    {
        return None;
    }
    let target = if path.is_empty() {
        source.to_path_buf()
    } else {
        source.parent().unwrap().join(path)
    };
    Some((target, fragment.to_string()))
}

#[test]
fn japanese_user_guides_are_complete_and_link_safe() {
    let guide_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/user-guide");
    let japanese_root = guide_root.join("ja-JP");
    let english_guides: Vec<_> = (1..=22)
        .map(|number| {
            let prefix = format!("{number:02}-");
            std::fs::read_dir(&guide_root)
                .unwrap()
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .find(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".md"))
                })
                .unwrap_or_else(|| panic!("missing English guide with prefix {prefix}"))
        })
        .collect();
    assert_eq!(english_guides.len(), 22);

    let link_mismatches: Vec<_> = english_guides
        .iter()
        .filter_map(|english_path| {
            let japanese_path = japanese_root.join(english_path.file_name().unwrap());
            assert!(
                japanese_path.is_file(),
                "missing Japanese guide {}",
                japanese_path.display()
            );
            let english = std::fs::read_to_string(english_path).unwrap();
            let japanese = std::fs::read_to_string(&japanese_path).unwrap();
            let japanese_chars = japanese
                .chars()
                .filter(|character| {
                    matches!(
                        *character as u32,
                        0x3040..=0x30ff | 0x3400..=0x4dbf | 0x4e00..=0x9fff
                    )
                })
                .count();
            assert!(
                japanese_chars >= 100,
                "Japanese guide lacks substantial Japanese text: {} ({japanese_chars} characters)",
                japanese_path.display()
            );

            let english_links = markdown_link_destinations(&english);
            let japanese_links = markdown_link_destinations(&japanese);
            (english_links != japanese_links).then_some((
                english_path.file_name().unwrap().to_owned(),
                english_links,
                japanese_links,
            ))
        })
        .collect();
    assert!(
        link_mismatches.is_empty(),
        "Japanese guide link destinations differ from English counterparts:\n{link_mismatches:#?}"
    );

    let mut unresolved = Vec::new();
    for japanese_path in english_guides
        .iter()
        .map(|path| japanese_root.join(path.file_name().unwrap()))
    {
        let japanese = std::fs::read_to_string(&japanese_path).unwrap();
        for destination in markdown_link_destinations(&japanese) {
            let Some((target, fragment)) = internal_fragment_target(&japanese_path, &destination)
            else {
                continue;
            };
            match std::fs::read_to_string(&target) {
                Ok(target_markdown) if markdown_anchors(&target_markdown).contains(&fragment) => {}
                Ok(_) => unresolved.push((japanese_path.clone(), destination, target)),
                Err(_) => unresolved.push((japanese_path.clone(), destination, target)),
            }
        }
    }
    assert!(
        unresolved.is_empty(),
        "unresolved internal Japanese guide fragments:\n{unresolved:#?}"
    );
}

fn reject_duplicate_keys(source: &str) -> Result<(), serde_json::Error> {
    use serde::Deserializer as _;
    use serde::de::{Error as _, IgnoredAny, MapAccess, Visitor};

    struct UniqueKeys;

    impl<'de> Visitor<'de> for UniqueKeys {
        type Value = ();

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a JSON object with unique string keys")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut keys = std::collections::HashSet::new();
            while let Some((key, _)) = map.next_entry::<String, IgnoredAny>()? {
                if !keys.insert(key.clone()) {
                    return Err(A::Error::custom(format!("duplicate key {key:?}")));
                }
            }
            Ok(())
        }
    }

    let mut deserializer = serde_json::Deserializer::from_str(source);
    deserializer.deserialize_map(UniqueKeys)?;
    deserializer.end()
}

#[test]
fn catalogs_have_no_duplicate_keys() {
    reject_duplicate_keys(EN_US_SOURCE).expect("en-US catalog keys must be unique");
    reject_duplicate_keys(JA_JP_SOURCE).expect("ja-JP catalog keys must be unique");
    assert!(reject_duplicate_keys(r#"{"same":"first","same":"second"}"#).is_err());
}

#[test]
fn all_clap_help_text_is_cataloged() {
    use clap::CommandFactory as _;

    fn collect(command: &clap::Command, values: &mut std::collections::BTreeSet<String>) {
        for value in [
            command.get_about(),
            command.get_long_about(),
            command.get_before_help(),
            command.get_after_help(),
        ]
        .into_iter()
        .flatten()
        {
            values.insert(value.to_string());
        }
        for arg in command.get_arguments() {
            if let Some(value) = arg.get_help() {
                values.insert(value.to_string());
            }
            if let Some(value) = arg.get_long_help() {
                values.insert(value.to_string());
            }
        }
        for subcommand in command.get_subcommands() {
            collect(subcommand, values);
        }
    }

    let mut values = std::collections::BTreeSet::new();
    collect(&xai_grok_pager::app::PagerArgs::command(), &mut values);
    let catalog = catalog_keys(Locale::EnUs);
    let missing: Vec<_> = values
        .into_iter()
        .filter(|value| !catalog.contains(value.as_str()))
        .collect();
    assert!(missing.is_empty(), "uncataloged clap help:\n{missing:#?}");
}

#[test]
fn pager_args_runtime_parse_renders_localized_help() {
    fn help_for(locale: Locale, args: &[&str]) -> String {
        xai_grok_pager::app::PagerArgs::try_parse_from_for_locale(args, locale)
            .expect_err("--help must return clap's display-help result")
            .to_string()
    }

    let ja_short = help_for(Locale::JaJp, &["grok", "-h"]);
    assert!(ja_short.contains("Grok Build ターミナルUI"));
    assert!(ja_short.contains("使用法: grok"));
    assert!(ja_short.contains("オプション:"));
    assert!(ja_short.contains("コマンド:"));
    assert!(ja_short.contains("--output-format <OUTPUT_FORMAT>"));
    assert!(ja_short.contains("agent"));

    let ja_long = help_for(Locale::JaJp, &["grok", "--help"]);
    assert!(ja_long.contains("対話UIなしでGrokを実行"));
    assert!(ja_long.contains("単一ターンの入力。応答を標準出力へ表示して終了"));

    let en = help_for(Locale::EnUs, &["grok", "--help"]);
    assert!(en.contains("Grok Build TUI"));
    assert!(en.contains("Usage: grok"));
    assert!(en.contains("Options:"));
    assert!(en.contains("Commands:"));
    assert!(!en.contains("ターミナルUI"));
}

#[test]
fn localized_clap_help_preserves_canonical_cli_tokens() {
    use clap::CommandFactory as _;

    fn command_at<'a>(mut command: &'a clap::Command, path: &[&str]) -> &'a clap::Command {
        for name in path {
            command = command
                .get_subcommands()
                .find(|sub| sub.get_name() == *name)
                .unwrap_or_else(|| panic!("missing command {name}"));
        }
        command
    }

    let mut ja = xai_grok_pager::app::PagerArgs::command();
    localize_clap_command_for_locale(&mut ja, Locale::JaJp);
    let help = ja.render_long_help().to_string();
    assert!(help.contains("Grok Build ターミナルUI"));
    assert!(help.contains("対話UIなしでGrokを実行"));
    assert!(help.contains("--output-format <OUTPUT_FORMAT>"));

    let root_names: Vec<_> = ja.get_subcommands().map(|sub| sub.get_name()).collect();
    assert!(root_names.contains(&"agent"));
    assert!(root_names.contains(&"mcp"));
    assert!(root_names.contains(&"plugin"));
    assert!(root_names.contains(&"sessions"));

    let mcp_add = command_at(&ja, &["mcp", "add"]);
    let possible: Vec<_> = mcp_add
        .get_arguments()
        .find(|arg| arg.get_id() == "transport")
        .expect("transport arg")
        .get_possible_values()
        .into_iter()
        .map(|value| value.get_name().to_string())
        .collect();
    assert_eq!(possible, ["stdio", "http", "sse"]);
}

#[test]
fn cjk_text_is_truncated_on_terminal_cell_boundaries() {
    use unicode_width::UnicodeWidthStr;

    let text = "設定画面";
    assert_eq!(text.width(), 8);
    let clipped = xai_grok_pager::render::line_utils::truncate_str(text, 5);
    assert!(clipped.width() <= 5);
    assert!(std::str::from_utf8(clipped.as_bytes()).is_ok());
}
