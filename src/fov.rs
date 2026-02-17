use bevy::prelude::*;

use mrpas::Mrpas;

use crate::{
    Player,
    cell::Cell,
    map::MapSpec,
    queries::MapCellData,
    tiles::{Hidden, MapTile, TileIdx},
};

#[derive(Debug, Resource, Deref, DerefMut)]
/// Newtype for field of view model that tracks which cells are transparent for visibility calculations.
pub struct Fov(Mrpas);

pub fn setup_fov(
    mut commands: Commands,
    spec: Res<MapSpec>,
    tiles: Query<MapCellData, With<MapTile>>,
) {
    let mut fov = Fov(Mrpas::new(spec.size.x as i32, spec.size.y as i32));

    let mut tiles_count = 0;
    let mut opaque_count = 0;
    fov.clear_field_of_view();
    for (cell, tile_idx) in tiles.iter().map(|it| (it.cell, it.tile_idx)) {
        let (x, y) = (*cell).into();
        fov.set_transparent((x, y), tile_idx.is_transparent());
        tiles_count += 1;
        if !tile_idx.is_transparent() {
            opaque_count += 1;
        }
    }

    info!(
        "Initialized FOV model with {} tiles, {} opaque.",
        tiles_count, opaque_count
    );

    commands.insert_resource(fov);
}

/// Updates the field of view model based on the transparency of cells in the FOV model when their atlas index changes.
pub fn update_fov_model(
    mut fov: ResMut<Fov>,
    query: Query<MapCellData, (Changed<TileIdx>, With<MapTile>)>,
) {
    for res in query.iter() {
        fov.set_transparent(res.xy(), res.tile_idx.is_transparent());
    }
}

/// Updates the visibility of map tiles' sprites based on the player's field of view.
pub fn update_tiles_in_view(
    mut fov: ResMut<Fov>,
    player_query: Query<&Cell, With<Player>>,
    mut tiles: Query<(&Cell, &mut Sprite, &mut Hidden), With<MapTile>>,
) {
    let Ok(player_cell) = player_query.single() else {
        warn!("No player entity found in the world.");
        return;
    };

    fov.clear_field_of_view();
    fov.compute_field_of_view((*player_cell).into(), 5);
    for (cell, mut sprite, mut hidden) in tiles.iter_mut() {
        hidden.0 = !fov.is_in_view((*cell).into());
        sprite.color = if hidden.0 { Color::NONE } else { Color::WHITE };
    }
}
