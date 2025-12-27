use bevy::prelude::*;

use crate::{data::{Beatmap, Meta}, loader::RtmPackage};
use std::collections::HashMap;
use std::path::PathBuf;

pub const AUDIO_POLL_INTERVAL_S: f32 = 0.5;

#[derive(Resource)]
pub struct EditorState {
    pub beatmap: Beatmap,
    pub meta: Meta,
    pub rtm_package: Option<RtmPackage>,
    pub rtm_file_path: Option<PathBuf>,
    pub selected_difficulty: usize,

    pub audio_rel_path: Option<String>,
    pub audio_handle: Option<Handle<bevy_kira_audio::AudioSource>>,
    pub audio_seek_request: Option<u32>,
    pub audio_instance: Option<Handle<bevy_kira_audio::AudioInstance>>,
    pub audio_poll_accum_s: f32,

    pub current_time: u32,
    pub is_hold_mode: bool,
    pub hold_starts: HashMap<String, u32>,
    pub is_playing: bool,

    pub timeline_window_ms: f32,
    pub timeline_playhead_ratio: f32,
    pub timeline_row_spacing: f32,
    pub timeline_drag_last_dx: Option<f32>,

    pub hovered_key: Option<String>,

    pub status: String,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            beatmap: Beatmap::new(),
            meta: Meta::default(),
            rtm_package: None,
            rtm_file_path: None,
            selected_difficulty: 0,

            audio_rel_path: None,
            audio_handle: None,
            audio_seek_request: None,
            audio_instance: None,
            audio_poll_accum_s: 0.0,

            current_time: 0,
            is_hold_mode: false,
            hold_starts: HashMap::new(),
            is_playing: false,

            timeline_window_ms: 10_000.0,
            timeline_playhead_ratio: 0.4,
            timeline_row_spacing: 30.0,
            timeline_drag_last_dx: None,

            hovered_key: None,

            status: "Ready".to_string(),
        }
    }
}
