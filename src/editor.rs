use std::path::PathBuf;

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures},
};
use rfd::AsyncFileDialog;

use crate::{
    actors::{Actor, Player, PlayerStats},
    cell::Cell,
    colors::KENNEY_RED,
    combat::Parameters,
    event_log,
    gamestate::GameState,
    interactions::Interactable,
    ldtk_loader,
    tilemap::{self, Portal, StratPortals, Stratum, TileStorage, TilemapSpec},
    tiles::{self, Highlighted, MapTile, TileIdx, TilePreview},
};
const DATA_DIR: &str = "data";

#[derive(States, Clone, Debug, Hash, Eq, PartialEq)]
pub enum EditorState {
    Disabled,
    Enabled,
}

#[derive(Resource)]
pub struct EditorContext {
    pub active_tile: tiles::TileIdx,
    pub active_tile_idx: usize,
    pub observers: Vec<Entity>,
}

impl Default for EditorContext {
    fn default() -> Self {
        Self {
            active_tile: tiles::TileIdx::Grass,
            active_tile_idx: default(),
            observers: Vec::new(),
        }
    }
}

#[derive(Resource)]
pub struct DesiredZoom(pub f32);

/// Handles zoom button input, updating the desired zoom level.
pub fn on_zoom_button_input(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    zoom_opt: Option<ResMut<DesiredZoom>>,
) {
    let mut current_zoom = zoom_opt.map_or(1.0, |zoom| zoom.0);

    if input.just_released(KeyCode::Equal) {
        current_zoom -= 0.2;
    } else if input.just_released(KeyCode::Minus) {
        current_zoom += 0.2
    } else if input.just_released(KeyCode::Backspace) {
        current_zoom = 1.0;
    } else if input.just_released(KeyCode::Digit0) {
        current_zoom = 10.;
    }

    let final_zoom = current_zoom.clamp(-10.0, 10.0);
    commands.insert_resource(DesiredZoom(final_zoom));
}

/// Handles button input, updating the active tile and logging events.
pub fn on_button_input(
    mut commands: Commands,
    player: Single<Entity, With<Player>>,
    input: Res<ButtonInput<KeyCode>>,
    mut editor_state: ResMut<EditorContext>,
    mut log: ResMut<event_log::MessageLog>,
) {
    if !input.is_changed() {
        return;
    }

    let lookup = tiles::TileIdx::all();

    if input.just_pressed(KeyCode::Digit1) {
        // First tile index
        editor_state.active_tile_idx = 0;
    } else if input.just_pressed(KeyCode::Digit2) {
        // Previous tile index
        editor_state.active_tile_idx = editor_state.active_tile_idx.saturating_sub(1);
    } else if input.just_pressed(KeyCode::Digit3) {
        // Next tile index
        editor_state.active_tile_idx = (editor_state.active_tile_idx + 1).min(lookup.len() - 1);
    } else if input.just_pressed(KeyCode::Digit4) {
        // Last viable tile index
        editor_state.active_tile_idx = lookup.len() - 1;
    } else if input.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight])
        && input.just_released(KeyCode::KeyP)
    {
        info!("relocating player");
        commands.entity(*player).insert(Cell::new(5, 5));
        return;
    } else {
        return;
    }

    // TODO: update the tile preview that follows the cursor.
    // At present it doesn't update until the mouse moves.
    editor_state.active_tile = *lookup
        .get(editor_state.active_tile_idx)
        .unwrap_or(&editor_state.active_tile);

    log.add(
        format!("active tile is now {:?}", editor_state.active_tile),
        KENNEY_RED,
    );
    info!("active tile is now {:?}", editor_state.active_tile);
}

/// Toggles the player's field of view range.
pub fn on_toggle_fov(input: Res<ButtonInput<KeyCode>>, mut stats: ResMut<PlayerStats>) {
    if input.just_pressed(KeyCode::KeyF) && input.pressed(KeyCode::ShiftLeft) {
        if stats.is_default() {
            stats.set_vision_range(25);
        } else {
            stats.reset_vision_range();
        }
    }
}

/// Dispatches map-related operations based on keyboard input.
pub fn handle_map_operations(
    commands: Commands,
    mut input: ResMut<ButtonInput<KeyCode>>,
    mut msg: MessageWriter<LdtkMapLoadMessage>,
) {
    if input.pressed(KeyCode::ShiftLeft) && input.just_released(KeyCode::KeyS) {
        warn!("requested to save");
        input.clear();
        open_save_dialog(commands);
    } else if input.pressed(KeyCode::ShiftLeft) && input.just_released(KeyCode::KeyL) {
        warn!("requested to load");
        input.clear();
        open_load_dialog(commands);
    } else if input.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight])
        && input.just_released(KeyCode::KeyL)
    {
        msg.write(LdtkMapLoadMessage);
    }
}

