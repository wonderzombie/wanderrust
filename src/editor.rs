use std::path::PathBuf;

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures},
};
use rfd::AsyncFileDialog;

use crate::{
    colors::KENNEY_RED,
    event_log,
    player::PlayerStats,
    tilemap::{self, SavedTilemap, TileStorage},
    tiles::{self, Highlighted, MapTile, TileIdx, TilePreview},
};

const DATA_DIR: &str = "data";

#[derive(Resource)]
pub struct EditorState {
    pub active_tile: tiles::TileIdx,
    pub active_tile_idx: usize,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            active_tile: tiles::TileIdx::Grass,
            active_tile_idx: Default::default(),
        }
    }
}

#[derive(Resource)]
pub struct DesiredZoom(pub f32);

pub fn on_zoom_button_input(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    zoom_opt: Option<ResMut<DesiredZoom>>,
) {
    let mut current_zoom = zoom_opt.map_or(1.0, |zoom| zoom.0);

    if input.just_released(KeyCode::Equal) {
        current_zoom += 0.1;
    } else if input.just_released(KeyCode::Minus) {
        current_zoom -= 0.1
    } else if input.just_released(KeyCode::Backspace) {
        current_zoom = 1.0;
    } else if input.just_released(KeyCode::Digit0) {
        current_zoom = 0.1;
    }

    let final_zoom = current_zoom.clamp(0.1, 2.0);
    commands.insert_resource(DesiredZoom(final_zoom));
}

pub fn on_button_input(
    input: Res<ButtonInput<KeyCode>>,
    mut editor_state: ResMut<EditorState>,
    mut log: ResMut<event_log::MessageLog>,
) {
    if !input.is_changed() {
        return;
    }

    let lookup = tiles::TileIdx::all();

    if input.just_pressed(KeyCode::Digit1) {
        editor_state.active_tile_idx = 0;
    } else if input.just_pressed(KeyCode::Digit2) {
        editor_state.active_tile_idx = editor_state.active_tile_idx.saturating_sub(1);
    } else if input.just_pressed(KeyCode::Digit3) {
        editor_state.active_tile_idx = (editor_state.active_tile_idx + 1).min(lookup.len() - 1);
    } else if input.just_pressed(KeyCode::Digit4) {
        editor_state.active_tile_idx = lookup.len() - 1;
    } else {
        return;
    }

    editor_state.active_tile = *lookup
        .get(editor_state.active_tile_idx)
        .unwrap_or(&editor_state.active_tile);

    log.add(
        format!("active tile is now {:?}", editor_state.active_tile),
        KENNEY_RED,
    );
    info!("active tile is now {:?}", editor_state.active_tile);
}

pub fn on_toggle_fov(input: Res<ButtonInput<KeyCode>>, mut stats: ResMut<PlayerStats>) {
    if input.just_pressed(KeyCode::KeyF) && input.pressed(KeyCode::ShiftLeft) {
        if stats.is_default() {
            stats.set_vision_range(10);
        } else {
            stats.reset_vision_range();
        }
    }
}

pub fn handle_map_operations(commands: Commands, mut input: ResMut<ButtonInput<KeyCode>>) {
    if input.pressed(KeyCode::ShiftLeft) && input.just_released(KeyCode::KeyS) {
        warn!("requested to save");
        input.clear();
        open_save_dialog(commands);
    } else if input.pressed(KeyCode::ShiftLeft) && input.just_released(KeyCode::KeyL) {
        warn!("requested to load");
        input.clear();
        open_load_dialog(commands);
    }
}

macro_rules! get_entity {
    ($query:expr, $on:expr) => {
        match $query.get_mut($on.event_target()) {
            Ok(val) => val,
            Err(_) => return,
        }
    };
}

pub fn setup_global_tile_observers(mut commands: Commands) {
    commands.add_observer(
        |on: On<Pointer<Over>>,
         mut tiles: Query<(&mut Highlighted, Option<&mut TilePreview>), With<MapTile>>,
         editor: Res<EditorState>| {
            let (mut highlighted, preview_opt) = get_entity!(tiles, on);
            highlighted.0 = true;
            if let Some(mut preview) = preview_opt {
                preview.set(editor.active_tile);
            }
        },
    );
    commands.add_observer(
        |on: On<Pointer<Out>>,
         mut tiles: Query<(&mut Highlighted, Option<&mut TilePreview>), With<MapTile>>| {
            let (mut highlighted, preview_opt) = get_entity!(tiles, on);
            highlighted.0 = false;
            if let Some(mut preview) = preview_opt {
                preview.clear();
            }
        },
    );
    commands.add_observer(
        |on: On<Pointer<Click>>,
         mut tiles: Query<&mut TileIdx, With<MapTile>>,
         editor: Res<EditorState>| {
            let mut tile_idx = get_entity!(tiles, on);
            *tile_idx = match on.button {
                PointerButton::Primary => editor.active_tile,
                PointerButton::Secondary => TileIdx::Blank,
                _ => *tile_idx,
            };
        },
    );
}

