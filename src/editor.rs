use bevy::prelude::*;

use crate::{
    colors::KENNEY_RED,
    event_log,
    player::PlayerStats,
    tilemap::{self, SavedTilemap, TileStorage},
    tiles::{self, Highlighted, MapTile, TileIdx, TilePreview},
};

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

pub fn handle_map_operations(
    mut commands: Commands,
    mut input: ResMut<ButtonInput<KeyCode>>,
    mut storage: Single<&mut TileStorage>,
    all_tiles: Query<&tiles::TileIdx, With<MapTile>>,
    mut log: ResMut<crate::event_log::MessageLog>,
) {
    if input.pressed(KeyCode::ShiftLeft) && input.just_released(KeyCode::KeyS) {
        warn!("requested to save");
        input.clear();
        let storage = tilemap::save_map(&mut storage, &all_tiles);
        let serialized = ron::to_string(&storage).unwrap();
        std::fs::write("data/level.ron", serialized).unwrap();
        log.add("Saved map", KENNEY_RED);
        info!("saved map to level.ron");
    } else if input.pressed(KeyCode::ShiftLeft) && input.just_released(KeyCode::KeyL) {
        warn!("requested to load");
        input.clear();
        let serialized = std::fs::read_to_string("data/level.ron").unwrap();
        let deserialized = ron::from_str::<SavedTilemap>(&serialized).unwrap();
        tilemap::load_map(&mut commands, &deserialized, storage.as_mut());
        log.add("Loaded map", KENNEY_RED);
        info!("loaded map from level.ron");
    }
}

pub fn pick_and_load(mut dialog: ResMut<AsyncLoadDialog>) -> Option<SavedTilemap> {
    use rfd::FileDialog;
    let picked = FileDialog::new()
        .add_filter("ron", &["ron"])
        .pick_file()
        .expect("expected a file to have been picked");

    let serialized = fs::read_to_string(&picked).expect("unable to read map file to string");

    let deserialized =
        ron::from_str::<SavedTilemap>(&serialized).expect("unable to deserialize map from string");

    info!("loaded map from {:?}", picked);
    Some(deserialized)
}

pub fn pick_and_save(tilemap: &SavedTilemap) {
    use rfd::FileDialog;
    let picked = FileDialog::new()
        .add_filter("ron", &["ron"])
        .pick_file()
        .expect("expected a file to have been picked");

    let serialized = ron::to_string(tilemap).expect("unable to serialize map to string");
    let _ = fs::write(&picked, serialized).expect("unable to save serialized map");

    info!("saved map to {:?}", picked);
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

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_global_tile_observers);
        app.add_systems(
            Update,
            (
                add_editor_components,
                on_button_input,
                on_zoom_button_input,
                on_toggle_fov,
                handle_map_operations,
            )
                .chain(),
        );
    }
}