/// Convenience macro for getting an entity from a query, returning early if the entity is not found.
macro_rules! get_entity {
    ($query:expr, $on:expr) => {
        match $query.get_mut($on.event_target()) {
            Ok(val) => val,
            Err(_) => return,
        }
    };
}

/// Sets up global tile observers that highlight and preview tiles when the pointer is over them.
pub fn setup_global_tile_observers(mut commands: Commands, mut editor: ResMut<EditorContext>) {
    for &obs in editor.observers.iter() {
        commands.entity(obs).despawn()
    }
    editor.observers.clear();

    let over_obs = commands
        .add_observer(
            |on: On<Pointer<Over>>,
             mut tiles: Query<Option<&mut TilePreview>, With<MapTile>>,
             editor: Res<EditorContext>| {
                let preview_opt = get_entity!(tiles, on);
                if let Some(mut preview) = preview_opt {
                    preview.set(editor.active_tile);
                }
            },
        )
        .insert(Name::new("editor over observer"))
        .id();
    let out_obs = commands
        .add_observer(
            |on: On<Pointer<Out>>,
             mut commands: Commands,
             mut tiles: Query<Option<&mut TilePreview>, With<MapTile>>| {
                let preview_opt = get_entity!(tiles, on);
                commands.entity(on.event_target()).remove::<Highlighted>();
                if let Some(mut preview) = preview_opt {
                    preview.clear();
                }
            },
        )
        .insert(Name::new("editor out observer"))
        .id();
    let click_obs = commands
        .add_observer(
            |on: On<Pointer<Click>>,
             mut tiles: Query<&mut TileIdx, With<MapTile>>,
             editor: Res<EditorContext>,
             state: Res<State<EditorState>>| {
                if state.get() != &EditorState::Enabled {
                    return;
                }
                let mut tile_idx = get_entity!(tiles, on);
                *tile_idx = match on.button {
                    PointerButton::Primary => editor.active_tile,
                    PointerButton::Secondary => TileIdx::Blank,
                    _ => *tile_idx,
                };
            },
        )
        .insert(Name::new("editor click observer"))
        .id();

    editor
        .observers
        .extend_from_slice(&[over_obs, out_obs, click_obs]);
}

/// Adds [Pickable] and [TilePreview] components to newly added [MapTile] entities.
pub fn add_editor_components(mut commands: Commands, tiles: Query<Entity, Added<MapTile>>) {
    for tile in tiles.iter() {
        commands.entity(tile).insert(TilePreview::default());
    }
}

pub fn remove_editor_components(mut commands: Commands, tiles: Query<Entity, With<MapTile>>) {
    for tile in tiles.iter() {
        commands
            .entity(tile)
            // TODO: we could try to remove TilePreview. Removing it means it won't be updated
            // though so we set the TilePreview to the default [`None`].
            .insert(TilePreview::default());
    }
}

pub fn remove_global_tile_observers(mut commands: Commands, mut editor: ResMut<EditorContext>) {
    for &obs in editor.observers.iter() {
        commands.entity(obs).despawn();
    }
    editor.observers.clear();
}

/// Represents a task (a dialog) which results in a [`PathBuf`] (a file path).
type PathBufTask = Task<Option<std::path::PathBuf>>;

#[derive(Component)]
pub struct LoadDialogTask(PathBufTask);

#[derive(Message)]
pub struct MapLoadMessage(PathBuf);

/// Opens a file dialog to select a map file and spawns a [LoadDialogTask] to load the selected file.
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

/// Polls the [LoadDialogTask] for a result and loads the map if one is available.
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

/// Loads a map from a spec at the behest of `load_messages` [`SystemParam`].
/// Uses a lot of unwrap.
pub fn on_load_map_message(
    mut load_messages: MessageReader<MapLoadMessage>,
    mut spec: ResMut<TilemapSpec>,
    mut next: ResMut<NextState<GameState>>,
) {
    for message in load_messages.read() {
        let serialized = std::fs::read_to_string(&message.0).unwrap();
        let mut new_spec = ron::from_str::<TilemapSpec>(&serialized).unwrap();

        let serialized = std::fs::read_to_string(message.0.with_file_name("portals.ron")).unwrap();
        let portals = ron::from_str::<StratPortals>(&serialized).unwrap();

        let serialized = std::fs::read_to_string(message.0.with_file_name("actors.ron")).unwrap();
        let _: Vec<(Cell, TileIdx, Interactable, Parameters)> =
            ron::from_str::<Vec<_>>(&serialized).unwrap();

        new_spec.all_portals = portals;
        *spec = new_spec;

        next.set(GameState::Loading);
    }
}

