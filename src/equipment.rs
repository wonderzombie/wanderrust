use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};

use crate::{
    colors,
    items::{ItemId, Slot},
    message_log::LogEvent,
    parameters::Parameters,
};

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, handle_toggle_equip)
        .add_message::<ToggleEquip>();
}

#[derive(Message, Debug, Default)]
pub struct EquipmentChanged;

#[derive(Component, Reflect, Debug)]
#[component(on_add = notify, on_insert = notify, on_discard = notify, on_remove = notify)]
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

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct Slots(pub [Slot; 4]);

impl Slots {
    pub fn standard() -> Self {
        Self([Slot::Armor, Slot::MainHand, Slot::OffHand, Slot::Trinket])
    }
}

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

pub fn unwrap_collection<T, O>(collection: Option<&T>) -> O
where
    T: Component + RelationshipTarget,
    O: FromIterator<Entity> + Default,
{
    collection
        .map(|coll| coll.iter().collect::<O>())
        .unwrap_or_default()
}

pub(crate) fn handle_toggle_equip(
    mut commands: Commands,
    mut toggle_equip: PopulatedMessageReader<ToggleEquip>,
    all_equipment_sets: Query<Option<&HasEquipped>, With<Slots>>,
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

        let eq_list = match all_equipment_sets.get(target) {
            Ok(found_coll) => unwrap_collection(found_coll),
            _ => vec![],
        };

        if let Some(nt) = in_slot(eq_list, &all_equipped_items, target_equipment_def.slot) {
            info!("unequipping {:?}", nt);
            commands.entity(nt).remove::<EquippedBy>();
            // If this item entity was the target of this operation, we're done.
            if nt == equipment {
                info!("only unequipping {nt:?} because target was {equipment:?}");
                continue;
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
