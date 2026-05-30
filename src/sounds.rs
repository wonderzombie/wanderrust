use std::collections::HashMap;

use bevy::{
    asset::LoadedFolder,
    audio::{PlaybackMode, Volume},
    prelude::*,
};

use crate::{
    actors::{Bonk, Moved, Player},
    combat::Attacked,
};

#[derive(Resource, Default)]
pub struct Sounds {
    lookup: HashMap<String, Handle<AudioSource>>,
    folder_handle: Handle<LoadedFolder>,
    pub loaded: bool,
}

const DEFAULT_SOUND_VOL: f32 = 1.;

pub fn load_sounds(mut sounds: ResMut<Sounds>, asset_server: Res<AssetServer>) {
    info!("🔈 preparing to load sounds");
    let handle = asset_server.load_folder("audio");

    *sounds = Sounds {
        folder_handle: handle,
        loaded: false,
        ..default()
    };
}

pub fn on_loaded(
    mut commands: Commands,
    mut sounds: ResMut<Sounds>,
    loaded_folders: Res<Assets<LoadedFolder>>,
    asset_server: Res<AssetServer>,
) {
    if sounds.loaded {
        return;
    }

    let Some(folder) = loaded_folders.get(&sounds.folder_handle) else {
        return;
    };

    info!("🔈 sounds loaded & accessible; initializing");
    sounds.lookup = folder
        .handles
        .iter()
        .filter_map(|handle| {
            let audio_handle = handle.clone().try_typed::<AudioSource>().ok()?;
            let path = asset_server.get_path(handle.id())?;
            let name = path.path().file_stem()?.to_string_lossy().into_owned();
            trace!("sound: {name:?} handle {audio_handle:?}");
            Some((name, audio_handle))
        })
        .collect();

    sounds.loaded = true;

    commands.add_observer(on_walk_sound);
    commands.add_observer(on_bonk_sound);
    commands.add_observer(on_attack_sound);
    commands.add_observer(on_acquired_sound);
    info!("🔈 finished initializing sounds");
}

fn on_bonk_sound(_on: On<Bonk>, mut commands: Commands, sounds: Res<Sounds>) {
    if let Some(s) = sounds.lookup.get("bonk") {
        commands.spawn(one_off_sound_bundle(s));
    }
}

fn on_walk_sound(_on: On<Moved>, mut commands: Commands, sounds: Res<Sounds>) {
    if let Some(s) = sounds.lookup.get("step") {
        commands.spawn(one_off_sound_bundle(s));
    }
}

fn on_attack_sound(
    on: On<Attacked>,
    mut commands: Commands,
    sounds: Res<Sounds>,
    player: Single<Entity, With<Player>>,
) {
    let sound = if *player == on.0 {
        "player_hurt"
    } else {
        "enemy_hurt"
    };
    if let Some(s) = sounds.lookup.get(sound) {
        commands.spawn(one_off_sound_bundle(s));
    }
}

#[derive(Event, Debug)]
pub(crate) struct Opened;

fn on_acquired_sound(_on: On<Opened>, mut commands: Commands, sounds: Res<Sounds>) {
    if let Some(s) = sounds.lookup.get("open") {
        commands.spawn(one_off_sound_bundle(s));
    }
}

fn one_off_sound_bundle(handle: &Handle<AudioSource>) -> impl Bundle {
    (
        AudioPlayer::new(handle.clone()),
        PlaybackSettings {
            mode: PlaybackMode::Despawn,
            volume: Volume::Linear(DEFAULT_SOUND_VOL),
            ..default()
        },
    )
}
