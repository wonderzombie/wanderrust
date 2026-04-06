use bevy::prelude::*;

use crate::{
    actors::Player,
    cell::Cell,
    editor::DesiredZoom,
    tilemap::{TilemapLayer, TilemapSpec},
    tiles,
};

const CAMERA_LAYER: TilemapLayer = TilemapLayer(0.);

pub fn setup_camera(mut commands: Commands, spec: Res<TilemapSpec>) {
    let size = spec.size;
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.5,
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(
            (size.width as f32 * tiles::TILE_SIZE_PX) / 2.0 - tiles::TILE_SIZE_PX / 2.0,
            (size.height as f32 * tiles::TILE_SIZE_PX) / 2.0 - tiles::TILE_SIZE_PX / 2.0,
            *CAMERA_LAYER,
        ),
    ));
}

pub fn update(
    mut camera_transform: Single<&mut Transform, With<Camera2d>>,
    player_cell: Single<&Cell, With<Player>>,
    zoom_opt: Option<Res<DesiredZoom>>,
) {
    camera_transform.translation.x =
        (player_cell.x as f32 * tiles::TILE_SIZE_PX) + (tiles::TILE_SIZE_PX / 2.0);
    camera_transform.translation.y =
        (player_cell.y as f32 * tiles::TILE_SIZE_PX) + (tiles::TILE_SIZE_PX / 2.0);
    let zoom = zoom_opt.map_or(1.0, |zoom| zoom.0);
    camera_transform.scale = Vec3::splat(zoom);
}

// TODO: we can use this to determine which cells are visible and need to be rendered.
#[allow(dead_code)]
pub fn visible_cell_range(viewport_size: Vec2, origin_cell: Cell) -> (Cell, Cell) {
    let half_w = (viewport_size.x / tiles::TILE_SIZE_PX / 2.0).ceil() as i32;
    let half_h = (viewport_size.y / tiles::TILE_SIZE_PX / 2.0).ceil() as i32;

    let tl = Cell::new(origin_cell.x - half_w, origin_cell.y - half_h);
    let br = Cell::new(origin_cell.x + half_w, origin_cell.y + half_h);

    (tl, br)
}
