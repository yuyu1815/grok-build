//! Provider identity and registry boundary.
//!
//! Lifecycle orchestration is layered on top in the next stacked change. This
//! module keeps model credential routing independent from Grok login state.

/// Internal canonical provider identifier.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum ProviderId {
    #[serde(rename = "xai")]
    Xai,
    /// Custom or otherwise unclassified models must not inherit xAI secrets.
    #[default]
    #[serde(rename = "unknown")]
    Unknown,
}

impl ProviderId {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Xai => "Grok",
            Self::Unknown => "Custom",
        }
    }

    /// Normalize the explicitly supported user-facing provider name.
    pub fn from_ui_name(name: &str) -> Option<Self> {
        name.eq_ignore_ascii_case("grok").then_some(Self::Xai)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderDescriptor {
    pub id: ProviderId,
    pub display_name: &'static str,
}

/// Fixed registry for providers with first-class UI lifecycle support.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProviderRegistry;

impl ProviderRegistry {
    pub const fn new() -> Self {
        Self
    }

    pub const fn providers(self) -> &'static [ProviderDescriptor] {
        &[ProviderDescriptor {
            id: ProviderId::Xai,
            display_name: "Grok",
        }]
    }

    pub fn resolve_ui_name(self, name: &str) -> Option<ProviderId> {
        ProviderId::from_ui_name(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_only_grok() {
        assert_eq!(ProviderId::from_ui_name("grok"), Some(ProviderId::Xai));
        assert_eq!(ProviderId::from_ui_name("GROK"), Some(ProviderId::Xai));
        assert_eq!(ProviderId::from_ui_name("xai"), None);
        assert_eq!(ProviderId::from_ui_name("codex"), None);
    }

    #[test]
    fn registry_is_grok_only_and_keeps_internal_id_separate() {
        let providers = ProviderRegistry::new().providers();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, ProviderId::Xai);
        assert_eq!(providers[0].display_name, "Grok");
        assert_eq!(ProviderId::Unknown.display_name(), "Custom");
    }

    #[test]
    fn unknown_is_the_safe_default() {
        assert_eq!(ProviderId::default(), ProviderId::Unknown);
    }
}
