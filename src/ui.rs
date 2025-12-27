use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioTween};

use crate::{
    audio_util,
    data,
    editor_state::{EditorState, AUDIO_POLL_INTERVAL_S},
    keyboard_ui,
    loader,
    timeline_ui,
};

pub fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

pub fn ui_system(
    mut contexts: EguiContexts,
    asset_server: Res<AssetServer>,
    audio: Res<Audio>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    mut state: ResMut<EditorState>,
    time: Res<Time>,
) {
    let ctx = contexts.ctx_mut();

    let prev_audio_file = state.meta.audioFile.clone();
    let prev_bpm = state.meta.bpm;
    let prev_offset = state.meta.offset;
    let prev_beatmap_name = state.beatmap.name.clone();
    let prev_od = state.beatmap.overallDifficulty;

    let mut beatmap_settings_changed = false;
    let mut meta_settings_changed = false;
    let mut timing_points_changed = false;
    let mut audio_file_changed = false;

    if state.is_playing {
        ctx.request_repaint();
    }

    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui.button("📦 Load .rtm").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("Map", &["rtm"]).pick_file() {
                    match loader::load_rtm(&path) {
                        Ok(pkg) => {
                            let diff_count = pkg.difficulties.len();
                            state.meta = pkg.meta.clone();
                            state.rtm_package = Some(pkg);
                            state.rtm_file_path = Some(path.clone());
                            state.selected_difficulty = 0;

                            if let Some(pkg) = &state.rtm_package {
                                if let Some(first) = pkg.difficulties.get(0) {
                                    state.beatmap = first.beatmap.clone();
                                }
                            }

                            state.current_time = 0;
                            state.is_playing = false;
                            state.is_hold_mode = false;
                            state.hold_starts.clear();
                            state.audio_rel_path = None;
                            state.audio_handle = None;
                            state.audio_seek_request = Some(0);
                            state.audio_instance = None;
                            state.status = format!(
                                "Loaded rtm: {} ({} diffs)",
                                path.file_name().and_then(|s| s.to_str()).unwrap_or("<file>"),
                                diff_count
                            );
                        }
                        Err(err) => state.status = format!("Load .rtm failed: {}", err),
                    }
                }
            }

            let has_pkg = state.rtm_package.is_some();
            if has_pkg {
                // Difficulty dropdown
                let mut next_idx = state.selected_difficulty;
                egui::ComboBox::from_label("Difficulty")
                    .selected_text({
                        state
                            .rtm_package
                            .as_ref()
                            .and_then(|p| p.difficulties.get(state.selected_difficulty))
                            .map(|d| d.meta.name.clone())
                            .unwrap_or_else(|| "<none>".to_string())
                    })
                    .show_ui(ui, |ui| {
                        if let Some(pkg) = &state.rtm_package {
                            for (i, d) in pkg.difficulties.iter().enumerate() {
                                ui.selectable_value(&mut next_idx, i, d.meta.name.clone());
                            }
                        }
                    });

                if next_idx != state.selected_difficulty {
                    // Save current edits into the currently selected difficulty, then swap in the new one
                    let old_idx = state.selected_difficulty;
                    let beatmap_snapshot = state.beatmap.clone();
                    if let Some(pkg) = state.rtm_package.as_mut() {
                        if let Some(cur) = pkg.difficulties.get_mut(old_idx) {
                            cur.beatmap = beatmap_snapshot;
                        }
                    }
                    state.selected_difficulty = next_idx;
                    if let Some(pkg) = state.rtm_package.as_ref() {
                        if let Some(sel) = pkg.difficulties.get(next_idx) {
                            state.beatmap = sel.beatmap.clone();
                        }
                    }
                    state.status = "Switched difficulty".to_string();
                }

                if ui.button("➕ Import Difficulty").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("Difficulty", &["json"]).pick_file() {
                        match loader::import_difficulty_json(&path) {
                            Ok(beatmap) => {
                                let stem = path
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("difficulty")
                                    .to_string();
                                let mut filename = path
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("difficulty.json")
                                    .to_string();

                                if let Some(pkg) = &mut state.rtm_package {
                                    // Ensure filename uniqueness within the package
                                    let used: std::collections::HashSet<String> = pkg
                                        .difficulties
                                        .iter()
                                        .map(|d| d.meta.filename.clone())
                                        .collect();
                                    if used.contains(&filename) {
                                        let mut n = 2;
                                        let base = stem.clone();
                                        loop {
                                            let candidate = format!("{}-{}.json", base, n);
                                            if !used.contains(&candidate) {
                                                filename = candidate;
                                                break;
                                            }
                                            n += 1;
                                        }
                                    }

                                    let display_name = if !beatmap.name.trim().is_empty() && beatmap.name != "New Beatmap" {
                                        beatmap.name.clone()
                                    } else {
                                        stem
                                    };

                                    pkg.difficulties.push(loader::RtmDifficulty {
                                        meta: data::MetaDifficulty {
                                            name: display_name,
                                            filename: filename.clone(),
                                        },
                                        beatmap,
                                    });
                                    let new_idx = pkg.difficulties.len().saturating_sub(1);
                                    let new_meta_diffs: Vec<data::MetaDifficulty> = pkg
                                        .difficulties
                                        .iter()
                                        .map(|d| d.meta.clone())
                                        .collect();
                                    let new_beatmap = pkg
                                        .difficulties
                                        .get(new_idx)
                                        .map(|d| d.beatmap.clone())
                                        .unwrap_or_else(crate::data::Beatmap::new);

                                    // Keep meta in sync
                                    state.meta.difficulties = new_meta_diffs;
                                    state.selected_difficulty = new_idx;
                                    state.beatmap = new_beatmap;

                                    state.status = format!(
                                        "Imported difficulty: {}",
                                        path.file_name().and_then(|s| s.to_str()).unwrap_or("<file>")
                                    );
                                }
                            }
                            Err(err) => state.status = format!("Import failed: {}", err),
                        }
                    }
                }

                if ui.button("⬇ Export Difficulty .json").clicked() {
                    let suggested_filename = state
                        .rtm_package
                        .as_ref()
                        .and_then(|p| p.difficulties.get(state.selected_difficulty))
                        .map(|d| d.meta.filename.clone())
                        .unwrap_or_else(|| {
                            let name = state.beatmap.name.trim();
                            if name.is_empty() || name == "New Beatmap" {
                                "difficulty.json".to_string()
                            } else {
                                let safe: String = name
                                    .chars()
                                    .map(|c| match c {
                                        '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                                        _ => c,
                                    })
                                    .collect();
                                format!("{}.json", safe)
                            }
                        });

                    if let Some(mut path) = rfd::FileDialog::new()
                        .add_filter("Difficulty", &["json"])
                        .set_file_name(&suggested_filename)
                        .save_file()
                    {
                        if path.extension().and_then(|e| e.to_str()).is_none() {
                            path.set_extension("json");
                        }

                        match loader::save_beatmap(&path, &state.beatmap) {
                            Ok(()) => {
                                state.status = format!(
                                    "Exported difficulty json: {}",
                                    path.file_name().and_then(|s| s.to_str()).unwrap_or("<file>")
                                );
                            }
                            Err(err) => state.status = format!("Export failed: {}", err),
                        }
                    }
                }

                if ui.button("💾 Save .rtm").clicked() {
                    let path = state.rtm_file_path.clone();
                    let idx = state.selected_difficulty;
                    let beatmap_snapshot = state.beatmap.clone();
                    let meta_snapshot = state.meta.clone();
                    if let (Some(pkg), Some(path)) = (state.rtm_package.as_mut(), path) {
                        if let Some(cur) = pkg.difficulties.get_mut(idx) {
                            cur.beatmap = beatmap_snapshot;
                        }
                        pkg.meta = meta_snapshot;
                        match loader::save_rtm(&path, pkg) {
                            Ok(()) => {
                                state.status = format!(
                                    "Saved rtm: {}",
                                    path.file_name().and_then(|s| s.to_str()).unwrap_or("<file>")
                                );
                            }
                            Err(err) => state.status = format!("Save .rtm failed: {}", err),
                        }
                    } else {
                        state.status = "No .rtm loaded".to_string();
                    }
                }
            }

            ui.separator();

            ui.label(format!("Time: {} ms", state.current_time));
            if ui.button("◀ -100ms").clicked() {
                state.current_time = state.current_time.saturating_sub(100);
                state.audio_seek_request = Some(state.current_time);
            }
            if ui.button("▶ +100ms").clicked() {
                state.current_time += 100;
                state.audio_seek_request = Some(state.current_time);
            }

            ui.separator();

            // Playback button
            let play_label = if state.is_playing { "⏸ PLAYING" } else { "▶️ PAUSED" };
            if ui.button(play_label).clicked() {
                let was_playing = state.is_playing;
                state.is_playing = !state.is_playing;

                // When starting playback from the beginning, start 1s before the offset (or 0).
                if !was_playing && state.is_playing && state.current_time == 0 {
                    let offset_ms = state.meta.offset.max(0) as u32;
                    state.current_time = offset_ms.saturating_sub(1000);
                }

                // Ensure audio snaps to editor time on toggle.
                state.audio_seek_request = Some(state.current_time);
            }

            // Space bar to toggle playback
            if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                let was_playing = state.is_playing;
                state.is_playing = !state.is_playing;

                // When starting playback from the beginning, start 1s before the offset (or 0)
                if !was_playing && state.is_playing && state.current_time == 0 {
                    let offset_ms = state.meta.offset.max(0) as u32;
                    state.current_time = offset_ms.saturating_sub(1000);
                }

                state.audio_seek_request = Some(state.current_time);
            }

            ui.separator();

            let mode_label = if state.is_hold_mode { "HOLD MODE" } else { "TAP MODE" };
            if ui.button(format!("🎯 {}", mode_label)).clicked() {
                state.is_hold_mode = !state.is_hold_mode;
            }

            ui.label(format!("Notes: {}", state.beatmap.notes.len()));

            ui.separator();
            ui.label(&state.status);
        });
    });

    // Sidebar for settings
    egui::SidePanel::right("sidebar")
        .resizable(true)
        .default_width(320.0)
        .min_width(260.0)
        .frame(
            egui::Frame::default()
                .fill(egui::Color32::from_rgb(30, 30, 30))
                .stroke(egui::Stroke::NONE)
                .inner_margin(egui::Margin::same(12.0))
                .outer_margin(egui::Margin::same(0.0)),
        )
        .show(ctx, |ui| {
            ui.heading("Settings");

            ui.group(|ui| {
                ui.heading("Beatmap Settings");
                if ui.text_edit_singleline(&mut state.beatmap.name).changed() {
                    beatmap_settings_changed = true;
                }
                ui.label("Name");

                let mut diff_val = state.beatmap.overallDifficulty;
                if ui.add(egui::Slider::new(&mut diff_val, 0.0..=10.0).text("Overall Difficulty")).changed() {
                    beatmap_settings_changed = true;
                }
                state.beatmap.overallDifficulty = diff_val.clamp(0.0, 10.0);
            });

            ui.separator();

            ui.group(|ui| {
                ui.heading("Meta (meta.json)");

                ui.label("Song Name");
                if ui.text_edit_singleline(&mut state.meta.songName).changed() {
                    meta_settings_changed = true;
                }

                ui.label("Artist Name");
                if ui.text_edit_singleline(&mut state.meta.artistName).changed() {
                    meta_settings_changed = true;
                }

                ui.label("Mapper");
                if ui.text_edit_singleline(&mut state.meta.mapper).changed() {
                    meta_settings_changed = true;
                }

                ui.label("Audio File");
                if ui.text_edit_singleline(&mut state.meta.audioFile).changed() {
                    meta_settings_changed = true;
                    audio_file_changed = true;
                }

                if state.meta.timingPoints.is_empty() {
                    ui.label("BPM");
                    if ui.add(egui::DragValue::new(&mut state.meta.bpm).speed(0.1)).changed() {
                        meta_settings_changed = true;
                    }

                    ui.label("Offset (ms)");
                    if ui.add(egui::DragValue::new(&mut state.meta.offset).speed(1)).changed() {
                        meta_settings_changed = true;
                    }
                } else {
                    let now_ms = state.current_time as i64;
                    let mut active_idx: usize = 0;
                    let mut best_offset: i64 = i64::MIN;
                    for (i, tp) in state.meta.timingPoints.iter().enumerate() {
                        if tp.offset <= now_ms && tp.offset >= best_offset {
                            best_offset = tp.offset;
                            active_idx = i;
                        }
                    }

                    ui.separator();
                    ui.heading("Active Timing Point");
                    ui.label(format!("Index: {}", active_idx));

                    let tp = &mut state.meta.timingPoints[active_idx];

                    ui.label("BPM");
                    if ui.add(egui::DragValue::new(&mut tp.bpm).speed(0.1)).changed() {
                        timing_points_changed = true;
                    }

                    ui.label("Offset (ms)");
                    if ui.add(egui::DragValue::new(&mut tp.offset).speed(1)).changed() {
                        timing_points_changed = true;
                    }

                    ui.label("Time Signature");
                    let mut numer = tp.timeSignature[0];
                    let mut denom = tp.timeSignature[1];
                    ui.horizontal(|ui| {
                        ui.label("Numer");
                        if ui.add(egui::DragValue::new(&mut numer).speed(1)).changed() {
                            timing_points_changed = true;
                        }
                        ui.label("Denom");
                        if ui.add(egui::DragValue::new(&mut denom).speed(1)).changed() {
                            timing_points_changed = true;
                        }
                    });
                    numer = numer.max(1);
                    denom = denom.max(1);
                    tp.timeSignature = [numer, denom];
                }
            });
        });

    if timing_points_changed {
        state.meta.timingPoints.sort_by(|a, b| a.offset.cmp(&b.offset));
        if let Some(first) = state.meta.timingPoints.first() {
            let first_bpm = first.bpm;
            let first_offset = first.offset;
            state.meta.bpm = first_bpm;
            state.meta.offset = first_offset;
            meta_settings_changed = true;
        }
    }

    let settings_changed = beatmap_settings_changed
        || meta_settings_changed
        || timing_points_changed
        || prev_audio_file != state.meta.audioFile
        || (prev_bpm - state.meta.bpm).abs() > f64::EPSILON
        || prev_offset != state.meta.offset
        || prev_beatmap_name != state.beatmap.name
        || (prev_od - state.beatmap.overallDifficulty).abs() > f32::EPSILON;

    if settings_changed {
        ctx.request_repaint();

        if audio_file_changed || prev_audio_file != state.meta.audioFile {
            state.audio_rel_path = None;
            state.audio_handle = None;
            state.audio_instance = None;
            state.audio_seek_request = Some(state.current_time);
            state.status = "Audio file changed (reloading audio)".to_string();
        }

        if state.is_playing && (prev_offset != state.meta.offset || (prev_bpm - state.meta.bpm).abs() > f64::EPSILON) {
            state.audio_seek_request = Some(state.current_time);
        }
    }

    egui::CentralPanel::default()
        .frame(
            egui::Frame::default()
                .fill(egui::Color32::from_rgb(30, 30, 30))
                .stroke(egui::Stroke::NONE)
                .inner_margin(egui::Margin::same(6.0))
                .outer_margin(egui::Margin::same(0.0)),
        )
        .show(ctx, |ui| {
            timeline_ui::draw_timeline(ui, ctx, &mut state);

            ui.separator();

            let keyboard_size = ui.available_size();
            let (keyboard_rect, _) = ui.allocate_exact_size(keyboard_size, egui::Sense::hover());
            keyboard_ui::draw_keyboard(ui, &mut state, keyboard_rect);
        });

    if state.is_playing {
        let dt_ms = (time.delta_seconds_f64() * 1000.0).max(0.0);
        state.current_time = state.current_time.saturating_add(dt_ms.round() as u32);
    }

    // Audio sync
    if let (Some(pkg), Some(rtm_path)) = (state.rtm_package.as_ref(), state.rtm_file_path.as_ref()) {
        if state.audio_rel_path.is_none() {
            match audio_util::ensure_audio_extracted(pkg, rtm_path) {
                Ok(Some(rel)) => state.audio_rel_path = Some(rel),
                Ok(None) => {}
                Err(err) => state.status = format!("Audio extract failed: {}", err),
            }
        }
        if state.audio_handle.is_none() {
            if let Some(rel) = state.audio_rel_path.clone() {
                state.audio_handle = Some(asset_server.load(rel));
            }
        }
    }

    let should_have_instance = state.is_playing || state.audio_seek_request.is_some();
    if should_have_instance && state.audio_instance.is_none() {
        if let Some(handle) = state.audio_handle.clone() {
            let instance = audio.play(handle).handle();
            state.audio_instance = Some(instance);
        }
    }

    if let Some(instance_handle) = state.audio_instance.clone() {
        if let Some(instance) = audio_instances.get_mut(&instance_handle) {
            let mut did_seek_this_frame = false;
            if let Some(ms) = state.audio_seek_request.take() {
                let seconds = (ms as f64) / 1000.0;
                debug!("Audio seek_to: {seconds:.3}s");
                instance.seek_to(seconds);
                did_seek_this_frame = true;
            }

            if state.is_playing {
                instance.resume(AudioTween::default());
            } else {
                instance.pause(AudioTween::default());
            }

            // Periodically re-sync audio to the editor clock.
            // bevy_kira_audio doesn't expose a simple "current position" getter for us here, so the
            // safest/cheapest "poll" we can do is periodically seeking to the editor's current_time.
            if state.is_playing {
                if did_seek_this_frame {
                    state.audio_poll_accum_s = 0.0;
                } else {
                    state.audio_poll_accum_s += time.delta_seconds();
                    if state.audio_poll_accum_s >= AUDIO_POLL_INTERVAL_S {
                        state.audio_poll_accum_s = 0.0;
                        let seconds = (state.current_time as f64) / 1000.0;
                        debug!("Periodic audio resync seek_to: {seconds:.3}s");
                        instance.seek_to(seconds);
                    }
                }
            } else {
                // Reset while paused
                state.audio_poll_accum_s = 0.0;
            }
        }
    }
}
