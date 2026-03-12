use bevy::prelude::*;

use mrpas::Mrpas;

use crate::{
    Player,
    cell::Cell,
    player,
    tilemap::TilemapSpec,
    tiles::{MapTile, Revealed, TileIdx},
};

#[derive(Resource, Debug, Deref, DerefMut)]
/// Newtype for field of view model that's a Resource and which tracks which cells are transparent for visibility calculations.
pub struct Fov(Mrpas);

impl Fov {
    pub fn from(&self, origin: (i32, i32), max_distance: u32) -> View {
        let mut model = self.0.clone();
        model.clear_field_of_view();
        model.compute_field_of_view(origin, max_distance as i32);
        View(model)
    }
}

#[derive(Resource, Debug, Deref, DerefMut)]
/// Newtype for a read-only clone of an existing Mrpas model configured for one viewer's origin and max_distance.
///
/// The MRPAS API is ported from GDScript is highly stateful: it maintains both the model (i.e. map of opaque/transparent positions)
/// *and* the currently computed (active) field of view. `clear_field_of_view()` is required before `compute_field_of_view()`, and
/// they both mutate the model.
pub struct View(Mrpas);

impl View {
    /// Queries a read-only MRPAS model using the origin and max_distance used to create `View`.
    pub fn has(&self, pos: (i32, i32)) -> bool {
        self.0.is_in_view(pos)
    }
}

/// Internalizes the field of view model by marking tiles as transparent or not.
/// The field of view is marked as opaque beforehand.
pub fn setup_fov(
    mut commands: Commands,
    spec: Res<TilemapSpec>,
    tiles: Query<(&Cell, &TileIdx), With<MapTile>>,
) {
    let mut fov = Fov(Mrpas::new(spec.size.width as i32, spec.size.width as i32));

    let mut tiles_count = 0;
    let mut opaque_count = 0;
    fov.clear_field_of_view(); // initializes current FOV to "zero"
    for (cell, tile_idx) in tiles.iter() {
        // Sets individual points in the model to transparent-or-not.
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
/// Uses the View type to avoid mutating `Res<Fov>`.
pub fn update_fov_markers(
    fov: Res<Fov>,
    player_query: Query<&Cell, With<Player>>,
    player_stats: Res<player::PlayerStats>,
    mut tiles: Query<(&Cell, &mut Revealed), With<MapTile>>,
) {
    let Ok(player_cell) = player_query.single() else {
        warn!("No player entity found in the world.");
        return;
    };

    let view = fov.from(player_cell.into(), player_stats.vision_range);
    for (cell, mut revealed) in tiles.iter_mut() {
        let should_reveal = view.has(cell.into());
        if should_reveal != revealed.0 {
            revealed.0 = should_reveal;
        }
    }
}
