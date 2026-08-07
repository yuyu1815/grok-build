//! UI-only state for the one-screen `/models` picker.
//!
//! The picker snapshots typed model IDs and display metadata when it opens. A
//! live catalog refresh can therefore neither reorder the visible rows nor
//! silently retarget a pending confirmation.

use agent_client_protocol as acp;
use xai_grok_shell::sampling::types::ReasoningEffort;

use crate::acp::model_state::ModelState;
use crate::views::modal_window::ModalWindowState;
use crate::views::picker::PickerState;

#[derive(Debug, Clone)]
pub struct ModelPickerEffort {
    pub id: String,
    pub label: String,
    pub value: ReasoningEffort,
}

#[derive(Debug, Clone)]
pub struct ModelPickerEntry {
    pub model_id: acp::ModelId,
    pub name: String,
    pub description: String,
    pub efforts: Vec<ModelPickerEffort>,
    pub effort_index: usize,
    pub effort_touched: bool,
    pub current: bool,
}

pub struct ModelPickerState {
    pub picker: PickerState,
    pub entries: Vec<ModelPickerEntry>,
    /// Visible picker index -> stable snapshot entry index.
    pub filtered_indices: Vec<usize>,
    pub window: ModalWindowState,
}

impl ModelPickerState {
    pub fn new(models: &ModelState) -> Self {
        let entries: Vec<ModelPickerEntry> = models
            .available
            .iter()
            .map(|(model_id, info)| {
                let current = models.current.as_ref() == Some(model_id);
                let options = models.reasoning_effort_options_for(model_id);
                let active = current.then_some(models.reasoning_effort).flatten();
                let effort_index = active
                    .and_then(|effort| options.iter().position(|option| option.value == effort))
                    .or_else(|| options.iter().position(|option| option.default))
                    .unwrap_or(0);
                ModelPickerEntry {
                    model_id: model_id.clone(),
                    name: info.name.clone(),
                    description: info.description.clone().unwrap_or_default(),
                    efforts: options
                        .into_iter()
                        .map(|option| ModelPickerEffort {
                            id: option.id,
                            label: option.label,
                            value: option.value,
                        })
                        .collect(),
                    effort_index,
                    effort_touched: false,
                    current,
                }
            })
            .collect();
        let selected = models
            .current
            .as_ref()
            .and_then(|id| entries.iter().position(|entry| &entry.model_id == id))
            .unwrap_or(0);
        let mut picker = PickerState::input_active();
        picker.selected = selected;
        let filtered_indices = (0..entries.len()).collect();
        Self {
            picker,
            entries,
            filtered_indices,
            window: ModalWindowState::new(),
        }
    }

    pub fn refresh_filter(&mut self) {
        let query = self.picker.query.to_lowercase();
        self.filtered_indices = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                (query.is_empty()
                    || entry.name.to_lowercase().contains(&query)
                    || entry.model_id.0.as_ref().to_lowercase().contains(&query)
                    || entry.description.to_lowercase().contains(&query))
                .then_some(index)
            })
            .collect();
        self.picker.selected = self
            .picker
            .selected
            .min(self.filtered_indices.len().saturating_sub(1));
    }

    pub fn visible_entry(&self, visible_index: usize) -> Option<&ModelPickerEntry> {
        self.filtered_indices
            .get(visible_index)
            .and_then(|&entry_index| self.entries.get(entry_index))
    }

    pub fn visible_entry_mut(&mut self, visible_index: usize) -> Option<&mut ModelPickerEntry> {
        let entry_index = *self.filtered_indices.get(visible_index)?;
        self.entries.get_mut(entry_index)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn filter_indices_map_back_to_stable_model_ids() {
        let mut models = ModelState::default();
        for (id, name) in [("alpha", "Alpha"), ("beta", "Beta"), ("gamma", "Gamma")] {
            let id = acp::ModelId::new(Arc::from(id));
            models
                .available
                .insert(id.clone(), acp::ModelInfo::new(id, name.to_string()));
        }
        let mut state = ModelPickerState::new(&models);
        state.picker.query = "bet".into();
        state.refresh_filter();
        assert_eq!(state.filtered_indices, vec![1]);
        assert_eq!(state.visible_entry(0).unwrap().model_id.0.as_ref(), "beta");
    }
}
