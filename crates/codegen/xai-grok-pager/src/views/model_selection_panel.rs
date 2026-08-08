//! UI-only state for the one-screen `/models` model selection panel.
//!
//! The panel snapshots typed model IDs and display metadata when it opens. A
//! live catalog refresh can therefore neither reorder the visible rows nor
//! silently retarget a pending confirmation.

use agent_client_protocol as acp;
use ratatui::layout::{Position, Rect};
use ratatui::style::Color;
use unicode_width::UnicodeWidthStr;
use xai_grok_shell::sampling::types::ReasoningEffort;

use crate::acp::model_state::ModelState;
use crate::views::modal_window::ModalWindowState;
use crate::views::picker::PickerState;

#[derive(Debug, Clone)]
pub struct EffortOption {
    pub id: String,
    pub label: String,
    pub value: ReasoningEffort,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub model_id: acp::ModelId,
    pub name: String,
    pub description: String,
    pub efforts: Vec<EffortOption>,
    pub effort_index: usize,
    pub effort_touched: bool,
    pub current: bool,
}

impl Entry {
    pub fn display_name(&self) -> String {
        format!(
            "{} {}",
            if self.effort_touched { '*' } else { ' ' },
            self.name
        )
    }

    pub fn effort_label(&self, width: usize) -> String {
        let Some(effort) = self.efforts.get(self.effort_index) else {
            return "{unavailable}".to_string();
        };
        format!("< {{{}}} >", pad_to_width(&effort.label, width))
    }

    pub fn selected_effort_color(&self, selected: bool) -> Option<Color> {
        if !selected {
            return None;
        }
        let option = self.efforts.get(self.effort_index)?;
        let (r, g, b) = effort_rgb(&option.id)?;
        Some(crate::theme::quantize(Color::Rgb(r, g, b)))
    }

    fn cycle_effort(&mut self, forward: bool) -> bool {
        if self.efforts.is_empty() {
            return false;
        }
        self.effort_index = if forward {
            (self.effort_index + 1) % self.efforts.len()
        } else {
            (self.effort_index + self.efforts.len() - 1) % self.efforts.len()
        };
        self.effort_touched = true;
        true
    }
}

fn pad_to_width(label: &str, width: usize) -> String {
    let padding = width.saturating_sub(label.width());
    format!("{label}{}", " ".repeat(padding))
}

