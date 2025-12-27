mod data;
mod loader;
mod editor_state;
mod timing_util;
mod audio_util;
mod timeline_ui;
mod keyboard_ui;
mod ui;

use bevy::prelude::*;
use bevy::asset::AssetPlugin;
use bevy_kira_audio::AudioPlugin as KiraAudioPlugin;
use editor_state::EditorState;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .build()
                .set(AssetPlugin {
                    file_path: ".".to_string(),
                    ..default()
                })
                // We use bevy_kira_audio for playback/seek so we disable Bevy's built-in audio plugin
                // to avoid duplicate AudioSource asset loader warnings.
                .disable::<bevy::audio::AudioPlugin>(),
        )
        .add_plugins(KiraAudioPlugin)
        .add_plugins(bevy_egui::EguiPlugin)
        .init_resource::<EditorState>()
        .add_systems(Startup, ui::setup)
        .add_systems(Update, ui::ui_system)
        .run();
}