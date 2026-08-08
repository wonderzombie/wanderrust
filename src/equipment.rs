use bevy::prelude::*;

use crate::{
    colors,
    items::{EquipDef, ItemId, Slot},
    message_log::LogEvent,
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

fn in_slot(equipped: Vec<Entity>, q: &Query<&EquippedBy>, slot: Slot) -> Option<Entity> {
    equipped
        .into_iter()
        .find(|&e| q.get(e).is_ok_and(|eq| eq.slot == slot))
}

fn list_collection<T: Component + RelationshipTarget>(
    collections: &Query<Option<&T>>,
    target_collection: Entity,
) -> Vec<Entity> {
    match collections.get(target_collection) {
        Ok(Some(found_coll)) => found_coll.iter().collect(),
        _ => vec![],
    }
}

pub(crate) fn handle_toggle_equip(
    mut commands: Commands,
    mut toggle_equip: PopulatedMessageReader<ToggleEquip>,
    all_equipment_sets: Query<Option<&HasEquipped>>,
    all_equipped_items: Query<&EquippedBy>,
    all_items: Query<&ItemId>,
    mut log: MessageWriter<LogEvent>,
) {
    for event in toggle_equip.read() {
        let ToggleEquip { target, equipment } = *event;

        let Ok(item_id) = all_items.get(equipment) else {
            error!("no such item {equipment}");
            continue;
        };

        let Some(target_equipment_def) = item_id.equip_def() else {
            error!("unable to find target item {equipment:?} as specified by {event:#?}");
            continue;
        };

        let eq_list = list_collection(&all_equipment_sets, target);

        if let Some(nt) = in_slot(eq_list, &all_equipped_items, target_equipment_def.slot) {
            info!("unequipping {:?}", nt);
            commands.entity(nt).remove::<EquippedBy>();
            // If this item entity was the target of this operation, we're done.
            if nt == equipment {
                info!("only unequipping {nt:?} because target was {equipment:?}");
                return;
            }
        } else {
            info!("no previous item in {:?}", target_equipment_def.slot);
        }

        info!("equipping {:?}", target_equipment_def);
        commands.entity(equipment).insert(EquippedBy {
            entity: target,
            slot: target_equipment_def.slot,
        });
        log.write(LogEvent {
            txt: format!("equipped {}", item_id.def()),
            color: Some(colors::KENNEY_GREEN),
        });
    }
}
