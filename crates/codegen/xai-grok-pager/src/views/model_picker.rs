//! `/model` picker state: model navigation and one picker-wide effort value.

use agent_client_protocol as acp;
use xai_grok_shell::sampling::types::{ReasoningEffort, ReasoningEffortOption};

use crate::acp::model_state::ModelState;
use crate::views::picker::PickerState;

pub const MAX_VISIBLE_MODELS: u16 = 10;

#[derive(Debug, Clone)]
pub struct ModelPickerEntry {
    pub id: acp::ModelId,
    pub label: String,
    pub description: String,
    pub efforts: Vec<ReasoningEffortOption>,
}

#[derive(Debug, Clone)]
pub struct ModelPickerState {
    pub picker: PickerState,
    pub entries: Vec<ModelPickerEntry>,
    pub effort: Option<ReasoningEffort>,
    pub original_label: String,
    effort_value_defined: bool,
    effort_toggled: bool,
}

impl ModelPickerState {
    pub fn new(models: &ModelState) -> Option<Self> {
        let mut entries: Vec<ModelPickerEntry> = models
            .available
            .iter()
            .map(|(id, info)| ModelPickerEntry {
                id: id.clone(),
                label: if info.name.is_empty() {
                    id.0.to_string()
                } else {
                    info.name.clone()
                },
                description: info.description.clone().unwrap_or_default(),
                efforts: models.reasoning_effort_options_for(id),
            })
            .collect();

        if let Some(current) = models.current.as_ref()
            && !entries.iter().any(|entry| entry.id == *current)
        {
            entries.push(ModelPickerEntry {
                id: current.clone(),
                label: current.0.to_string(),
                description: "Current model".into(),
                efforts: Vec::new(),
            });
        }
        if entries.is_empty() {
            return None;
        }

        let selected = models
            .current
            .as_ref()
            .and_then(|current| entries.iter().position(|entry| entry.id == *current))
            .unwrap_or(0);
        let original_label = models
            .current_model_name()
            .unwrap_or_else(|| entries[selected].label.clone());
        let mut picker = PickerState::default();
        picker.selected = selected;
        let effort = models
            .reasoning_effort
            .or_else(|| default_effort_for_entry(&entries[selected]));

        Some(Self {
            picker,
            entries,
            effort,
            original_label,
            effort_value_defined: models.reasoning_effort_explicit,
            effort_toggled: false,
        })
    }

    pub fn move_selection(&mut self, delta: isize) {
        let len = self.entries.len();
        if len == 0 {
            return;
        }
        self.picker.selected = if delta < 0 {
            self.picker
                .selected
                .checked_sub(delta.unsigned_abs())
                .unwrap_or(len - 1)
        } else {
            (self.picker.selected + delta as usize) % len
        };
        self.picker.hovered = None;
        self.picker.scroll_offset = None;
        if !self.effort_toggled && !self.effort_value_defined {
            self.effort = default_effort_for_entry(&self.entries[self.picker.selected]);
        }
    }

    pub fn cycle_effort(&mut self, delta: isize) {
        let Some(entry) = self.entries.get(self.picker.selected) else {
            return;
        };
        if entry.efforts.is_empty() {
            return;
        }
        let current = self
            .effort
            .and_then(|effort| {
                entry
                    .efforts
                    .iter()
                    .position(|option| option.value == effort)
            })
            // Claude keeps `max` as the picker-wide state when focus moves to
            // a model without max support, but uses `high` as the cycle basis.
            // Rust's canonical equivalent of `max` is `Xhigh`.
            .or_else(|| {
                if self.effort == Some(ReasoningEffort::Xhigh) {
                    entry
                        .efforts
                        .iter()
                        .position(|option| option.value == ReasoningEffort::High)
                } else {
                    None
                }
            })
            .or_else(|| entry.efforts.iter().position(|option| option.default))
            .unwrap_or(0);
        let len = entry.efforts.len();
        let next = if delta < 0 {
            current.checked_sub(delta.unsigned_abs()).unwrap_or(len - 1)
        } else {
            (current + delta as usize) % len
        };
        self.effort = Some(entry.efforts[next].value);
        self.effort_toggled = true;
    }

