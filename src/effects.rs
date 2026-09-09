use bevy::prelude::*;

use crate::{
    equipment::{EquipmentChanged, HasEquipped, Slots},
    gamestate::PlayerSpawned,
    items::ItemId,
    parameters::{BaseParameters, Parameters},
    unwrap_collection,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        PostUpdate,
        apply_params_modifiers.run_if(on_message::<EquipmentChanged>),
    )
    .add_observer(detect_spawn);
}

pub fn apply_params_modifiers(
    curr_equip: Query<
        (
            Entity,
            Option<&HasEquipped>,
            &BaseParameters,
            &mut Parameters,
        ),
        With<Slots>,
    >,
    equipment: Query<&ItemId>,
) {
    for (entity, has_equipped_opt, base_params, mut extant_params) in curr_equip {
        let has_equipped: Vec<_> = unwrap_collection(has_equipped_opt);

        let modified: Parameters = equipment
            .iter_many(has_equipped)
            .flat_map(|it| it.equip_def())
            .fold(base_params.params(), |acc, eq| eq.mods.modify(acc));

        info!("modified params for {entity}: {modified:?}");
        extant_params.set_if_neq(modified);
    }
}

pub fn detect_spawn(_event: On<PlayerSpawned>, mut refresh: MessageWriter<EquipmentChanged>) {
    refresh.write_default();
}
