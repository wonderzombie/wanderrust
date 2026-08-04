use bevy::prelude::*;

use mrpas::Mrpas;

use crate::{
    actors::Player,
    cell::Cell,
    parameters::{Parameters, Vision},
    tilemap::{Dimensions, Level},
    tiles::{MapTile, Opaque, Revealed, TileIdx},
};

const EMOJI: &str = "🔦";

/// Newtype for field of view model that's a Resource and which tracks which
/// cells are transparent for visibility calculations.
///
/// Instead of mutating a shared MRPAS model, we change this model only when the
/// environment changes. In order to compute a specific field of view, we
/// first create a snapshot and return that.
///
/// Rationale: the MRPAS API is ported from GDScript is highly stateful: it
/// maintains both the model (i.e. map of opaque/transparent positions for the
/// currently modeled environment) *and* the currently computed (active) field
/// of view based on the that model. `clear_field_of_view()` is required before
/// `compute_field_of_view()` and mutate the "active" field of view.
#[derive(Component, Debug, Deref, DerefMut)]
pub struct Fov(Mrpas);

impl Fov {
    /// Creates a View from origin, max distance, and the current MRPAS model.
    /// The view-related computation happens once, here, and may be used to check
    /// a specific origin/distance combination many times. Note that this is
    /// merely a snapshot, so any changes to the model occurring subsequently
    /// (such as a door opened in the next frame) will not be represented here.
    pub fn from(&self, origin: (i32, i32), max_distance: u32) -> View {
        let mut model = self.0.clone();
        model.clear_field_of_view();
        model.compute_field_of_view(origin, max_distance as i32);
        View(model)
    }
}

/// Newtype for a read-only snapshot (clone) of an existing Mrpas model
/// configured for one viewer's origin and max_distance. Therefore it is not
/// recommended to store this in a persistent way; it is only a snapshot of the
/// current MRPAS model at a given point in time, useful for checking visibility
/// of multiple points in the model.
#[derive(Resource, Debug, Deref, DerefMut)]
pub struct View(Mrpas);

impl View {
    /// Queries a read-only MRPAS model using the origin and max_distance used
    /// to create `View`.
    pub fn has(&self, pos: (i32, i32)) -> bool {
        self.0.is_in_view(pos)
    }
}

/// Internalizes the field of view model by marking tiles as transparent or not.
/// The field of view defaults to entirely opaque, so we carve out viewable
/// points based on `Without<Opaque>`.
pub fn setup_fov(
    mut commands: Commands,
    level_children: Query<(&Level, &Dimensions, &Children)>,
    transparent_tiles: Query<&Cell, (With<MapTile>, Without<Opaque>)>,
) {
    for (Level(level_entity, level_id), dimensions, children) in level_children {
        info!(
            "{EMOJI} {level_id:?} checking {} children",
            children.collection().len()
        );
        let tiles_count = dimensions.ntiles();
        let mut transparent_count = 0;
        let mut fov = Fov(Mrpas::new(dimensions.width as i32, dimensions.width as i32));

        for cell in transparent_tiles.iter_many(children) {
            fov.set_transparent(cell.into(), true);
            transparent_count += 1;
        }

        // Initializes the FOV to "none". See [`fov::View`].
        commands.entity(*level_entity).insert(fov);

        info!(
            "{EMOJI} {level_id:?}: initialized FOV model with {tiles_count} tiles, {transparent_count} transparent.",
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
/// Uses Option<&Vision> to allow overrides for player vision (debugging, utility).
pub fn update_fov_markers(
    all_fov: Query<(&Children, &Fov)>,
    player_query: Single<(&Cell, &ChildOf, &Parameters, Option<&Vision>), With<Player>>,
    mut tiles: Query<(&Cell, &mut Revealed), With<MapTile>>,
) {
    let (cell, &ChildOf(parent_level), params, vis_opt) = *player_query;

    let Some((child_tiles, player_fov)) = all_fov.get(parent_level).ok() else {
        error!("{EMOJI} no Fov found for player's level: {parent_level:?}");
        return;
    };

    let vis = vis_opt.unwrap_or(&params.vision);

    let view = player_fov.from(cell.into(), vis.range());

    // Since we got these tiles as children of `all_fov`, aka Level we can look
    // up each in `tiles`, which is constrained to `MapTile`.
    for &tile_entity in child_tiles {
        if let Ok((cell, mut revealed)) = tiles.get_mut(tile_entity) {
            let should_reveal = view.has(cell.into());
            revealed.set_if_neq(Revealed(should_reveal));
        }
    }
}
