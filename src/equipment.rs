use bevy::prelude::*;

use crate::{
    items::{EquipDef, ItemId, Slot},
    parameters::Parameters,
};

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, handle_toggle_equip)
        .add_message::<ToggleEquip>();
}

#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = HasEquipped)]
#[reflect(Component)]
pub struct EquippedBy {
    #[relationship]
    pub entity: Entity,
    pub slot: Slot,
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct Slots(pub Vec<[Slot; 4]>);

#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = EquippedBy, linked_spawn)]
#[reflect(Component)]
pub struct HasEquipped(Vec<Entity>);

impl IntoIterator for HasEquipped {
    type Item = Entity;

    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

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

#[derive(Message, Debug)]
pub(crate) struct ToggleEquip {
    pub(crate) target: Entity,
    pub(crate) equipment: Entity,
}

fn in_slot(equipped: &HasEquipped, q: &Query<&EquippedBy>, slot: Slot) -> Option<Entity> {
    equipped
        .iter()
        .find(|&e| q.get(e).is_ok_and(|eq| eq.slot == slot))
}

fn equip_def(item: Entity, all_items: &Query<&ItemId>) -> Option<EquipDef> {
    all_items.get(item).ok().and_then(|it| it.equip_def())
}

pub(crate) fn handle_toggle_equip(
    mut commands: Commands,
    mut toggle_equip: PopulatedMessageReader<ToggleEquip>,
    all_equipment_sets: Query<&HasEquipped>,
    all_equipped_items: Query<&EquippedBy>,
    all_items: Query<&ItemId>,
) {
    for event in toggle_equip.read() {
        let ToggleEquip { target, equipment } = *event;

        let Ok(target_equipment_set) = all_equipment_sets.get(target) else {
            error!("unable to find target equipper {target:?} as specified by {event:#?}");
            continue;
        };

        let Some(target_equipment) = equip_def(equipment, &all_items) else {
            error!("unable to find target item {equipment:?} as specified by {event:#?}");
            continue;
        };

        match in_slot(
            target_equipment_set,
            &all_equipped_items,
            target_equipment.slot,
        ) {
            Some(nt) => {
                info!("unequipping {:?}", target_equipment);
                commands.entity(nt).remove::<EquippedBy>();
            }
            None => (),
        }

        info!("equipping {:?}", target_equipment);
        commands.entity(equipment).insert(EquippedBy {
            entity: target,
            slot: target_equipment.slot,
        });
    }
}
