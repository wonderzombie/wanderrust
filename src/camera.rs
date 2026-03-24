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

pub fn update_camera(
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    player_query: Query<&Cell, With<Player>>,
    zoom_opt: Option<Res<DesiredZoom>>,
) {
    let Ok(player_cell) = player_query.single() else {
        error!("No player entity found in the world.");
        return;
    };

    let Ok(mut camera_transform) = camera_query.single_mut() else {
        error!("No camera entity found in the world.");
        return;
    };

    camera_transform.translation.x =
        (player_cell.x as f32 * tiles::TILE_SIZE_PX) + (tiles::TILE_SIZE_PX / 2.0);
    camera_transform.translation.y =
        (player_cell.y as f32 * tiles::TILE_SIZE_PX) + (tiles::TILE_SIZE_PX / 2.0);
    let zoom = zoom_opt.map_or(1.0, |zoom| zoom.0);
    camera_transform.scale = Vec3::splat(zoom);
}
