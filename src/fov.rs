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
/// Also, Mrpas is not a Resource.
pub struct Fov(Mrpas);

#[derive(Resource, Debug, Deref, DerefMut)]
/// Newtype for a read-only clone of an existing Mrpas model for one viewer's origin and max_distance.
pub struct View(Mrpas);

impl View {
    pub fn new(model: &Mrpas, origin: (i32, i32), max_distance: i32) -> View {
        let mut model = model.clone();
        model.clear_field_of_view();
        model.compute_field_of_view(origin, max_distance);
        View(model)
    }

    pub fn has(&self, pos: (i32, i32)) -> bool {
        self.0.is_in_view(pos)
    }
}

pub fn setup_fov(
    mut commands: Commands,
    spec: Res<MapSpec>,
    tiles: Query<(&Cell, &TileIdx), With<MapTile>>,
) {
    let mut fov = Fov(Mrpas::new(spec.size.x as i32, spec.size.y as i32));

    let mut tiles_count = 0;
    let mut opaque_count = 0;
    // Intentionally clear the field.
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

/// Updates the field of view model based on the type of tile's transparency-or-not.
pub fn update_fov_model(
    mut fov: ResMut<Fov>,
    query: Query<(&Cell, &TileIdx), (Changed<TileIdx>, With<MapTile>)>,
) {
    for (cell, tile_idx) in query.iter() {
        fov.set_transparent(cell.into(), tile_idx.is_transparent());
    }
}

/// Updates the visibility of map tiles based on the player's field of view.
/// Changes FOV because the API is stateful.
pub fn update_fov_markers(
    fov: Res<Fov>,
    player_query: Query<&Cell, With<Player>>,
    mut tiles: Query<(&Cell, &mut Revealed), With<MapTile>>,
) {
    let Ok(player_cell) = player_query.single() else {
        warn!("No player entity found in the world.");
        return;
    };

    let view = View::new(&fov, player_cell.into(), 5);
    for (cell, mut revealed) in tiles.iter_mut() {
        revealed.0 = view.has(cell.into());
    }
}
