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
pub struct Sounds(HashMap<String, Handle<AudioSource>>);

const DEFAULT_SOUND_VOL: f32 = 1.;

#[derive(Resource, Deref)]
pub struct SoundFolder(Handle<LoadedFolder>);

pub fn load_sounds(mut commands: Commands, asset_server: Res<AssetServer>) {
    info!("🔈 preparing to load sounds");
    let handle = asset_server.load_folder("audio");
    commands.insert_resource(SoundFolder(handle));
}

pub fn on_loaded(
    mut commands: Commands,
    folder_handle: Res<SoundFolder>,
    loaded_folders: Res<Assets<LoadedFolder>>,
    asset_server: Res<AssetServer>,
) {
    let Some(folder) = loaded_folders.get(folder_handle.id()) else {
        return;
    };

    info!("🔈 sounds loaded & accessible; initializing");
    let lookup: HashMap<String, Handle<AudioSource>> = folder
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

    info!("🔈 sounds loaded: {}", lookup.len());

    commands.insert_resource(Sounds(lookup));

    commands.add_observer(on_walk_sound);
    commands.add_observer(on_bonk_sound);
    commands.add_observer(on_attack_sound);
    commands.add_observer(on_acquired_sound);
    commands.add_observer(on_quaff_sound);
    commands.add_observer(on_equip_sound);
    commands.add_observer(on_unequip_sound);
    commands.add_observer(on_enemy_defeated_sound);
    info!("🔈 finished initializing sounds");
}

fn on_bonk_sound(_on: On<Bonk>, mut commands: Commands, sounds: Res<Sounds>) {
    if let Some(s) = sounds.0.get("bonk") {
        commands.spawn(one_off_sound_bundle(s));
    }
}

fn on_walk_sound(_on: On<Moved>, mut commands: Commands, sounds: Res<Sounds>) {
    if let Some(s) = sounds.0.get("step") {
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
    if let Some(s) = sounds.0.get(sound) {
        commands.spawn(one_off_sound_bundle(s));
    }
}

#[derive(Event, Debug)]
pub(crate) struct Quaffed;

fn on_quaff_sound(_on: On<Quaffed>, mut commands: Commands, sounds: Res<Sounds>) {
    if let Some(s) = sounds.0.get("quaff") {
        commands.spawn(one_off_sound_bundle(s));
    }
}

#[derive(Event, Debug)]
pub(crate) struct Equip;

fn on_equip_sound(_on: On<Equip>, mut commands: Commands, sounds: Res<Sounds>) {
    if let Some(s) = sounds.0.get("equip_01") {
        commands.spawn(one_off_sound_bundle(s));
    }
}

#[derive(Event, Debug)]
pub(crate) struct Unequip;

fn on_unequip_sound(_on: On<Unequip>, mut commands: Commands, sounds: Res<Sounds>) {
    if let Some(s) = sounds.0.get("unequip_01") {
        commands.spawn(one_off_sound_bundle(s));
    }
}

#[derive(Event, Debug)]
pub(crate) struct Opened;

fn on_acquired_sound(_on: On<Opened>, mut commands: Commands, sounds: Res<Sounds>) {
    if let Some(s) = sounds.0.get("open") {
        commands.spawn(one_off_sound_bundle(s));
    }
}

#[derive(Event, Debug)]
pub(crate) struct EnemyDefeated;

fn on_enemy_defeated_sound(_on: On<EnemyDefeated>, mut commands: Commands, sounds: Res<Sounds>) {
    if let Some(s) = sounds.0.get("enemy_defeated") {
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
