use xai_grok_sampling_types::{ReasoningEffort, ReasoningEffortOption};

/// Model-specific reasoning metadata published by OpenAI.
///
/// IDs are deliberately enumerated. Matching is case-sensitive and accepts only
/// the exact ID or the final `/`-delimited segment of a provider-prefixed ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OpenAiReasoningEffortPolicy {
    pub(crate) allowed: &'static [ReasoningEffort],
    pub(crate) default: ReasoningEffort,
}

impl OpenAiReasoningEffortPolicy {
    pub(crate) fn options(self) -> Vec<ReasoningEffortOption> {
        self.allowed
            .iter()
            .copied()
            .map(|value| ReasoningEffortOption {
                id: value.as_str().to_owned(),
                value,
                label: effort_label(value).to_owned(),
                description: None,
                default: value == self.default,
            })
            .collect()
    }
}

const GPT_5_5: OpenAiReasoningEffortPolicy = OpenAiReasoningEffortPolicy {
    allowed: &[
        ReasoningEffort::None,
        ReasoningEffort::Low,
        ReasoningEffort::Medium,
        ReasoningEffort::High,
        ReasoningEffort::Xhigh,
    ],
    default: ReasoningEffort::Medium,
};

const GPT_5_5_PRO: OpenAiReasoningEffortPolicy = OpenAiReasoningEffortPolicy {
    allowed: &[
        ReasoningEffort::Medium,
        ReasoningEffort::High,
        ReasoningEffort::Xhigh,
    ],
    default: ReasoningEffort::High,
};

const GPT_5_6: OpenAiReasoningEffortPolicy = OpenAiReasoningEffortPolicy {
    allowed: &[
        ReasoningEffort::None,
        ReasoningEffort::Low,
        ReasoningEffort::Medium,
        ReasoningEffort::High,
        ReasoningEffort::Xhigh,
        ReasoningEffort::Max,
    ],
    default: ReasoningEffort::Medium,
};

pub(crate) fn policy_for_model(model: &str) -> Option<OpenAiReasoningEffortPolicy> {
    let exact_id = model.rsplit('/').next()?;
    match exact_id {
        "gpt-5.5" | "gpt-5.5-2026-04-23" => Some(GPT_5_5),
        "gpt-5.5-pro" | "gpt-5.5-pro-2026-04-23" => Some(GPT_5_5_PRO),
        "gpt-5.6" | "gpt-5.6-sol" | "gpt-5.6-terra" | "gpt-5.6-luna" => Some(GPT_5_6),
        _ => None,
    }
}

fn effort_label(effort: ReasoningEffort) -> &'static str {
    match effort {
        ReasoningEffort::None => "None",
        ReasoningEffort::Minimal => "Minimal",
        ReasoningEffort::Low => "Low",
        ReasoningEffort::Medium => "Medium",
        ReasoningEffort::High => "High",
        ReasoningEffort::Xhigh => "Xhigh",
        ReasoningEffort::Max => "Max",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_is_exact_or_final_provider_segment_only() {
        for model in ["gpt-5.5", "gpt-5.5-2026-04-23"] {
            assert_eq!(policy_for_model(model), Some(GPT_5_5));
        }
        for model in ["gpt-5.5-pro", "gpt-5.5-pro-2026-04-23"] {
            assert_eq!(policy_for_model(model), Some(GPT_5_5_PRO));
        }
        for model in ["gpt-5.6", "gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna"] {
            assert_eq!(policy_for_model(model), Some(GPT_5_6));
        }
        assert_eq!(policy_for_model("anything/gpt-5.5"), Some(GPT_5_5));
        assert_eq!(policy_for_model("one/two/gpt-5.5"), Some(GPT_5_5));

        for rejected in [
            "GPT-5.5",
            "gpt-5.5-latest",
            "gpt-5.5-2026-04-24",
            "prefix-gpt-5.5",
            "gpt-5.5/suffix",
        ] {
            assert_eq!(
                policy_for_model(rejected),
                None,
                "unexpected match: {rejected}"
            );
        }
    }
}
