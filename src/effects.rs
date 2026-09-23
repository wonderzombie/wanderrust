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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bestiary, equipment::EquippedBy, items::Slot};
    #[test]
    fn test_apply_params_modifiers() -> Result<(), BevyError> {
        let mut app = _init_app();
        app.add_systems(Update, apply_params_modifiers);

        let player_nt = app
            .world_mut()
            .commands()
            .spawn((Bestiary::Player, Slots::standard()))
            .id();
        app.update();

        let params = app.world().get::<Parameters>(player_nt);
        assert_eq!(Some(&Bestiary::Player.params()), params);

        let item_nt = app
            .world_mut()
            .commands()
            .spawn((
                ItemId::Sword,
                EquippedBy {
                    entity: player_nt,
                    slot: Slot::MainHand,
                },
            ))
            .id();
        app.update();

        let params = app.world().get::<Parameters>(player_nt);
        assert_ne!(Some(&Bestiary::Player.params()), params);

        app.world_mut().commands().entity(item_nt).despawn();
        app.update();

        let params = app.world().get::<Parameters>(player_nt);
        assert_eq!(Some(&Bestiary::Player.params()), params);

        Ok(())
    }

    fn _init_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app
    }
}
