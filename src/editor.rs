use bevy::prelude::*;

use crate::{
    cell::Cell,
    colors::{KENNEY_GOLD, KENNEY_RED},
    event_log,
    tilemap::{self, SavedTilemap, TilemapStorage},
    tiles::{self, Highlighted, MapTile, TileIdx},
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

pub fn on_button_input(
    input: Res<ButtonInput<KeyCode>>,
    mut editor_state: ResMut<EditorState>,
    mut log: ResMut<event_log::MessageLog>,
) {
    if !input.is_changed() {
        return;
    }

    let lookup = tiles::TileIdx::all();

    if input.just_pressed(KeyCode::Digit0) {
        editor_state.active_tile_idx = 0;
    } else if input.just_pressed(KeyCode::Digit1) {
        editor_state.active_tile_idx = editor_state.active_tile_idx.saturating_sub(1);
    } else if input.just_pressed(KeyCode::Digit2) {
        editor_state.active_tile_idx = (editor_state.active_tile_idx + 1).min(lookup.len() - 1);
    } else if input.just_pressed(KeyCode::Digit3) {
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

pub fn handle_map_operations(
    mut commands: Commands,
    mut input: ResMut<ButtonInput<KeyCode>>,
    mut storage: Single<&mut TilemapStorage>,
    all_tiles: Query<&tiles::TileIdx, With<MapTile>>,
    mut log: ResMut<crate::event_log::MessageLog>,
) {
    if input.all_pressed([KeyCode::ShiftLeft, KeyCode::KeyS]) {
        warn!("requested to save");
        input.clear();
        let storage = tilemap::save_map(&mut storage, &all_tiles);
        let serialized = ron::to_string(&storage).unwrap();
        std::fs::write("level.ron", serialized).unwrap();
        log.add("Saved map", KENNEY_RED);
        info!("saved map to level.ron");
    } else if input.all_pressed([KeyCode::ShiftLeft, KeyCode::KeyL]) {
        warn!("requested to load");
        input.clear();
        let serialized = std::fs::read_to_string("level.ron").unwrap();
        let deserialized = ron::from_str::<SavedTilemap>(&serialized).unwrap();
        tilemap::load_map(&mut commands, &deserialized, storage.as_mut());
        log.add("Loaded map", KENNEY_RED);
        info!("loaded map from level.ron");
    }
}

pub fn setup_tile_observers(
    mut commands: Commands,
    mut picking_settings: ResMut<SpritePickingSettings>,
    tiles: Query<Entity, Or<(With<MapTile>, Added<MapTile>)>>,
) {
    picking_settings.picking_mode = SpritePickingMode::BoundingBox;

    let mut count = 0;
    for tile in tiles.iter() {
        commands
            .entity(tile)
            .insert(Pickable::default())
            .insert(Highlighted(false))
            .observe(
                |on: On<Pointer<Over>>, mut sprites: Query<&mut Highlighted, With<MapTile>>| {
                    let Ok(mut highlighted) = sprites.get_mut(on.event_target()) else {
                        // warn!("over? not i");
                        return;
                    };
                    highlighted.0 = true;
                },
            )
            .observe(
                |on: On<Pointer<Out>>, mut sprites: Query<&mut Highlighted, With<MapTile>>| {
                    let Ok(mut highlighted) = sprites.get_mut(on.event_target()) else {
                        // warn!("out? not i");
                        return;
                    };
                    highlighted.0 = false;
                },
            )
            .observe(
                |on: On<Pointer<Click>>,
                 mut tiles: Query<&mut TileIdx, With<MapTile>>,
                 editor: Res<EditorState>| {
                    let Ok(mut tile_idx) = tiles.get_mut(on.event_target()) else {
                        return;
                    };

                    *tile_idx = match on.button {
                        PointerButton::Primary => editor.active_tile,
                        PointerButton::Secondary => TileIdx::Blank,
                        _ => *tile_idx,
                    };
                },
            );
        count += 1;
    }
    info!("Total tiles observed: {}", count);
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                setup_tile_observers.run_if(run_once),
                on_button_input,
                handle_map_operations,
            )
                .chain(),
        );
    }
}
