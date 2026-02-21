use bevy::prelude::*;

use mrpas::Mrpas;

use crate::{
    Player,
    cell::Cell,
    map::MapSpec,
    tiles::{MapTile, Revealed, TileIdx},
};

#[derive(Resource, Debug, Deref, DerefMut)]
/// Newtype for field of view model that tracks which cells are transparent for visibility calculations.
pub struct Fov(Mrpas);

pub fn setup_fov(
    mut commands: Commands,
    spec: Res<MapSpec>,
    tiles: Query<(&Cell, &TileIdx), With<MapTile>>,
) {
    let mut fov = Fov(Mrpas::new(spec.size.x as i32, spec.size.y as i32));

    let mut tiles_count = 0;
    let mut opaque_count = 0;
    fov.clear_field_of_view();
    for (cell, tile_idx) in tiles.iter() {
        fov.set_transparent(cell.into(), tile_idx.is_transparent());
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

/// Updates the field of view model based on the transparency of tiles when their atlas index changes.
pub fn update_fov_model(
    mut fov: ResMut<Fov>,
    query: Query<(&Cell, &TileIdx), (Changed<TileIdx>, With<MapTile>)>,
) {
    for (cell, tile_idx) in query.iter() {
        fov.set_transparent(cell.into(), tile_idx.is_transparent());
    }
}

/// Updates the visibility of map tiles based on the player's field of view.
pub fn update_fov_markers(
    mut fov: ResMut<Fov>,
    player_query: Query<&Cell, With<Player>>,
    mut tiles: Query<(&Cell, &mut Revealed), With<MapTile>>,
) {
    let Ok(player_cell) = player_query.single() else {
        warn!("No player entity found in the world.");
        return;
    };

    fov.clear_field_of_view();
    fov.compute_field_of_view(player_cell.into(), 5);
    for (cell, mut revealed) in tiles.iter_mut() {
        revealed.0 = fov.is_in_view(cell.into());
    }
}
