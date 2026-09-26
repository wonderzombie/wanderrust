use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};

use crate::{
    inventory::CarriedBy,
    items::{ItemId, Slot},
    parameters::Parameters,
    sounds, unwrap_collection,
};

pub(crate) fn plugin(app: &mut App) {
    app.add_observer(on_toggle_equipped)
        .add_message::<EquipmentChanged>();
}

#[derive(Message, Debug, Default)]
pub struct EquipmentChanged;

#[derive(Component, Reflect, Debug)]
#[component(on_insert = notify, on_discard = notify)]
#[relationship(relationship_target = HasEquipped)]
#[reflect(Component)]
#[component(immutable)]
pub struct EquippedBy {
    #[relationship]
    pub entity: Entity,
    pub slot: Slot,
}

fn notify(mut w: DeferredWorld, _c: HookContext) {
    info!("equipment changed");
    w.write_message_default::<EquipmentChanged>();
}

/// Defines a collection of [`Slot`].
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct Slots(pub Vec<Slot>);

impl Slots {
    /// Returns a standard list of [`Slot`] for such as a human-adjacent creature.
    pub fn standard() -> Self {
        Self(vec![
            Slot::Armor,
            Slot::MainHand,
            Slot::OffHand,
            Slot::Trinket,
        ])
    }
}

#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = EquippedBy, linked_spawn)]
#[reflect(Component)]
pub struct HasEquipped(Vec<Entity>);

/// [`Modifiers`] is a newtype over [`Parameters`] which is combined with
/// [`crate::parameters::BaseParameters`].
#[derive(Component, Default, Hash, Debug, Copy, Clone, Reflect, PartialEq, Eq)]
pub struct Modifiers(pub Parameters);

impl Modifiers {
    pub fn modify(&self, parameters: Parameters) -> Parameters {
        self.0 + parameters
    }
}

macro_rules! modifiers {
    ( $( $fieldn:tt: $fieldv:expr )* $(,)? ) => {
        Modifiers(Parameters {
            $( $fieldn: $fieldv, )*
            ..Default::default()
        })
    };
}
pub(crate) use modifiers;

#[derive(EntityEvent, Debug)]
pub struct ToggleEquipped {
    #[event_target]
    pub target: Entity,
    pub equipment: Entity,
}

/// Returns which item (if any) is equipped in `slot`.
fn in_slot(equipped: Vec<Entity>, q: &Query<&EquippedBy>, slot: Slot) -> Option<Entity> {
    equipped
        .into_iter()
        .find(|&e| q.get(e).is_ok_and(|eq| eq.slot == slot))
}

/// Handles [`ToggleEquipped`] by marking a piece of equipment as [`EquippedBy`],
/// removing [`CarriedBy`] accordingly, removing [`EquippedBy`] (and restoring
/// [`CarriedBy`]) for an item previously equipped in the same [`Slot`] (if any).
pub fn on_toggle_equipped(
    event: On<ToggleEquipped>,
    mut commands: Commands,
    all_equipment_sets: Query<Option<&HasEquipped>, With<Slots>>,
    all_equipped_itam: Query<&EquippedBy>,
    all_itam: Query<&ItemId>,
) {
    let ToggleEquipped { target, equipment } = *event;

    // Need `ItemId` to 1) see equipment def, and 2) log the change.
    let Ok(item_id) = all_itam.get(equipment) else {
        error!("no such item: {event:?}");
        return;
    };

    let Some(target_eq_slot) = item_id.equip_def().map(|it| it.slot) else {
        error!("unable to find target item {equipment:?} ({item_id}) as specified by {event:?}");
        return;
    };

    info!("toggle equipped: {item_id} {target_eq_slot:?} ({target} toggles {equipment})");

    let target_eq_list = match all_equipment_sets.get(target) {
        Ok(has_equipped_opt) => unwrap_collection(has_equipped_opt),
        _ => vec![],
    };

    if let Some(extant_eq) = in_slot(target_eq_list, &all_equipped_itam, target_eq_slot) {
        commands
            .entity(extant_eq)
            .remove::<EquippedBy>()
            .insert(CarriedBy(target));

        if extant_eq == equipment {
            commands.trigger(sounds::Unequip);
            return;
        }
    }

    commands
        .entity(equipment)
        .remove::<CarriedBy>()
        .insert(EquippedBy {
            entity: target,
            slot: target_eq_slot,
        })
        .commands()
        .trigger(sounds::Equip);
}
