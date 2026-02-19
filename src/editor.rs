use std::ops::Deref;

use bevy::{
    camera::Camera,
    ecs::{
        change_detection::DetectChanges,
        resource::Resource,
        system::{Commands, Query, Res, ResMut, Single},
    },
    input::{ButtonInput, keyboard::KeyCode, mouse::MouseButton},
    log::{info, warn},
    transform::components::GlobalTransform,
    window::Window,
};
use itertools::Itertools;

use crate::{cell::Cell, editor, tilemap::TilemapStorage, tiles};

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

fn cursor_to_cell(
    window: &Window,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    tile_size: u32,
) -> Option<Cell> {
    let cursor_pos = window.cursor_position()?;
    let world_pos = camera
        .viewport_to_world_2d(camera_transform, cursor_pos)
        .ok()?;
    Some(Cell::new(
        (world_pos.x / tile_size as f32).floor() as i32,
        (world_pos.y / tile_size as f32).floor() as i32,
    ))
}

pub fn handle_mouse_button(
    mut commands: Commands,
    win: Single<&Window>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    grid_query: Query<&TilemapStorage>,
    editor_state: Res<EditorState>,
) {
    let grid = grid_query.single().unwrap();
    let (cam, xform) = *camera;
    let maybe_entity = cursor_to_cell(&win, cam, xform, 16u32)
        .map(|it| grid.get(&it))
        .flatten();

    if let Some(entity) = maybe_entity {
        if mouse_button.pressed(MouseButton::Left) {
            commands.entity(entity).insert(editor_state.active_tile);
        } else if mouse_button.pressed(MouseButton::Right) {
            commands.entity(entity).insert(tiles::TileIdx::Blank);
        }
    }
}

pub fn handle_editor_keys(input: Res<ButtonInput<KeyCode>>, mut editor_state: ResMut<EditorState>) {
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

    info!("active tile is now {:?}", editor_state.active_tile);
}
