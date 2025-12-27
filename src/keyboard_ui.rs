use crate::{
    editor_state::EditorState,
    timing_util::{meta_timing_points_sorted, snap_time_to_beat_divisor_ms},
};
use bevy_egui::egui;

pub const KEYBOARD_LAYOUT: &[&[&str]] = &[
    &["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"],
    &["A", "S", "D", "F", "G", "H", "J", "K", "L", ";"],
    &["Z", "X", "C", "V", "B", "N", "M", ",", ".", "/"],
];

pub fn draw_keyboard(ui: &mut egui::Ui, state: &mut EditorState, keyboard_rect: egui::Rect) {
    ui.allocate_ui_at_rect(keyboard_rect, |ui| {
        ui.set_min_size(keyboard_rect.size());

        ui.group(|ui| {
            ui.set_min_size(keyboard_rect.size());
            ui.heading("🎹 Keyboard (active + click to place)");

            let mut hovered_key: Option<String> = None;

            // Get all keys that are active at current time
            let tap_linger_ms: u32 = 120;
            let active_keys: std::collections::HashSet<String> = state
                .beatmap
                .notes
                .iter()
                .filter(|n| {
                    let start = n.get_start_time();
                    let end = if n.note_type == "hold" {
                        n.get_end_time()
                    } else {
                        start.saturating_add(tap_linger_ms)
                    };
                    state.current_time >= start && state.current_time <= end
                })
                .map(|n| n.key.to_lowercase())
                .collect();

            // Scale keys to fill space.
            let row_count = KEYBOARD_LAYOUT.len().max(1) as f32;
            let spacing_y = ui.spacing().item_spacing.y;
            let spacing_x = ui.spacing().item_spacing.x;

            // Reserve a little room for the group heading
            let available_h = (ui.available_height() - 10.0).max(0.0);
            let key_h = ((available_h - spacing_y * (row_count - 1.0)) / row_count)
                .clamp(36.0, 110.0);

                let max_cols = KEYBOARD_LAYOUT
                .iter()
                .map(|r| r.len())
                .max()
                .unwrap_or(1)
                .max(1) as f32;

            let max_indent_factor = 0.70_f32;
            let key_w = ((ui.available_width()
                - (max_indent_factor * (ui.available_width() / max_cols).max(24.0))
                - spacing_x * (max_cols - 1.0))
                / max_cols)
                .max(24.0);

            for (row_idx, &row) in KEYBOARD_LAYOUT.iter().enumerate() {
                // Give the key rows an indent for the layout
                let indent_factor = match row_idx {
                    1 => 0.35,
                    2 => 0.70,
                    _ => 0.0,
                };
                let indent = key_w * indent_factor;

                ui.horizontal(|ui| {
                    if indent > 0.0 {
                        ui.add_space(indent);
                    }

                    for &key in row {
                        let key_lower: String = key.to_lowercase();
                        let is_active = active_keys.contains(&key_lower);
                        let is_hold_toggled = state.hold_starts.contains_key(&key_lower);

                        let button = egui::Button::new(
                            egui::RichText::new(key)
                                .color(egui::Color32::from_rgb(220, 220, 220))
                                .size((key_h * 0.45).clamp(12.0, 26.0)),
                        )
                        .fill(if is_active {
                            egui::Color32::from_rgb(110, 110, 110)
                        } else if is_hold_toggled {
                            // Indicates this key has a toggled hold
                            egui::Color32::from_rgb(70, 70, 70)
                        } else {
                            egui::Color32::from_rgb(42, 42, 42)
                        });

                        let resp = ui.add_sized(egui::Vec2::new(key_w, key_h), button);
                        if resp.hovered() {
                            hovered_key = Some(key_lower.clone());
                        }

                        // Right-click deletes a note at the current snapped time.
                        if resp.secondary_clicked() {
                            let timing_points = meta_timing_points_sorted(&state.meta);
                            let snapped_time = snap_time_to_beat_divisor_ms(&timing_points, state.current_time, 2);

                            // If this key currently has a toggled hold, toggling it off is probably
                            // not what the user intends when deleting; keep the toggle as-is and
                            // only delete committed notes.
                            let deleted = state.beatmap.delete_note_at(&key_lower, snapped_time);
                            if deleted {
                                state.status = format!("Deleted note: {} @ {}ms", key.to_uppercase(), snapped_time);
                            }
                        }

                        if resp.clicked() {
                            let timing_points = meta_timing_points_sorted(&state.meta);
                            let snapped_time = snap_time_to_beat_divisor_ms(&timing_points, state.current_time, 2);

                            if state.is_hold_mode {
                                if let Some(start) = state.hold_starts.remove(&key_lower) {
                                    // Add hold note when toggled off
                                    let end = snapped_time.max(start);
                                    state.beatmap.add_hold_note(key_lower.clone(), start, end);
                                } else {
                                    // Remember start time when toggled on
                                    state.hold_starts.insert(key_lower.clone(), snapped_time);
                                }
                            } else {
                                state.beatmap.add_tap_note(key_lower, snapped_time);
                            }
                        }
                    }
                });
            }

            state.hovered_key = hovered_key;
        });
    });
}