pub fn add_editor_components(mut commands: Commands, tiles: Query<Entity, Added<MapTile>>) {
    for tile in tiles.iter() {
        commands
            .entity(tile)
            .insert(Pickable::default())
            .insert(Highlighted(false))
            .insert(TilePreview::default());
    }
}

type PathBufTask = Task<Option<std::path::PathBuf>>;

#[derive(Component)]
pub(crate) struct LoadDialogTask(PathBufTask);

#[derive(Message)]
pub(crate) struct MapLoadMessage(PathBuf);

pub fn open_load_dialog(mut commands: Commands) {
    let task_pool = AsyncComputeTaskPool::get();
    let task = task_pool.spawn(async move {
        rfd::AsyncFileDialog::new()
            .add_filter("RON files", &["ron"])
            .set_directory(DATA_DIR)
            .pick_file()
            .await
            .map(|handle| handle.path().to_owned())
    });
    commands.spawn(LoadDialogTask(task));
}

pub fn poll_load_dialog(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut LoadDialogTask)>,
    mut load_events: MessageWriter<MapLoadMessage>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(opt_path) = futures::check_ready(&mut task.0) {
            if let Some(path) = opt_path {
                load_events.write(MapLoadMessage(path));
            }
            commands.entity(entity).remove::<LoadDialogTask>();
        }
    }
}

pub fn load_map(
    mut commands: Commands,
    mut storage: Single<&mut TileStorage>,
    mut load_messages: MessageReader<MapLoadMessage>,
) {
    for message in load_messages.read() {
        let serialized = std::fs::read_to_string(&message.0).unwrap();
        let deserialized = ron::from_str::<SavedTilemap>(&serialized).unwrap();
        tilemap::load_map(&mut commands, &deserialized, storage.as_mut());
    }
}

#[derive(Message)]
pub(crate) struct MapSaveMessage(PathBuf);

#[derive(Component)]
pub(crate) struct SaveDialogTask(PathBufTask);

pub fn open_save_dialog(mut commands: Commands) {
    let task_pool = AsyncComputeTaskPool::get();
    let task = task_pool.spawn(async move {
        AsyncFileDialog::new()
            .add_filter("ron", &["ron"])
            .set_directory(DATA_DIR)
            .save_file()
            .await
            .map(|handle| handle.path().to_path_buf())
    });
    commands.spawn(SaveDialogTask(task));
}

pub fn poll_save_dialog(
    mut commands: Commands,
    mut save_dialog_tasks: Query<(Entity, &mut SaveDialogTask)>,
    mut save_events: MessageWriter<MapSaveMessage>,
) {
    for (entity, mut task) in save_dialog_tasks.iter_mut() {
        if let Some(opt_path) = futures::check_ready(&mut task.0) {
            if let Some(path) = opt_path {
                save_events.write(MapSaveMessage(path));
            }
            commands.entity(entity).remove::<SaveDialogTask>();
        }
    }
}

pub fn save_map(
    mut storage: Single<&mut TileStorage>,
    all_tiles: Query<&tiles::TileIdx, With<MapTile>>,
    mut save_messages: MessageReader<MapSaveMessage>,
) {
    for message in save_messages.read() {
        let storage = tilemap::save_map(&mut storage, &all_tiles);
        if let Ok(serialized) = ron::to_string(&storage) {
            let Ok(_) = std::fs::write(&message.0, serialized) else {
                continue;
            };
        }
    }
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_global_tile_observers)
            .add_systems(
                Update,
                (
                    add_editor_components,
                    on_button_input,
                    on_zoom_button_input,
                    on_toggle_fov,
                    handle_map_operations,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                (poll_load_dialog, poll_save_dialog, load_map, save_map),
            )
            .add_message::<MapLoadMessage>()
            .add_message::<MapSaveMessage>();
    }
}