    pub fn selected(&self) -> Option<(acp::ModelId, Option<ReasoningEffort>)> {
        self.entries.get(self.picker.selected).map(|entry| {
            (
                entry.id.clone(),
                (self.effort_toggled && !entry.efforts.is_empty())
                    .then_some(self.effort)
                    .flatten(),
            )
        })
    }

    pub fn effort_label(&self, index: usize) -> String {
        if index != self.picker.selected {
            return String::new();
        }
        let Some(entry) = self.entries.get(index) else {
            return String::new();
        };
        if entry.efforts.is_empty() {
            return "Effort not supported".into();
        }
        display_effort_for_entry(entry, self.effort)
            .map(|effort| format!("{effort} effort"))
            .unwrap_or_else(|| "Effort not supported".into())
    }
}

fn default_effort_for_entry(entry: &ModelPickerEntry) -> Option<ReasoningEffort> {
    entry
        .efforts
        .iter()
        .find(|option| option.default)
        .or_else(|| entry.efforts.first())
        .map(|option| option.value)
}

fn display_effort_for_entry(
    entry: &ModelPickerEntry,
    effort: Option<ReasoningEffort>,
) -> Option<ReasoningEffort> {
    let effort = effort.or_else(|| default_effort_for_entry(entry))?;
    if effort == ReasoningEffort::Xhigh
        && !entry
            .efforts
            .iter()
            .any(|option| option.value == ReasoningEffort::Xhigh)
        && entry
            .efforts
            .iter()
            .any(|option| option.value == ReasoningEffort::High)
    {
        Some(ReasoningEffort::High)
    } else {
        Some(effort)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn info(id: &str, name: &str) -> (acp::ModelId, acp::ModelInfo) {
        let id = acp::ModelId::new(Arc::from(id));
        let info = acp::ModelInfo::new(id.clone(), name.to_string());
        (id, info)
    }

    fn reasoning_info(
        id: &str,
        name: &str,
        efforts: serde_json::Value,
    ) -> (acp::ModelId, acp::ModelInfo) {
        let id = acp::ModelId::new(Arc::from(id));
        let mut meta = serde_json::Map::new();
        meta.insert("supportsReasoningEffort".into(), serde_json::json!(true));
        meta.insert("reasoningEfforts".into(), efforts);
        let info = acp::ModelInfo::new(id.clone(), name.to_string()).meta(Some(meta));
        (id, info)
    }

    #[test]
    fn initial_selection_is_current_and_allowlist_filtered() {
        let mut models = ModelState::default();
        let (a, ai) = info("a", "A");
        let (b, bi) = info("b", "B");
        models.available.insert(a.clone(), ai);
        models.available.insert(b, bi);
        models.current = Some(a);

        let picker = ModelPickerState::new(&models).unwrap();
        assert_eq!(picker.entries.len(), 2);
        assert_eq!(picker.picker.selected, 0);
        assert_eq!(picker.original_label, "A");
    }

    #[test]
    fn current_outside_candidates_is_retained() {
        let mut models = ModelState::default();
        let (a, ai) = info("a", "A");
        models.available.insert(a, ai);
        models.current = Some(acp::ModelId::new(Arc::from("legacy")));

        let picker = ModelPickerState::new(&models).unwrap();
        assert_eq!(picker.entries.len(), 2);
        assert_eq!(
            picker.entries[picker.picker.selected].id.0.as_ref(),
            "legacy"
        );
    }

    #[test]
    fn left_right_cycles_only_the_selected_models_efforts() {
        let mut models = ModelState::default();
        let id = acp::ModelId::new(Arc::from("reasoning"));
        let mut meta = serde_json::Map::new();
        meta.insert("supportsReasoningEffort".into(), serde_json::json!(true));
        meta.insert(
            "reasoningEfforts".into(),
            serde_json::json!(["low", "high"]),
        );
        let info = acp::ModelInfo::new(id.clone(), "Reasoning").meta(Some(meta));
        models.available.insert(id.clone(), info);
        models.current = Some(id);
        models.reasoning_effort = Some(ReasoningEffort::Low);

        let mut picker = ModelPickerState::new(&models).unwrap();
        assert_eq!(picker.effort, Some(ReasoningEffort::Low));
        picker.cycle_effort(1);
        assert_eq!(picker.effort, Some(ReasoningEffort::High));
        picker.cycle_effort(-1);
        assert_eq!(picker.effort, Some(ReasoningEffort::Low));
    }

    #[test]
    fn untouched_effort_is_displayed_but_not_committed() {
        let mut models = ModelState::default();
        let id = acp::ModelId::new(Arc::from("reasoning"));
        let mut meta = serde_json::Map::new();
        meta.insert("supportsReasoningEffort".into(), serde_json::json!(true));
        meta.insert(
            "reasoningEfforts".into(),
            serde_json::json!([{"id": "high", "value": "high", "default": true}]),
        );
        let info = acp::ModelInfo::new(id.clone(), "Reasoning").meta(Some(meta));
        models.available.insert(id.clone(), info);
        models.current = Some(id.clone());

        let picker = ModelPickerState::new(&models).unwrap();
        assert_eq!(picker.effort, Some(ReasoningEffort::High));
        assert_eq!(picker.selected(), Some((id, None)));
    }

    #[test]
    fn toggled_effort_is_committed() {
        let mut models = ModelState::default();
        let id = acp::ModelId::new(Arc::from("reasoning"));
        let mut meta = serde_json::Map::new();
        meta.insert("supportsReasoningEffort".into(), serde_json::json!(true));
        meta.insert(
            "reasoningEfforts".into(),
            serde_json::json!(["low", "high"]),
        );
        let info = acp::ModelInfo::new(id.clone(), "Reasoning").meta(Some(meta));
        models.available.insert(id.clone(), info);
        models.current = Some(id.clone());

        let mut picker = ModelPickerState::new(&models).unwrap();
        picker.cycle_effort(1);
        assert_eq!(picker.selected(), Some((id, picker.effort)));
    }

    #[test]
    fn moving_between_models_with_different_defaults_preserves_active_effort() {
        let mut models = ModelState::default();
        let (a, ai) = reasoning_info(
            "a",
            "A",
            serde_json::json!([
                {"value": "low", "default": true},
                {"value": "high"}
            ]),
        );
        let (b, bi) = reasoning_info(
            "b",
            "B",
            serde_json::json!([
                {"value": "low"},
                {"value": "high", "default": true}
            ]),
        );
        models.available.insert(a.clone(), ai);
        models.available.insert(b, bi);
        models.current = Some(a);
        models.reasoning_effort = Some(ReasoningEffort::Low);
        models.reasoning_effort_explicit = true;

        let mut picker = ModelPickerState::new(&models).unwrap();
        picker.move_selection(1);

        assert_eq!(picker.effort, Some(ReasoningEffort::Low));
        assert_eq!(
            picker.selected(),
            Some((picker.entries[1].id.clone(), None))
        );
    }

    #[test]
    fn defined_medium_effort_survives_move_to_high_default_without_commit() {
        let mut models = ModelState::default();
        let (a, ai) = reasoning_info(
            "a",
            "A",
            serde_json::json!([
                {"value": "low", "default": true},
                {"value": "medium"},
                {"value": "high"}
            ]),
        );
        let (b, bi) = reasoning_info(
            "b",
            "B",
            serde_json::json!([
                {"value": "low"},
                {"value": "medium"},
                {"value": "high", "default": true}
            ]),
        );
        models.available.insert(a.clone(), ai);
        models.available.insert(b.clone(), bi);
        models.current = Some(a);
        models.reasoning_effort = Some(ReasoningEffort::Medium);
        models.reasoning_effort_explicit = true;

        let mut picker = ModelPickerState::new(&models).unwrap();
        picker.move_selection(1);

        assert_eq!(picker.effort, Some(ReasoningEffort::Medium));
        assert_eq!(picker.effort_label(picker.picker.selected), "medium effort");
        assert_eq!(picker.selected(), Some((b, None)));
    }

    #[test]
    fn toggled_effort_survives_model_move() {
        let mut models = ModelState::default();
        let (a, ai) = reasoning_info(
            "a",
            "A",
            serde_json::json!([
                {"value": "low", "default": true},
                {"value": "medium"},
                {"value": "high"}
            ]),
        );
        let (b, bi) = reasoning_info(
            "b",
            "B",
            serde_json::json!([
                {"value": "low"},
                {"value": "medium"},
                {"value": "high", "default": true}
            ]),
        );
        models.available.insert(a.clone(), ai);
        models.available.insert(b, bi);
        models.current = Some(a);
        models.reasoning_effort = Some(ReasoningEffort::Low);

        let mut picker = ModelPickerState::new(&models).unwrap();
        picker.cycle_effort(1);
        assert_eq!(picker.effort, Some(ReasoningEffort::Medium));
        picker.move_selection(1);

        assert_eq!(picker.effort, Some(ReasoningEffort::Medium));
    }

    #[test]
    fn untouched_effort_after_model_move_is_not_committed() {
        let mut models = ModelState::default();
        let (a, ai) = reasoning_info(
            "a",
            "A",
            serde_json::json!([{"value": "low", "default": true}]),
        );
        let (b, bi) = reasoning_info(
            "b",
            "B",
            serde_json::json!([{"value": "high", "default": true}]),
        );
        models.available.insert(a.clone(), ai);
        models.available.insert(b.clone(), bi);
        models.current = Some(a);
        models.reasoning_effort = Some(ReasoningEffort::Low);

        let mut picker = ModelPickerState::new(&models).unwrap();
        picker.move_selection(1);

        assert_eq!(picker.effort, Some(ReasoningEffort::High));
        assert_eq!(picker.selected(), Some((b, None)));
    }

    #[test]
    fn moving_back_does_not_restore_a_per_model_effort_memory() {
        let mut models = ModelState::default();
        let (a, ai) = reasoning_info(
            "a",
            "A",
            serde_json::json!([
                {"value": "low", "default": true},
                {"value": "medium"},
                {"value": "high"}
            ]),
        );
        let (b, bi) = reasoning_info(
            "b",
            "B",
            serde_json::json!([
                {"value": "low"},
                {"value": "medium"},
                {"value": "high", "default": true}
            ]),
        );
        models.available.insert(a.clone(), ai);
        models.available.insert(b, bi);
        models.current = Some(a);
        models.reasoning_effort = Some(ReasoningEffort::Low);
        models.reasoning_effort_explicit = true;

        let mut picker = ModelPickerState::new(&models).unwrap();
        picker.move_selection(1);
        picker.cycle_effort(1);
        assert_eq!(picker.effort, Some(ReasoningEffort::Medium));
        picker.move_selection(-1);

        assert_eq!(picker.effort, Some(ReasoningEffort::Medium));
    }

    #[test]
    fn unsupported_model_keeps_effort_state_but_does_not_commit_it() {
        let mut models = ModelState::default();
        let (a, ai) = reasoning_info("a", "A", serde_json::json!(["low", "high"]));
        let (plain, plain_info) = info("plain", "Plain");
        models.available.insert(a.clone(), ai);
        models.available.insert(plain.clone(), plain_info);
        models.current = Some(a);
        models.reasoning_effort = Some(ReasoningEffort::Low);

        let mut picker = ModelPickerState::new(&models).unwrap();
        picker.cycle_effort(1);
        assert_eq!(picker.effort, Some(ReasoningEffort::High));
        picker.move_selection(1);

        assert_eq!(picker.effort, Some(ReasoningEffort::High));
        assert_eq!(picker.selected(), Some((plain, None)));
        assert_eq!(
            picker.effort_label(picker.picker.selected),
            "Effort not supported"
        );
    }

    #[test]
    fn xhigh_state_displays_as_high_on_model_without_xhigh_support() {
        let mut models = ModelState::default();
        let (a, ai) = reasoning_info("a", "A", serde_json::json!(["low", "high", "xhigh"]));
        let (b, bi) = reasoning_info("b", "B", serde_json::json!(["low", "high"]));
        models.available.insert(a.clone(), ai);
        models.available.insert(b, bi);
        models.current = Some(a);
        models.reasoning_effort = Some(ReasoningEffort::Xhigh);
        models.reasoning_effort_explicit = true;

        let mut picker = ModelPickerState::new(&models).unwrap();
        picker.move_selection(1);

        assert_eq!(picker.effort, Some(ReasoningEffort::Xhigh));
        assert_eq!(picker.effort_label(picker.picker.selected), "high effort");
        picker.effort_toggled = true;
        assert_eq!(
            picker.selected(),
            Some((picker.entries[1].id.clone(), Some(ReasoningEffort::Xhigh)))
        );
    }
}
