use std::collections::HashMap;

use bevy::{
    asset::LoadedFolder,
    audio::{PlaybackMode, Volume},
    prelude::*,
};


#[derive(Resource, Default, Deref)]
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
            debug!("sound: {name:?} handle {audio_handle:?}");
            Some((name, audio_handle))
        })
        .collect();

    info!("🔈 sounds loaded: {}", lookup.len());

    commands.insert_resource(Sounds(lookup));
    commands.add_observer(on_sound_triggered);

    info!("🔈 finished initializing sounds");
}

macro_rules! define_sounds {
    ( $( $id:ident => $name:expr $(,)? )+ ) => {
        #[derive(Event, Debug)]
        pub(crate) enum SoundEvent {
            $( $id, )+
        }

        impl SoundEvent {
            pub(crate) fn name(&self) -> &'static str {
                match self {
                    $( Self::$id => $name,  )+
                }
            }
        }
    };
}

define_sounds! {
    EnemyDefeated => "enemy_defeated",
    EquipOff => "unequip",
    EquipOn => "equip",
    HurtEnemy => "enemy_hurt",
    HurtPlayer => "player_hurt",
    // LitShrine => "lit_shrine",
    Opened => "open",
    Quaff => "quaff",
    // Rest => "rest",
    Walk => "step",
}

fn on_sound_triggered(on: On<SoundEvent>, sounds: Res<Sounds>, mut commands: Commands) {
    if let Some(handle) = sounds.get(on.event().name()) {
        commands.spawn((
            AudioPlayer::new(handle.clone()),
            PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: Volume::Linear(DEFAULT_SOUND_VOL),
                ..default()
            },
        ));
    }
}
