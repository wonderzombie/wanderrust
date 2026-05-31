use bevy::prelude::*;

use crate::{
    bestiary::Bestiary,
    equipment::{Equippable, HasEquipped},
    parameters::Parameters,
    tiles::TileIdx,
};

pub fn apply_params_modifiers(
    affected: Query<(Entity, &TileIdx, &HasEquipped, &mut Parameters), Changed<HasEquipped>>,
    equipment: Query<&Equippable>,
) {
    for (nt, tile_idx, has_equipped, mut extant_params) in affected {
        let params = Bestiary::from_tile(tile_idx).unwrap_or_default();
        if params.is_default() {
            warn!(
                "{nt:?}: no stats found for {tile_idx}; using defaults {:?}",
                params
            );
        }
        trace!("params for {tile_idx}: {:?}", params);

        let modified: Parameters = equipment
            .iter_many(has_equipped.iter())
            .fold(params, |acc, eq| eq.modify(acc));

        trace!("modified params for {tile_idx}: {:?}", modified);

        extant_params.set_if_neq(modified);
    }
}
