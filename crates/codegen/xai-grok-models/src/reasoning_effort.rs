use xai_grok_sampling_types::{ReasoningEffort, ReasoningEffortOption};

/// Model-specific reasoning metadata published by OpenAI.
///
/// IDs are deliberately enumerated. Matching is case-sensitive and accepts only
/// the exact ID or the final `/`-delimited segment of a provider-qualified ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReasoningEffortPolicy {
    /// Effort levels offered by the model, in display order.
    pub allowed: &'static [ReasoningEffort],
    /// Effort selected when no caller override is present.
    pub default: ReasoningEffort,
}

impl ReasoningEffortPolicy {
    /// Convert the policy into canonical selectable options.
    pub fn options(self) -> Vec<ReasoningEffortOption> {
        self.allowed
            .iter()
            .copied()
            .map(|value| ReasoningEffortOption::canonical(value, value == self.default))
            .collect()
    }
}

const GPT_5_5: ReasoningEffortPolicy = ReasoningEffortPolicy {
    allowed: &[
        ReasoningEffort::None,
        ReasoningEffort::Low,
        ReasoningEffort::Medium,
        ReasoningEffort::High,
        ReasoningEffort::Xhigh,
    ],
    default: ReasoningEffort::Medium,
};

const GPT_5_5_PRO: ReasoningEffortPolicy = ReasoningEffortPolicy {
    allowed: &[
        ReasoningEffort::Medium,
        ReasoningEffort::High,
        ReasoningEffort::Xhigh,
    ],
    default: ReasoningEffort::High,
};

const GPT_5_6: ReasoningEffortPolicy = ReasoningEffortPolicy {
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

/// Return the built-in reasoning policy for an explicitly supported model.
pub fn reasoning_effort_policy_for_model(model: &str) -> Option<ReasoningEffortPolicy> {
    let exact_id = model
        .rsplit('/')
        .next()
        .expect("rsplit always yields a segment");
    match exact_id {
        "gpt-5.5" | "gpt-5.5-2026-04-23" => Some(GPT_5_5),
        "gpt-5.5-pro" | "gpt-5.5-pro-2026-04-23" => Some(GPT_5_5_PRO),
        "gpt-5.6" | "gpt-5.6-sol" | "gpt-5.6-terra" | "gpt-5.6-luna" => Some(GPT_5_6),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_is_exact_or_final_provider_segment_only() {
        for model in ["gpt-5.5", "gpt-5.5-2026-04-23"] {
            assert_eq!(reasoning_effort_policy_for_model(model), Some(GPT_5_5));
        }
        for model in ["gpt-5.5-pro", "gpt-5.5-pro-2026-04-23"] {
            assert_eq!(reasoning_effort_policy_for_model(model), Some(GPT_5_5_PRO));
        }
        for model in ["gpt-5.6", "gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna"] {
            assert_eq!(reasoning_effort_policy_for_model(model), Some(GPT_5_6));
        }
        assert_eq!(
            reasoning_effort_policy_for_model("anything/gpt-5.5"),
            Some(GPT_5_5)
        );
        assert_eq!(
            reasoning_effort_policy_for_model("one/two/gpt-5.5"),
            Some(GPT_5_5)
        );

        for rejected in [
            "GPT-5.5",
            "gpt-5.5-latest",
            "gpt-5.5-2026-04-24",
            "prefix-gpt-5.5",
            "gpt-5.5/suffix",
        ] {
            assert_eq!(
                reasoning_effort_policy_for_model(rejected),
                None,
                "unexpected match: {rejected}"
            );
        }
    }

    #[test]
    fn policies_preserve_options_and_mark_one_default() {
        for policy in [GPT_5_5, GPT_5_5_PRO, GPT_5_6] {
            let options = policy.options();
            assert_eq!(
                options
                    .iter()
                    .map(|option| option.value)
                    .collect::<Vec<_>>(),
                policy.allowed
            );
            assert_eq!(options.iter().filter(|option| option.default).count(), 1);
            assert_eq!(
                options
                    .iter()
                    .find(|option| option.default)
                    .map(|option| option.value),
                Some(policy.default)
            );
        }
    }
}