fn effort_rgb(id: &str) -> Option<(u8, u8, u8)> {
    match id.to_ascii_lowercase().as_str() {
        "low" => Some((0x7A, 0xA2, 0xF7)),    // Low -> blue
        "medium" => Some((0x2A, 0xC3, 0xDE)), // Medium -> cyan
        "high" => Some((0xE0, 0xAF, 0x68)),   // High -> yellow
        "xhigh" => Some((0xFF, 0x9E, 0x64)),  // XHigh -> orange
        "max" => Some((0xF7, 0x76, 0x8E)),    // Max -> red/pink
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffortDirection {
    Previous,
    Next,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffortHitTarget {
    pub visible_index: usize,
    pub direction: EffortDirection,
    pub rect: Rect,
}

pub struct State {
    pub picker: PickerState,
    pub entries: Vec<Entry>,
    /// Visible picker index -> stable snapshot entry index.
    pub filtered_indices: Vec<usize>,
    pub window: ModalWindowState,
    pub effort_hit_targets: Vec<EffortHitTarget>,
}

impl State {
    pub fn new(models: &ModelState) -> Self {
        let entries: Vec<Entry> = models
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
                Entry {
                    model_id: model_id.clone(),
                    name: info.name.clone(),
                    description: info.description.clone().unwrap_or_default(),
                    efforts: options
                        .into_iter()
                        .map(|option| EffortOption {
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
            effort_hit_targets: Vec::new(),
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

    pub fn visible_entry(&self, visible_index: usize) -> Option<&Entry> {
        self.filtered_indices
            .get(visible_index)
            .and_then(|&entry_index| self.entries.get(entry_index))
    }

    pub fn visible_entry_mut(&mut self, visible_index: usize) -> Option<&mut Entry> {
        let entry_index = *self.filtered_indices.get(visible_index)?;
        self.entries.get_mut(entry_index)
    }

    pub fn cycle_visible_effort(&mut self, visible_index: usize, forward: bool) -> bool {
        self.visible_entry_mut(visible_index)
            .is_some_and(|entry| entry.cycle_effort(forward))
    }

    pub fn effort_label_width(&self) -> usize {
        self.entries
            .iter()
            .flat_map(|entry| entry.efforts.iter().map(|effort| effort.label.width()))
            .max()
            .unwrap_or(0)
    }

    pub fn refresh_effort_hit_targets(&mut self) {
        self.effort_hit_targets.clear();
        let Some(hit_areas) = self.picker.hit_areas.as_ref() else {
            return;
        };
        let control_width = self.effort_label_width() as u16 + 6; // `< {` + label + `} >`
        for (rect, &visible_index) in hit_areas
            .item_rects
            .iter()
            .zip(hit_areas.entry_indices.iter())
        {
            let Some(entry) = self.visible_entry(visible_index) else {
                continue;
            };
            if entry.efforts.is_empty() || rect.width <= control_width {
                continue;
            }
            // Picker rows reserve one trailing cell after the right-aligned label.
            let control_x = rect.x + rect.width - control_width - 1;
            // Make each arrow easier to click without overlapping the effort value:
            // `< ` for previous and ` >` for next.
            self.effort_hit_targets.push(EffortHitTarget {
                visible_index,
                direction: EffortDirection::Previous,
                rect: Rect::new(control_x, rect.y, 2, 1),
            });
            self.effort_hit_targets.push(EffortHitTarget {
                visible_index,
                direction: EffortDirection::Next,
                rect: Rect::new(control_x + control_width - 2, rect.y, 2, 1),
            });
        }
    }

    pub fn effort_hit_target(&self, position: Position) -> Option<EffortHitTarget> {
        self.effort_hit_targets
            .iter()
            .copied()
            .find(|target| target.rect.contains(position))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn models_with_reasoning() -> ModelState {
        let mut models = ModelState::default();
        let id = acp::ModelId::new(Arc::from("grok-4.5"));
        let meta = serde_json::json!({
            "supportsReasoningEffort": true,
            "reasoningEfforts": [
                { "id": "low", "value": "low", "label": "Low", "default": true },
                { "id": "high", "value": "high", "label": "High" }
            ]
        });
        models.available.insert(
            id.clone(),
            acp::ModelInfo::new(id.clone(), "Grok 4.5".to_string()).meta(meta.as_object().cloned()),
        );
        models.current = Some(id);
        models
    }

    #[test]
    fn filter_indices_map_back_to_stable_model_ids() {
        let mut models = ModelState::default();
        for (id, name) in [("alpha", "Alpha"), ("beta", "Beta"), ("gamma", "Gamma")] {
            let id = acp::ModelId::new(Arc::from(id));
            models
                .available
                .insert(id.clone(), acp::ModelInfo::new(id, name.to_string()));
        }
        let mut state = State::new(&models);
        state.picker.query = "bet".into();
        state.refresh_filter();
        assert_eq!(state.filtered_indices, vec![1]);
        assert_eq!(state.visible_entry(0).unwrap().model_id.0.as_ref(), "beta");
    }

    #[test]
    fn effort_label_marks_an_unconfirmed_change() {
        let mut state = State::new(&models_with_reasoning());
        assert_eq!(state.entries[0].display_name(), "  Grok 4.5");
        assert_eq!(state.entries[0].effort_label(6), "< {Low   } >");
        assert!(state.cycle_visible_effort(0, true));
        assert_eq!(state.entries[0].display_name(), "* Grok 4.5");
        assert_eq!(state.entries[0].effort_label(6), "< {High  } >");
    }

    #[test]
    fn effort_colors_progress_from_blue_to_red() {
        assert_eq!(effort_rgb("low"), Some((0x7A, 0xA2, 0xF7)));
        assert_eq!(effort_rgb("medium"), Some((0x2A, 0xC3, 0xDE)));
        assert_eq!(effort_rgb("high"), Some((0xE0, 0xAF, 0x68)));
        assert_eq!(effort_rgb("xhigh"), Some((0xFF, 0x9E, 0x64)));
        assert_eq!(effort_rgb("max"), Some((0xF7, 0x76, 0x8E)));
        assert_eq!(effort_rgb("minimal"), None);
    }

    #[test]
    fn only_the_selected_row_gets_an_effort_color() {
        let state = State::new(&models_with_reasoning());
        assert_eq!(state.entries[0].selected_effort_color(false), None);
        assert!(state.entries[0].selected_effort_color(true).is_some());
    }

    #[test]
    fn effort_cycles_in_both_directions_with_wrapping() {
        let mut state = State::new(&models_with_reasoning());
        assert!(state.cycle_visible_effort(0, false));
        assert_eq!(state.entries[0].effort_index, 1);
        assert!(state.cycle_visible_effort(0, true));
        assert_eq!(state.entries[0].effort_index, 0);
        assert!(state.entries[0].effort_touched);
    }

    #[test]
    fn effort_hit_targets_keep_buttons_at_fixed_columns() {
        let mut state = State::new(&models_with_reasoning());
        state.picker.hit_areas = Some(crate::views::picker::PickerHitAreas {
            close_button: Rect::default(),
            search_bar: Rect::default(),
            item_rects: vec![Rect::new(10, 20, 30, 1)],
            entry_indices: vec![0],
            tab_rects: vec![],
            filter_rect: None,
        });
        state.refresh_effort_hit_targets();
        assert_eq!(state.effort_hit_targets.len(), 2);
        let previous = state.effort_hit_targets[0];
        let next = state.effort_hit_targets[1];
        assert_eq!(previous.rect, Rect::new(32, 20, 2, 1));
        assert_eq!(next.rect, Rect::new(37, 20, 2, 1));
        assert_eq!(
            state
                .effort_hit_target(Position::new(33, 20))
                .unwrap()
                .direction,
            EffortDirection::Previous
        );
        assert_eq!(
            state
                .effort_hit_target(Position::new(37, 20))
                .unwrap()
                .direction,
            EffortDirection::Next
        );

        assert!(state.cycle_visible_effort(0, true));
        state.refresh_effort_hit_targets();
        assert_eq!(state.effort_hit_targets[0].rect, previous.rect);
        assert_eq!(state.effort_hit_targets[1].rect, next.rect);
    }

    #[test]
    fn unsupported_effort_is_visible_and_does_not_become_pending() {
        let mut models = ModelState::default();
        let id = acp::ModelId::new(Arc::from("plain"));
        models
            .available
            .insert(id.clone(), acp::ModelInfo::new(id, "Plain".to_string()));
        let mut state = State::new(&models);
        assert_eq!(state.entries[0].effort_label(11), "{unavailable}");
        assert!(!state.cycle_visible_effort(0, true));
        assert!(!state.entries[0].effort_touched);
    }
}
