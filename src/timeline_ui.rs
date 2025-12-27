use crate::{
    editor_state::EditorState,
    timing_util::{beat_len_ms, meta_timing_points_sorted, snap_time_to_beat_divisor_ms, timing_point_at},
};
use bevy_egui::egui;

pub fn draw_timeline(ui: &mut egui::Ui, ctx: &egui::Context, state: &mut EditorState) {
    ui.group(|ui| {
        ui.heading("⏱️ Timeline");

        let row_spacing = state.timeline_row_spacing.clamp(16.0, 80.0);
        let timeline_height = 34.0 + row_spacing * 2.0 + 20.0;
        let available_width = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(
            egui::Vec2::new(available_width, timeline_height),
            egui::Sense::click_and_drag(),
        );

        let window_ms = state.timeline_window_ms.max(1000.0);
        let ms_per_pixel = window_ms / available_width.max(1.0);
        let pixels_per_ms = 1.0 / ms_per_pixel;

        // View starts such that playhead is at a fixed ratio in the timeline
        let playhead_ratio = state.timeline_playhead_ratio.clamp(0.05, 0.95);
        let desired_view_start_ms = (state.current_time as f32) - window_ms * playhead_ratio;
        let (view_start_ms, playhead_x) = if desired_view_start_ms < 0.0 {
            (0.0, rect.left() + (state.current_time as f32) * pixels_per_ms)
        } else {
            (desired_view_start_ms, rect.left() + available_width * playhead_ratio)
        };
        let view_end_ms = view_start_ms + window_ms;

        if response.hovered() {
            let modifiers = ctx.input(|i| i.modifiers);
            let scroll = ctx.input(|i| i.raw_scroll_delta);
            // Windows turns shift+scroll into horizontal scroll (x-axis)
            // Use the dominant axis so the gesture still works
            let wheel = if scroll.y.abs() >= scroll.x.abs() {
                scroll.y
            } else {
                scroll.x
            };
            // Windows turns Ctrl+scroll into zoom
            let zoom = ctx.input(|i| i.zoom_delta());

            if modifiers.ctrl {
                let mut changed = false;

                if (zoom - 1.0).abs() > 1e-6 {
                    // zoom > 1.0 means "scroll up"; zoom < 1.0 means "scroll down".
                    let delta = (zoom.ln() * 50.0).clamp(-12.0, 12.0);
                    state.timeline_row_spacing = (state.timeline_row_spacing + delta).clamp(16.0, 80.0);
                    changed = true;
                } else if wheel.abs() > 0.0 {
                    // Fallback path for platforms that still provide wheel delta.
                    let step = ((wheel.abs() / 40.0).round() as i32).clamp(1, 8) as f32;
                    if wheel > 0.0 {
                        state.timeline_row_spacing += 2.0 * step;
                    } else {
                        state.timeline_row_spacing -= 2.0 * step;
                    }
                    state.timeline_row_spacing = state.timeline_row_spacing.clamp(16.0, 80.0);
                    changed = true;
                }

                if changed {
                    ctx.request_repaint();
                }
            } else if modifiers.shift {
                if wheel.abs() > 0.0 {
                    // Zoom by scaling the visible window (smaller window = zoom in = notes further apart).
                    let notches = ((wheel.abs() / 40.0).round() as i32).clamp(1, 8);
                    for _ in 0..notches {
                        if wheel > 0.0 {
                            state.timeline_window_ms *= 0.9;
                        } else {
                            state.timeline_window_ms *= 1.1;
                        }
                    }
                    state.timeline_window_ms = state.timeline_window_ms.clamp(1_000.0, 300_000.0);
                    ctx.request_repaint();
                }
            } else {
                if wheel.abs() > 0.0 {
                    // Scrub by 1/8 notes (half-beats) so scrolling matches the visible grid.
                    // If we're not already on the grid, snap first.
                    let timing_points = meta_timing_points_sorted(&state.meta);
                    let snapped_start = snap_time_to_beat_divisor_ms(&timing_points, state.current_time, 2);
                    let tp = timing_point_at(&timing_points, snapped_start);
                    let step_ms = (beat_len_ms(&tp) * 0.5).round().max(1.0) as i64;

                    // Normalize wheel delta into discrete notches
                    let notches = ((wheel.abs() / 40.0).round() as i64).clamp(1, 16);
                    let dir = if wheel > 0.0 { -1 } else { 1 };
                    let delta_ms = dir * step_ms * notches;

                    let new_time = snapped_start as i64 + delta_ms;
                    let unsnapped = new_time.max(0) as u32;
                    state.current_time = snap_time_to_beat_divisor_ms(&timing_points, unsnapped, 2);
                    state.audio_seek_request = Some(state.current_time);
                    ctx.request_repaint();
                }
            }
        }

        // Click seeks to the clicked time
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let t = view_start_ms + (pos.x - rect.left()) * ms_per_pixel;
                state.current_time = t.max(0.0).round() as u32;
                state.audio_seek_request = Some(state.current_time);
            }
        }

        // Drag scrubs time (horizontal)
        if response.drag_started() {
            state.timeline_drag_last_dx = Some(0.0);
        }
        if response.dragged() {
            let total_dx = response.drag_delta().x;
            let last_dx = state.timeline_drag_last_dx.unwrap_or(0.0);
            let frame_dx = total_dx - last_dx;
            state.timeline_drag_last_dx = Some(total_dx);

            let delta_ms = (-frame_dx * ms_per_pixel).round() as i64;
            let new_time = state.current_time as i64 + delta_ms;
            state.current_time = new_time.max(0) as u32;
            state.audio_seek_request = Some(state.current_time);
        }
        if response.drag_stopped() {
            state.timeline_drag_last_dx = None;
        }

        // Clip all drawing to the timeline area so it never overlaps other panels
        let painter = ui.painter_at(rect);

        // Draw timeline background
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(30, 30, 30));

        // Draw timeline from meta timing points
        let timing_points = meta_timing_points_sorted(&state.meta);
        let active_tp = timing_point_at(&timing_points, state.current_time);
        let bpm: f32 = (active_tp.bpm as f32).max(1.0);

        painter.text(
            egui::pos2(rect.left() + 6.0, rect.top() + 2.0),
            egui::Align2::LEFT_TOP,
            format!(
                "BPM {:.2}  TS {}/{}  Offset {}ms",
                bpm,
                active_tp.timeSignature[0],
                active_tp.timeSignature[1],
                active_tp.offset
            ),
            egui::FontId::monospace(11.0),
            egui::Color32::from_rgb(200, 200, 200),
        );

        // Draw lines for each timing segment that overlaps the view
        for (idx, tp) in timing_points.iter().enumerate() {
            let seg_start = tp.offset as f32;
            let seg_end = timing_points
                .get(idx + 1)
                .map(|n| n.offset as f32)
                .unwrap_or(f32::INFINITY);

            let draw_start = seg_start.max(view_start_ms);
            let draw_end = seg_end.min(view_end_ms);
            if draw_end <= draw_start {
                continue;
            }

            let bpm: f32 = (tp.bpm as f32).max(1.0);
            // We interpret BPM as quarter-notes per minute and derive beat length from the
            // time signature denominator so timing points can change the timeline.
            let quarter_len: f32 = 60_000.0 / bpm;
            let numer = tp.timeSignature[0].max(1) as f32;
            let denom = tp.timeSignature[1].max(1) as f32;
            let beat_len: f32 = (quarter_len * (4.0 / denom)).max(1.0);
            let measure_len: f32 = beat_len * numer;

            let sub: f32 = (beat_len * 0.5).max(1.0);

            let mut t = {
                let k = ((draw_start - seg_start) / sub).ceil();
                seg_start + k * sub
            };

            while t <= draw_end + 0.5 {
                let x = rect.left() + (t - view_start_ms) * pixels_per_ms;
                if x >= rect.left() && x <= rect.right() {
                    let rel = (t - seg_start).max(0.0);
                    let rel_m = rel % measure_len;
                    let rel_b = rel % beat_len;
                    let is_measure = rel_m < 0.5 || (measure_len - rel_m) < 0.5;
                    let is_beat = rel_b < 0.5 || (beat_len - rel_b) < 0.5;

                    let stroke = if is_measure {
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(95, 95, 95))
                    } else if is_beat {
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(75, 75, 75))
                    } else {
                        // 1/8 notes
                        egui::Stroke::new(0.75, egui::Color32::from_rgb(60, 60, 60))
                    };

                    let header_h = 18.0;
                    let content_top = rect.top() + header_h;
                    let content_bottom = rect.bottom();

                    let (y0, y1) = if is_measure {
                        (rect.top(), rect.bottom())
                    } else {
                        let content_h = (content_bottom - content_top).max(1.0);
                        let line_frac = if is_beat { 0.60 } else { 0.35 };
                        let min_h = if is_beat { 10.0 } else { 6.0 };
                        let line_h = (content_h * line_frac).clamp(min_h, content_h);
                        let mid = (content_top + content_bottom) * 0.5;
                        let mut y0 = mid - line_h * 0.5;
                        let mut y1 = mid + line_h * 0.5;
                        y0 = y0.max(content_top);
                        y1 = y1.min(content_bottom);
                        (y0, y1)
                    };

                    painter.line_segment([egui::pos2(x, y0), egui::pos2(x, y1)], stroke);
                }
                t += sub;
            }
        }

        // Helper for note vertical placement
        let key_row = |k: &str| -> usize {
            match k {
                "q" | "w" | "e" | "r" | "t" | "y" | "u" | "i" | "o" | "p" => 0,
                "a" | "s" | "d" | "f" | "g" | "h" | "j" | "k" | "l" | ";" => 1,
                _ => 2,
            }
        };

        let row_y = |row: usize| -> f32 { (rect.top() + 34.0) + row as f32 * row_spacing };

        // Draw notes
        for note in &state.beatmap.notes {
            let start_time = note.get_start_time() as f32;
            let end_time = note.get_end_time() as f32;

            // Quick cull
            if end_time < view_start_ms || start_time > view_end_ms {
                continue;
            }

            let x_start = rect.left() + (start_time - view_start_ms) * pixels_per_ms;
            let x_end = rect.left() + (end_time - view_start_ms) * pixels_per_ms;

            let row = key_row(note.key.as_str());
            let y = row_y(row);

            let color = egui::Color32::from_rgb(180, 180, 180);

            if note.note_type == "hold" {
                let r = egui::Rect::from_min_max(
                    egui::pos2(x_start, y - 6.0),
                    egui::pos2(x_end.max(x_start + 2.0), y + 6.0),
                );
                painter.rect_filled(r, 3.0, color);
            } else {
                painter.circle_filled(egui::pos2(x_start, y), 5.0, color);
            }

            // key label
            painter.text(
                egui::pos2(x_start + 6.0, y - 12.0),
                egui::Align2::LEFT_TOP,
                note.key.to_uppercase(),
                egui::FontId::monospace(10.0),
                egui::Color32::from_rgb(210, 210, 210),
            );
        }

        // show any keys that are currently toggled on
        if !state.hold_starts.is_empty() {
            let snapped_now = snap_time_to_beat_divisor_ms(&timing_points, state.current_time, 2) as f32;
            let hold_color = egui::Color32::from_rgba_premultiplied(200, 200, 200, 110);

            for (key, &start_ms) in state.hold_starts.iter() {
                let s = start_ms as f32;
                let e = snapped_now.max(s);

                // Cull if entirely out of view.
                if e < view_start_ms || s > view_end_ms {
                    continue;
                }

                let x0 = rect.left() + (s - view_start_ms) * pixels_per_ms;
                let x1 = rect.left() + (e - view_start_ms) * pixels_per_ms;
                let row = key_row(key.as_str());
                let y = row_y(row);

                let r = egui::Rect::from_min_max(
                    egui::pos2(x0, y - 6.0),
                    egui::pos2(x1.max(x0 + 2.0), y + 6.0),
                );
                painter.rect_filled(r, 3.0, hold_color);

                // key label at start
                painter.text(
                    egui::pos2(x0 + 6.0, y - 12.0),
                    egui::Align2::LEFT_TOP,
                    key.to_uppercase(),
                    egui::FontId::monospace(10.0),
                    egui::Color32::from_rgba_premultiplied(210, 210, 210, 150),
                );
            }
        }

        // show where a click would place a note
        if let Some(hover_key) = state.hovered_key.as_deref() {
            let snapped_time = snap_time_to_beat_divisor_ms(&timing_points, state.current_time, 2) as f32;

            let ghost_color = egui::Color32::from_rgba_premultiplied(200, 200, 200, 90);

            if state.is_hold_mode {
                if let Some(&hold_start) = state.hold_starts.get(hover_key) {
                    let s = hold_start as f32;
                    let e = snapped_time.max(s);

                    if e >= view_start_ms && s <= view_end_ms {
                        let x0 = rect.left() + (s - view_start_ms) * pixels_per_ms;
                        let x1 = rect.left() + (e - view_start_ms) * pixels_per_ms;
                        let row = key_row(hover_key);
                        let y = row_y(row);
                        let r = egui::Rect::from_min_max(
                            egui::pos2(x0, y - 6.0),
                            egui::pos2(x1.max(x0 + 2.0), y + 6.0),
                        );
                        painter.rect_filled(r, 3.0, ghost_color);
                    }
                } else {
                    if snapped_time >= view_start_ms && snapped_time <= view_end_ms {
                        let x = rect.left() + (snapped_time - view_start_ms) * pixels_per_ms;
                        let row = key_row(hover_key);
                        let y = row_y(row);
                        painter.circle_filled(egui::pos2(x, y), 5.0, ghost_color);
                    }
                }
            } else {
                if snapped_time >= view_start_ms && snapped_time <= view_end_ms {
                    let x = rect.left() + (snapped_time - view_start_ms) * pixels_per_ms;
                    let row = key_row(hover_key);
                    let y = row_y(row);
                    painter.circle_filled(egui::pos2(x, y), 5.0, ghost_color);
                }
            }
        }

        // Draw fixed playhead
        painter.line_segment(
            [egui::pos2(playhead_x, rect.top()), egui::pos2(playhead_x, rect.bottom())],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 0)),
        );

        painter.text(
            egui::pos2(playhead_x + 4.0, rect.bottom() - 14.0),
            egui::Align2::LEFT_BOTTOM,
            format!("{}ms", state.current_time),
            egui::FontId::monospace(10.0),
            egui::Color32::from_rgb(0, 255, 0),
        );
    });
}
