use std::collections::HashMap;

use bevy::{
    asset::LoadedFolder,
    audio::{PlaybackMode, Volume},
    prelude::*,
};
use rand::seq::IndexedRandom;

use crate::actors::Moved;

#[derive(Resource, Default)]
pub struct Sounds {
    lookup: HashMap<String, Handle<AudioSource>>,
    folder: Handle<LoadedFolder>,
    loaded: bool,
}

pub fn load_sounds(mut sounds: ResMut<Sounds>, asset_server: Res<AssetServer>) {
    info!("preparing to load sounds");
    let handle = asset_server.load_folder("audio");

    *sounds = Sounds {
        folder: handle,
        loaded: false,
        ..default()
    };
}

pub fn on_sounds_loaded(
    mut commands: Commands,
    mut sounds: ResMut<Sounds>,
    loaded_folders: Res<Assets<LoadedFolder>>,
    asset_server: Res<AssetServer>,
) {
    if sounds.loaded {
        return;
    }

    let handle = asset_server.load_folder("audio");

    let Some(folder) = loaded_folders.get(&handle) else {
        info!("Sounds not ready");
        return;
    };

    info!("sounds loaded; initializing");
    sounds.lookup = folder
        .handles
        .iter()
        .filter_map(|handle| {
            let audio_handle = handle.clone().try_typed::<AudioSource>().ok()?;
            let path = asset_server.get_path(handle.id())?;
            let name = path.path().file_stem()?.to_string_lossy().into_owned();
            Some((name, audio_handle))
        })
        .collect();

    sounds.loaded = true;
    sounds.folder = handle;

    commands.add_observer(on_moved_sounds);

    info!("finished initializing sounds");
}

const GRASS_FOOTSTEPS: [&str; 5] = [
    "footstep_grass_000",
    "footstep_grass_001",
    "footstep_grass_002",
    "footstep_grass_003",
    "footstep_grass_004",
];

fn on_moved_sounds(_on: On<Moved>, mut commands: Commands, sounds: Res<Sounds>) {
    let mut rng = rand::rng();

    let rand_footstep: &'static str = GRASS_FOOTSTEPS.choose(&mut rng).unwrap();
    let Some(footstep) = sounds.lookup.get(rand_footstep) else {
        error!("footstep sound not found: {}", rand_footstep);
        return;
    };

    commands.spawn((
        AudioPlayer::new(footstep.clone()),
        PlaybackSettings {
            mode: PlaybackMode::Despawn,
            volume: Volume::Linear(0.1),
            ..default()
        },
    ));
}
