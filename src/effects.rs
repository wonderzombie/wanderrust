use bevy::prelude::*;

use crate::{
    bestiary::Bestiary,
    equipment::{HasEquipped, ToggleEquip},
    gamestate::PlayerSpawn,
    items::ItemId,
    parameters::Parameters,
    tiles::TileIdx,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, detect_refresh)
        .add_systems(PostUpdate, apply_params_modifiers)
        .add_message::<RefreshModifiers>()
        .add_observer(detect_spawn);
}

#[derive(Message, Debug, Default)]
pub struct RefreshModifiers;

pub fn apply_params_modifiers(
    mut refresh_mods: PopulatedMessageReader<RefreshModifiers>,
    curr_equip: Query<(Entity, &TileIdx, &HasEquipped, &mut Parameters)>,
    equipment: Query<&ItemId>,
) {
    for _ in refresh_mods.read() {}

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

pub fn detect_refresh(
    mut toggle: PopulatedMessageReader<ToggleEquip>,
    mut refresh: MessageWriter<RefreshModifiers>,
) {
    for _ in toggle.read() {}
    refresh.write_default();
}

