//! Cohesive one-screen model picker used exclusively by `/models`.
//!
//! The picker snapshots the catalog when opened so filtering, navigation, and
//! pending effort changes remain stable if a live catalog update reorders or
//! removes models. The app adapter validates the final typed selection against
//! live [`ModelState`] before dispatching a model switch.

use agent_client_protocol as acp;
use crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;
use xai_grok_shell::sampling::types::ReasoningEffortOption;

use crate::acp::model_state::ModelState;
use crate::theme::Theme;
use crate::views::modal_window::{
    self as mw, ModalSizing, ModalWindowConfig, ModalWindowOutcome, ModalWindowState, Shortcut,
};
use crate::views::picker::{
    self, PickerConfig, PickerEntry, PickerOutcome, PickerRow, PickerState,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelPickerSelection {
    DefaultModel {
        model_id: acp::ModelId,
    },
    ExplicitEffort {
        model_id: acp::ModelId,
        effort: ReasoningEffortOption,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelPickerOutcome {
    Changed,
    Unchanged,
    Closed,
    Selected(ModelPickerSelection),
}

#[derive(Debug, Clone)]
struct ModelEntry {
    model_id: acp::ModelId,
    name: String,
    description: String,
    efforts: Vec<ReasoningEffortOption>,
    effort_index: usize,
    effort_touched: bool,
    current: bool,
}

impl ModelEntry {
    fn display_name(&self) -> String {
        format!(
            "{} {}",
            if self.effort_touched { '*' } else { ' ' },
            self.name
        )
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

    fn selected_effort(&self) -> Option<&ReasoningEffortOption> {
        self.efforts.get(self.effort_index)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffortDirection {
    Previous,
    Next,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EffortHitTarget {
    visible_index: usize,
    direction: EffortDirection,
    rect: Rect,
}

pub struct ModelPicker {
    picker: PickerState,
    entries: Vec<ModelEntry>,
    /// Visible picker index -> stable snapshot entry index.
    filtered_indices: Vec<usize>,
    window: ModalWindowState,
    effort_hit_targets: Vec<EffortHitTarget>,
    vim_normal_first: bool,
}

impl ModelPicker {
    pub fn new(models: &ModelState, vim_normal_first: bool) -> Self {
        let entries: Vec<ModelEntry> = models
            .available
            .iter()
            .map(|(model_id, info)| {
                let current = models.current.as_ref() == Some(model_id);
                let efforts = models.reasoning_effort_options_for(model_id);
                let active_effort = current.then_some(models.reasoning_effort).flatten();
                let effort_index = active_effort
                    .and_then(|effort| efforts.iter().position(|option| option.value == effort))
                    .or_else(|| efforts.iter().position(|option| option.default))
                    .unwrap_or(0);
                ModelEntry {
                    model_id: model_id.clone(),
                    name: info.name.clone(),
                    description: info.description.clone().unwrap_or_default(),
                    efforts,
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
        Self {
            picker,
            filtered_indices: (0..entries.len()).collect(),
            entries,
            window: ModalWindowState::new(),
            effort_hit_targets: Vec::new(),
            vim_normal_first,
        }
    }

    pub fn handle_event(&mut self, event: &Event) -> ModelPickerOutcome {
        if let Event::Mouse(mouse) = event {
            match mw::handle_modal_mouse(&mut self.window, mouse.kind, mouse.column, mouse.row) {
                ModalWindowOutcome::CloseRequested => return ModelPickerOutcome::Closed,
                ModalWindowOutcome::Handled => return ModelPickerOutcome::Changed,
                ModalWindowOutcome::Unhandled => {}
                _ => return ModelPickerOutcome::Changed,
            }

            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
                && let Some(target) = self.effort_hit_target(Position::new(mouse.column, mouse.row))
            {
                self.picker.selected = target.visible_index;
                self.picker.hovered = None;
                self.picker.selection_hidden = false;
                self.cycle_visible_effort(
                    target.visible_index,
                    target.direction == EffortDirection::Next,
                );
                return ModelPickerOutcome::Changed;
            }
        }

        if let Event::Key(key) = event {
            let chrome_config = self.modal_config(false, &[]);
            match mw::handle_modal_key(&mut self.window, key, &chrome_config) {
                ModalWindowOutcome::CloseRequested => {
                    if !self.picker.query.is_empty() {
                        self.picker.clear_query();
                        // Esc clear is also the Vim transition back to nav mode,
                        // so the next Esc closes rather than requiring a third press.
                        self.picker.search_active = false;
                        self.refresh_filter();
                        return ModelPickerOutcome::Changed;
                    }
                    // Delegate an empty-query Esc to PickerInput: in Vim input
                    // mode it first returns to nav mode; otherwise it closes.
                }
                ModalWindowOutcome::Unhandled => {}
                _ => return ModelPickerOutcome::Changed,
            }

            // Left/Right cycle effort only outside a query. While a query exists,
            // PickerInput owns both keys as cursor movement.
            if matches!(key.code, KeyCode::Left | KeyCode::Right)
                && self.picker.query.is_empty()
                && !self.picker.selection_hidden
                && !self.picker.tabs_focused
            {
                let visible_index = self.picker.selected;
                self.cycle_visible_effort(visible_index, key.code == KeyCode::Right);
                return ModelPickerOutcome::Changed;
            }
        }

        let config = self.picker_config();
        let query_before = self.picker.query.clone();
        let outcome = picker::handle_picker_input(
            event,
            &mut self.picker,
            self.filtered_indices.len(),
            &config,
        );
        if matches!(outcome, PickerOutcome::Changed) && self.picker.query != query_before {
            self.refresh_filter();
        }
        match outcome {
            PickerOutcome::Closed => ModelPickerOutcome::Closed,
            PickerOutcome::Selected(visible_index) => self
                .selection_for_visible(visible_index)
                .map(ModelPickerOutcome::Selected)
                .unwrap_or(ModelPickerOutcome::Changed),
            PickerOutcome::Changed => ModelPickerOutcome::Changed,
            PickerOutcome::Unchanged => ModelPickerOutcome::Unchanged,
            _ => ModelPickerOutcome::Changed,
        }
    }

    pub fn render(&mut self, buf: &mut Buffer, area: Rect, theme: &Theme, compact: bool) {
        let effort_width = self.effort_label_width();
        let display_names: Vec<String> =
            self.entries.iter().map(ModelEntry::display_name).collect();
        let effort_labels: Vec<String> = self
            .entries
            .iter()
            .map(|entry| self.effort_label(entry, effort_width))
            .collect();
        let picker_entries: Vec<PickerEntry<'_>> = self
            .filtered_indices
            .iter()
            .enumerate()
            .filter_map(|(visible_index, &entry_index)| {
                let entry = self.entries.get(entry_index)?;
                Some(PickerEntry::Row(PickerRow {
                    label: &display_names[entry_index],
                    right_label: &effort_labels[entry_index],
                    selected: self.picker.hovered == Some(visible_index)
                        || (self.picker.hovered.is_none()
                            && !self.picker.selection_hidden
                            && self.picker.selected == visible_index),
                    expanded: false,
                    fields: &[],
                    description_lines: &[],
                    summary_lines: &[],
                    dimmed: false,
                    indent: 0,
                    badge: if entry.current { "current" } else { "" },
                    badge_color: None,
                    collapsible: false,
                    underline_last_desc: false,
                }))
            })
            .collect();

        let mut shortcuts = vec![
            Shortcut {
                label: "↑/↓ model",
                clickable: false,
                id: 0,
            },
            Shortcut {
                label: "←/→ effort",
                clickable: false,
                id: 0,
            },
            Shortcut {
                label: "Enter select",
                clickable: false,
                id: 0,
            },
            Shortcut {
                label: "Esc cancel",
                clickable: false,
                id: 0,
            },
        ];
        mw::push_vim_nav_search_hint(&mut shortcuts, self.picker.search_active);
        let modal_config = self.modal_config(compact, &shortcuts);
        if let Some(content) =
            mw::render_modal_window(buf, area, &mut self.window, &modal_config, theme)
        {
            picker::render_picker_in_modal(
                buf,
                content.content,
                content.inner_x,
                content.inner_width,
                theme,
                &mut self.picker,
                &picker_entries,
                &[],
                false,
            );
            self.render_effort_overlay(buf, theme, effort_width);
        } else {
            self.effort_hit_targets.clear();
        }
    }

    fn picker_config(&self) -> PickerConfig<'static> {
        PickerConfig {
            title: None,
            show_search_hint: false,
            expandable: false,
            esc_clears_query: true,
            shortcuts: Some(picker::picker_shortcuts()),
            pending_hint: None,
            non_selectable: &[],
            non_selectable_clickable: &[],
            shortcuts_area: None,
            tabs: None,
            active_tab: 0,
            filter_label: None,
            filter_key_hint: None,
            filter_active: false,
            action_keys: &[],
            disable_search: false,
            compact_bottom_bar: false,
            search_only_on_slash: false,
            vim_normal_first: self.vim_normal_first,
        }
    }

    fn modal_config<'a>(
        &self,
        compact: bool,
        shortcuts: &'a [Shortcut<'a>],
    ) -> ModalWindowConfig<'a> {
        ModalWindowConfig {
            title: "Model selection",
            tabs: None,
            shortcuts,
            sizing: ModalSizing {
                width_pct: 0.50,
                max_width: 80,
                min_width: 44,
                v_margin: 4,
                h_pad: 2,
                v_pad: 1,
                footer_lines: 2,
            }
            .with_compact(compact),
            fold_info: None,
        }
    }

    fn refresh_filter(&mut self) {
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

    fn visible_entry(&self, visible_index: usize) -> Option<&ModelEntry> {
        self.filtered_indices
            .get(visible_index)
            .and_then(|&entry_index| self.entries.get(entry_index))
    }

    fn visible_entry_mut(&mut self, visible_index: usize) -> Option<&mut ModelEntry> {
        let entry_index = *self.filtered_indices.get(visible_index)?;
        self.entries.get_mut(entry_index)
    }

    fn cycle_visible_effort(&mut self, visible_index: usize, forward: bool) -> bool {
        self.visible_entry_mut(visible_index)
            .is_some_and(|entry| entry.cycle_effort(forward))
    }

    fn selection_for_visible(&self, visible_index: usize) -> Option<ModelPickerSelection> {
        let entry = self.visible_entry(visible_index)?;
        if entry.effort_touched
            && let Some(effort) = entry.selected_effort()
        {
            return Some(ModelPickerSelection::ExplicitEffort {
                model_id: entry.model_id.clone(),
                effort: effort.clone(),
            });
        }
        Some(ModelPickerSelection::DefaultModel {
            model_id: entry.model_id.clone(),
        })
    }

    fn effort_label_width(&self) -> usize {
        self.entries
            .iter()
            .flat_map(|entry| entry.efforts.iter().map(|effort| effort.label.width()))
            .max()
            .unwrap_or(0)
    }

    fn effort_label(&self, entry: &ModelEntry, width: usize) -> String {
        let Some(effort) = entry.selected_effort() else {
            return "{unavailable}".to_string();
        };
        let padding = width.saturating_sub(effort.label.width());
        format!("< {{{}{}}} >", effort.label, " ".repeat(padding))
    }

    fn render_effort_overlay(&mut self, buf: &mut Buffer, theme: &Theme, effort_width: usize) {
        self.effort_hit_targets.clear();
        let Some(hit_areas) = self.picker.hit_areas.as_ref() else {
            return;
        };
        let rows: Vec<(Rect, usize)> = hit_areas
            .item_rects
            .iter()
            .copied()
            .zip(hit_areas.entry_indices.iter().copied())
            .collect();
        let control_width = effort_width as u16 + 6;
        let rendered_rows = rows.len();
        let visible_height: usize = self
            .filtered_indices
            .iter()
            .filter_map(|&entry_index| self.entries.get(entry_index))
            .map(|_| 1usize)
            .sum();
        let scrollbar_visible = visible_height > rendered_rows;
        for (rect, visible_index) in rows {
            let Some(entry) = self.visible_entry(visible_index) else {
                continue;
            };
            let label = self.effort_label(entry, effort_width);
            let label_width = label.width() as u16;
            let rendered_width = rect.width.saturating_sub(u16::from(scrollbar_visible));
            if rendered_width <= label_width {
                continue;
            }
            // Picker rows reserve one trailing cell after right-aligned labels.
            let control_x = rect.x + rendered_width - label_width - 1;
            let selected = self.picker.hovered == Some(visible_index)
                || (self.picker.hovered.is_none()
                    && !self.picker.selection_hidden
                    && self.picker.selected == visible_index);
            let bg = buf
                .cell((control_x, rect.y))
                .and_then(|cell| cell.style().bg)
                .unwrap_or(theme.bg_base);
            let color = entry
                .selected_effort()
                .and_then(|effort| effort_color(&effort.id))
                .filter(|_| selected)
                .unwrap_or(theme.gray);
            buf.set_span(
                control_x,
                rect.y,
                &Span::styled(&label, Style::default().fg(color).bg(bg)),
                label_width,
            );

            if !entry.efforts.is_empty() && label_width == control_width {
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
    }

    fn effort_hit_target(&self, position: Position) -> Option<EffortHitTarget> {
        self.effort_hit_targets
            .iter()
            .copied()
            .find(|target| target.rect.contains(position))
    }
}

fn effort_color(id: &str) -> Option<Color> {
    let color = match id.to_ascii_lowercase().as_str() {
        "low" => Color::Rgb(0x7A, 0xA2, 0xF7),
        "medium" => Color::Rgb(0x2A, 0xC3, 0xDE),
        "high" => Color::Rgb(0xE0, 0xAF, 0x68),
        "xhigh" => Color::Rgb(0xFF, 0x9E, 0x64),
        "max" => Color::Rgb(0xF7, 0x76, 0x8E),
        _ => return None,
    };
    Some(crate::theme::quantize(color))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crossterm::event::{KeyEvent, KeyModifiers};
    use xai_grok_shell::sampling::types::ReasoningEffort;

    use super::*;

    fn reasoning_model(id: &str, name: &str, description: &str) -> (acp::ModelId, acp::ModelInfo) {
        let id = acp::ModelId::new(Arc::from(id));
        let meta = serde_json::json!({
            "supportsReasoningEffort": true,
            "reasoningEfforts": [
                { "id": "low", "value": "low", "label": "Low", "default": true },
                { "id": "deep", "value": "xhigh", "label": "Deep" }
            ]
        });
        let info = acp::ModelInfo::new(id.clone(), name.to_string())
            .description(description.to_string())
            .meta(meta.as_object().cloned());
        (id, info)
    }

    fn sample_models() -> ModelState {
        let mut models = ModelState::default();
        let (alpha_id, alpha) = reasoning_model("alpha-id", "Alpha", "fast reasoning");
        let beta_id = acp::ModelId::new(Arc::from("beta-id"));
        let beta = acp::ModelInfo::new(beta_id.clone(), "Beta".to_string())
            .description("plain model".to_string());
        models.available.insert(alpha_id.clone(), alpha);
        models.available.insert(beta_id, beta);
        models.current = Some(alpha_id);
        models.reasoning_effort = Some(ReasoningEffort::Low);
        models
    }

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn snapshot_filter_searches_name_id_and_description() {
        for query in ["alpha", "alpha-id", "reasoning"] {
            let mut picker = ModelPicker::new(&sample_models(), false);
            picker.picker.query = query.into();
            picker.picker.query_cursor = query.len();
            picker.refresh_filter();
            assert_eq!(picker.filtered_indices, vec![0]);
        }
        let mut picker = ModelPicker::new(&sample_models(), false);
        picker.picker.query = "plain".into();
        picker.refresh_filter();
        assert_eq!(picker.filtered_indices, vec![1]);
    }

    #[test]
    fn effort_cycles_both_directions_with_wrap_and_marks_pending() {
        let mut picker = ModelPicker::new(&sample_models(), false);
        assert!(picker.cycle_visible_effort(0, false));
        assert_eq!(picker.entries[0].effort_index, 1);
        assert!(picker.entries[0].effort_touched);
        assert!(picker.cycle_visible_effort(0, true));
        assert_eq!(picker.entries[0].effort_index, 0);
        assert!(picker.entries[0].display_name().starts_with('*'));
    }

    #[test]
    fn unsupported_effort_is_visible_and_never_pending() {
        let mut picker = ModelPicker::new(&sample_models(), false);
        assert_eq!(picker.effort_label(&picker.entries[1], 4), "{unavailable}");
        assert!(!picker.cycle_visible_effort(1, true));
        assert!(!picker.entries[1].effort_touched);
    }

    #[test]
    fn left_right_move_query_cursor_instead_of_cycling_effort() {
        let mut picker = ModelPicker::new(&sample_models(), false);
        picker.picker.query = "ab".into();
        picker.picker.query_cursor = 2;
        assert_eq!(
            picker.handle_event(&key(KeyCode::Left)),
            ModelPickerOutcome::Changed
        );
        assert_eq!(picker.picker.query_cursor, 1);
        assert!(!picker.entries[0].effort_touched);
    }

    #[test]
    fn enter_converts_effort_touched_to_typed_selection() {
        let mut picker = ModelPicker::new(&sample_models(), false);
        picker.cycle_visible_effort(0, true);
        let outcome = picker.handle_event(&key(KeyCode::Enter));
        assert!(matches!(
            outcome,
            ModelPickerOutcome::Selected(ModelPickerSelection::ExplicitEffort {
                ref model_id,
                ref effort,
            }) if model_id.0.as_ref() == "alpha-id"
                && effort.id == "deep"
                && effort.value == ReasoningEffort::Xhigh
        ));
    }

    #[test]
    fn untouched_enter_selects_default_model_lifecycle() {
        let mut picker = ModelPicker::new(&sample_models(), false);
        picker.picker.selected = 1;
        assert!(matches!(
            picker.handle_event(&key(KeyCode::Enter)),
            ModelPickerOutcome::Selected(ModelPickerSelection::DefaultModel { ref model_id })
                if model_id.0.as_ref() == "beta-id"
        ));
    }

    #[test]
    fn escape_clears_query_then_closes() {
        let mut picker = ModelPicker::new(&sample_models(), false);
        picker.picker.query = "alpha".into();
        picker.picker.query_cursor = 5;
        assert_eq!(
            picker.handle_event(&key(KeyCode::Esc)),
            ModelPickerOutcome::Changed
        );
        assert!(picker.picker.query.is_empty());
        assert_eq!(
            picker.handle_event(&key(KeyCode::Esc)),
            ModelPickerOutcome::Closed
        );
    }

    #[test]
    fn snapshot_selection_stays_bound_to_id_after_live_reorder() {
        let mut picker = ModelPicker::new(&sample_models(), false);
        picker.picker.selected = 1;
        let selection = picker.selection_for_visible(1).unwrap();
        assert!(matches!(
            selection,
            ModelPickerSelection::DefaultModel { ref model_id }
                if model_id.0.as_ref() == "beta-id"
        ));
    }

    #[test]
    fn clickable_effort_arrow_cycles_without_confirming_row() {
        let mut picker = ModelPicker::new(&sample_models(), false);
        picker.effort_hit_targets = vec![EffortHitTarget {
            visible_index: 0,
            direction: EffortDirection::Next,
            rect: Rect::new(10, 10, 2, 1),
        }];
        let event = Event::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 10,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(picker.handle_event(&event), ModelPickerOutcome::Changed);
        assert_eq!(picker.entries[0].effort_index, 1);
        assert!(picker.entries[0].effort_touched);
    }

    #[test]
    fn row_click_confirms_the_hit_tested_snapshot_entry() {
        let mut picker = ModelPicker::new(&sample_models(), false);
        picker.picker.hit_areas = Some(picker::PickerHitAreas {
            close_button: Rect::default(),
            search_bar: Rect::default(),
            item_rects: vec![Rect::new(5, 7, 20, 1)],
            entry_indices: vec![1],
            tab_rects: vec![],
            filter_rect: None,
        });
        let event = Event::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 8,
            row: 7,
            modifiers: KeyModifiers::NONE,
        });
        assert!(matches!(
            picker.handle_event(&event),
            ModelPickerOutcome::Selected(ModelPickerSelection::DefaultModel { ref model_id })
                if model_id.0.as_ref() == "beta-id"
        ));
    }

    #[test]
    fn wheel_and_hover_delegate_to_picker_input() {
        let mut picker = ModelPicker::new(&sample_models(), false);
        picker.picker.hit_areas = Some(picker::PickerHitAreas {
            close_button: Rect::default(),
            search_bar: Rect::default(),
            item_rects: vec![Rect::new(5, 7, 20, 1), Rect::new(5, 8, 20, 1)],
            entry_indices: vec![0, 1],
            tab_rects: vec![],
            filter_rect: None,
        });
        let hover = Event::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Moved,
            column: 8,
            row: 8,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(picker.handle_event(&hover), ModelPickerOutcome::Changed);
        assert_eq!(picker.picker.hovered, Some(1));

        let wheel = Event::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 8,
            row: 8,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(picker.handle_event(&wheel), ModelPickerOutcome::Changed);
        assert_eq!(picker.picker.scroll_offset, Some(3));
        assert_eq!(picker.picker.hovered, None);
    }

    #[test]
    fn vim_escape_still_clears_then_closes() {
        let mut picker = ModelPicker::new(&sample_models(), true);
        picker.picker.query = "alpha".into();
        picker.picker.query_cursor = 5;
        assert_eq!(
            picker.handle_event(&key(KeyCode::Esc)),
            ModelPickerOutcome::Changed
        );
        assert!(!picker.picker.search_active);
        assert_eq!(
            picker.handle_event(&key(KeyCode::Esc)),
            ModelPickerOutcome::Closed
        );
    }

    #[test]
    fn effort_colors_cover_supported_scale() {
        for id in ["low", "medium", "high", "xhigh", "max"] {
            assert!(effort_color(id).is_some());
        }
        assert!(effort_color("minimal").is_none());
    }
}
