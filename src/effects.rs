use bevy::prelude::*;

use crate::{
    bestiary::Bestiary,
    equipment::{EquipmentChanged, HasEquipped, ToggleEquip},
    gamestate::PlayerSpawned,
    items::ItemId,
    parameters::Parameters,
    tiles::TileIdx,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        PostUpdate,
        apply_params_modifiers
            .run_if(on_message::<EquipmentChanged>.or_else(on_message::<ToggleEquip>)),
    )
    .add_message::<EquipmentChanged>()
    .add_observer(detect_spawn);
}

pub fn apply_params_modifiers(
    curr_equip: Query<(Entity, &TileIdx, &HasEquipped, &mut Parameters)>,
    equipment: Query<&ItemId>,
) {
    for (nt, tile_idx, has_equipped, mut extant_params) in curr_equip {
        let params = Bestiary::from_tile(tile_idx).unwrap_or_default();
        if params.is_default() {
            warn!("{nt:?}: no stats found for {tile_idx}; using defaults {params:?}",);
        }
        trace!("params for {tile_idx}: {params:?}");

        let modified: Parameters = equipment
            .iter_many(has_equipped.iter())
            .flat_map(|it| it.equip_def())
            .fold(params, |acc, eq| eq.mods.modify(acc));

        info!("modified params for {tile_idx}: {modified:?}");
        extant_params.set_if_neq(modified);
    }
}

pub fn detect_spawn(_event: On<PlayerSpawned>, mut refresh: MessageWriter<EquipmentChanged>) {
    refresh.write_default();
}