#[derive(Debug, Message)]
pub struct LdtkMapLoadMessage;

pub fn on_load_ldtk_map_message(
    mut commands: Commands,
    mut messages: MessageReader<LdtkMapLoadMessage>,
) {
    for _ in messages.read() {
        let fname = "data/wandrs_proto.ldtk";

        let res = ldtk_loader::load_and_import(fname.into()).expect("expected to load ldtk level");

        commands.insert_resource(res);
    }
}

#[derive(Message)]
pub struct MapSaveMessage(PathBuf);

#[derive(Component)]
pub struct SaveDialogTask(PathBufTask);

/// Opens a save dialog and spawns a [SaveDialogTask] to handle the result.
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

/// Polls the [SaveDialogTask] for a result and saves the map if a path is returned.
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

/// Saves the map to disk using the provided queries.
pub fn on_save_map_message(
    spec: Res<TilemapSpec>,
    mut strat_storage: Query<(&Stratum, &TileStorage)>,
    all_tiles: Query<&tiles::TileIdx>,
    all_portals: Query<(&Portal, &Cell, &ChildOf)>,
    all_actors: Query<(&Cell, &TileIdx, &Interactable, &Parameters), With<Actor>>,
    mut save_messages: MessageReader<MapSaveMessage>,
) {
    let mut new_spec = spec.into_inner().clone();
    for message in save_messages.read() {
        let tiles = tilemap::get_live_tiles(&new_spec.size, &strat_storage, &all_tiles);
        new_spec.all_tiles = tiles;

        info!("saving {:?}", message.0);
        if let Ok(serialized) = ron::to_string(&new_spec) {
            let Ok(_) = std::fs::write(&message.0, serialized) else {
                continue;
            };
        }

        let mut strata = strat_storage.transmute_lens::<&Stratum>();
        let portals: tilemap::StratPortals =
            tilemap::get_live_portals(&strata.query(), &all_portals);
        let path = message.0.with_file_name("portals.ron");

        info!("saving {:?}", path);
        if let Ok(serialized) = ron::to_string(&portals) {
            let Ok(_) = std::fs::write(&path, serialized) else {
                continue;
            };
        }

        // TODO: de/serialize more than just this.
        let actors = all_actors.iter().collect::<Vec<_>>();
        let path = message.0.with_file_name("actors.ron");

        info!("saving {:?}", path);
        if let Ok(serialized) = ron::to_string(&actors) {
            let Ok(_) = std::fs::write(&path, serialized) else {
                continue;
            };
        }

        info!("✅\tdone saving.\t✅")
    }
}

pub fn on_editor_toggle(
    input: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<EditorState>>,
    mut next_state: ResMut<NextState<EditorState>>,
    mut log: ResMut<event_log::MessageLog>,
) {
    if input.just_pressed(KeyCode::Backspace)
        && input.any_pressed([KeyCode::ShiftRight, KeyCode::ShiftLeft])
    {
        let next = match **current_state {
            EditorState::Enabled => EditorState::Disabled,
            EditorState::Disabled => EditorState::Enabled,
        };
        log.add(format!("! editor: {:?} !", next), Color::WHITE);
        next_state.set(next);
    }
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_global_tile_observers)
            .add_systems(
                Update,
                (
                    (
                        // This should run in case any map tiles have been added/removed.
                        add_editor_components,
                        on_button_input,
                        on_zoom_button_input,
                        on_toggle_fov,
                        handle_map_operations,
                    )
                        .chain()
                        .run_if(in_state(EditorState::Enabled)),
                    on_editor_toggle,
                ),
            )
            .add_systems(
                OnExit(EditorState::Enabled),
                // These only need to run once per transition to Disabled.
                (remove_editor_components, remove_global_tile_observers),
            )
            .add_systems(
                PostUpdate,
                (
                    poll_load_dialog,
                    poll_save_dialog,
                    on_load_map_message,
                    on_save_map_message,
                    on_load_ldtk_map_message,
                )
                    .run_if(in_state(EditorState::Enabled)),
            )
            .insert_resource(EditorContext::default())
            .insert_state(EditorState::Disabled)
            .add_message::<LdtkMapLoadMessage>()
            .add_message::<MapLoadMessage>()
            .add_message::<MapSaveMessage>();
    }
}
