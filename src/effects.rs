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
        let modified: Parameters = has_equipped
            .iter()
            .filter_map(|e| equipment.get(e).ok())
            .fold(params, |acc, eq| eq.modify(acc));

        extant_params.set_if_neq(modified);
    }
}
