use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Canonical reasoning effort shared by task, subagent, and sampling layers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    #[default]
    Medium,
    High,
    Xhigh,
}

impl ReasoningEffort {
    /// Source-compatible spelling retained for the former agent config enum.
    #[allow(non_upper_case_globals)]
    pub const XHigh: Self = Self::Xhigh;

    pub const VALID_VALUES: &[&str] = &["none", "minimal", "low", "medium", "high", "xhigh"];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
        }
    }
}

impl std::fmt::Display for ReasoningEffort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ReasoningEffort {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "none" => Ok(Self::None),
            "minimal" => Ok(Self::Minimal),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "xhigh" | "max" => Ok(Self::Xhigh),
            _ => Err(format!(
                "invalid reasoning effort: {value:?} (expected one of: {})",
                Self::VALID_VALUES.join(", ")
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ReasoningEffort;

    #[test]
    fn public_values_and_wire_strings_are_six_values() {
        assert_eq!(
            ReasoningEffort::VALID_VALUES,
            ["none", "minimal", "low", "medium", "high", "xhigh"]
        );
        for value in ReasoningEffort::VALID_VALUES {
            let parsed: ReasoningEffort = value.parse().unwrap();
            assert_eq!(parsed.as_str(), *value);
        }
        assert_eq!(
            "max".parse::<ReasoningEffort>().unwrap(),
            ReasoningEffort::Xhigh
        );
        assert!(serde_json::from_str::<ReasoningEffort>("\"max\"").is_err());
    }
}
