use bevy::prelude::*;

use mrpas::Mrpas;
use serde::{Deserialize, Serialize};

use crate::{
    actors::{Player, PlayerStats},
    cell::Cell,
    tilemap::{Dimensions, Stratum},
    tiles::{MapTile, Revealed, TileIdx},
};

/// Newtype for field of view model that's a Resource and which tracks which cells are transparent for visibility calculations.
#[derive(Resource, Component, Debug, Deref, DerefMut)]
pub struct Fov(Mrpas);

impl Fov {
    pub fn from(&self, origin: (i32, i32), max_distance: u32) -> View {
        let mut model = self.0.clone();
        model.clear_field_of_view();
        model.compute_field_of_view(origin, max_distance as i32);
        View(model)
    }
}

/// Newtype for a read-only clone of an existing Mrpas model configured for one
/// viewer's origin and max_distance.
///
/// The MRPAS API is ported from GDScript is highly stateful: it maintains both
/// the model (i.e. map of opaque/transparent positions) *and* the currently
/// computed (active) field of view. `clear_field_of_view()` is required before
/// `compute_field_of_view()`, and they both mutate the model.
#[derive(Resource, Debug, Deref, DerefMut)]
pub struct View(Mrpas);

impl View {
    /// Queries a read-only MRPAS model using the origin and max_distance used to create `View`.
    pub fn has(&self, pos: (i32, i32)) -> bool {
        self.0.is_in_view(pos)
    }
}

#[derive(Component, Copy, Clone, Debug, Serialize, Deserialize, Reflect)]
#[reflect(Component)]
pub struct Vision(pub u32);

impl Default for Vision {
    fn default() -> Self {
        Self(2)
    }
}

impl Vision {
    pub fn range(&self) -> u32 {
        self.0
    }
}

/// Internalizes the field of view model by marking tiles as transparent or not.
/// The field of view is marked as opaque beforehand.
pub fn setup_fov(
    mut commands: Commands,
    stratum_children: Query<(&Stratum, &Dimensions, &Children)>,
    tiles: Query<(&Cell, &TileIdx), With<MapTile>>,
) {
    for (Stratum(strat_entity, _), dimensions, children) in stratum_children {
        info!("👀 checking {} children", children.iter().len());
        let mut tiles_count = 0;
        let mut opaque_count = 0;
        let mut fov = Fov(Mrpas::new(dimensions.width as i32, dimensions.width as i32));
        for &child in children {
            if let Ok((cell, tile_idx)) = tiles.get(child) {
                // Sets individual points in the model to transparent-or-not.
                fov.set_transparent(cell.into(), tile_idx.is_transparent());
                tiles_count += 1;
                if !tile_idx.is_transparent() {
                    opaque_count += 1;
                }
            }
        }
        fov.clear_field_of_view(); // initializes current FOV to "zero"
        commands.entity(*strat_entity).insert(fov);

        info!(
            "👀 initialized FOV model with {} tiles, {} opaque.",
            tiles_count, opaque_count
        )
    }
}

/// Updates the field of view model based on the type of tile's transparency-or-not.
pub fn update_fov_model(
    mut all_fov: Query<&mut Fov>,
    query: Query<(&Cell, &TileIdx, &ChildOf), Changed<TileIdx>>,
) {
    for (cell, tile_idx, child_of) in query.iter() {
        if let Ok(mut fov) = all_fov.get_mut(child_of.parent()) {
            fov.set_transparent(cell.into(), tile_idx.is_transparent());
        }
    }
}

/// Updates the [Revealed] status of [MapTile]s based on the player's [Fov].
/// Uses the [View] type to avoid mutating `Res<Fov>`.
pub fn update_fov_markers(
    all_fov: Query<(&Children, &Fov)>,
    player_query: Single<(&Cell, &ChildOf), With<Player>>,
    player_stats: Res<PlayerStats>,
    mut tiles: Query<(&Cell, &mut Revealed), With<MapTile>>,
) {
    // TODO: figure out the real active stratum that the player is on.
    // See also [`setup_player`].
    let (cell, player_child_of) = *player_query;

    let parent_strat = player_child_of.parent();
    let Some((child_tiles, player_fov)) = all_fov.get(parent_strat).ok() else {
        error!("No Fov found for player's stratum.");
        return;
    };

    let view = player_fov.from(cell.into(), player_stats.vision_range);

    // Since we got these tiles as children of `all_fov`, aka Stratum
    // we can look up each in `tiles`, which is constrained to `MapTile`.
    for &entity in child_tiles {
        if let Ok((cell, mut revealed)) = tiles.get_mut(entity) {
            let should_reveal = view.has(cell.into());
            if should_reveal != revealed.0 {
                revealed.0 = should_reveal;
            }
        }
    }
}
