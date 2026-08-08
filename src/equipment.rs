use bevy::{
    ecs::query::{QueryData, QueryFilter},
    platform::collections::HashMap,
    prelude::*,
};

use crate::{
    actors::Player,
    inventory::CarriedBy,
    items::{ItemId, Quantity, Slot},
    parameters::Parameters,
};

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, handle_toggle_equip)
        .add_message::<ToggleEquip>();
}

#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = HasEquipped)]
#[reflect(Component)]
pub struct EquippedBy(pub Entity);

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

#[derive(QueryFilter)]
pub(crate) struct WithOwnedItam {
    or: Or<(With<CarriedBy>, With<HasEquipped>)>,
}

#[derive(QueryData)]
#[query_data(derive(Debug))]
pub(crate) struct Itam {
    entity: Entity,
    item_id: &'static ItemId,
    quantity: Option<&'static Quantity>,
    ownership: Ownership,
}

impl<'w, 's> ItamItem<'w, 's> {
    pub(crate) fn slot(&self) -> Option<Slot> {
        self.item_id.equip().map(|it| it.slot)
    }

    pub(crate) fn uses_slot(&self, slot: Slot) -> bool {
        self.slot() == Some(slot)
    }
}

#[derive(QueryData)]
#[query_data(derive(Debug))]
pub(crate) struct Ownership {
    carrier_opt: Option<&'static CarriedBy>,
    equipper_opt: Option<&'static EquippedBy>,
}

impl<'w, 's> OwnershipItem<'w, 's> {
    // pub(crate) fn carried_by(&self) -> Option<Entity> {
    //     self.carrier_opt.map(|it| it.0)
    // }

    pub(crate) fn equipped_by(&self) -> Option<Entity> {
        self.equipper_opt.map(|it| it.0)
    }

    pub(crate) fn is_equipped_by(&self, entity: Entity) -> bool {
        self.equipped_by()
            .map(|it| it == entity)
            .unwrap_or_default()
    }

    // pub(crate) fn is_carried_by(&self, entity: Entity) -> bool {
    //     self.carried_by().map(|it| it == entity).unwrap_or_default()
    // }
}

pub(crate) fn handle_toggle_equip(
    mut toggle_equip: MessageReader<ToggleEquip>,
    mut commands: Commands,
    obtained_items: Query<Itam, WithOwnedItam>,
    equipped_items: Single<Option<&HasEquipped>, With<Player>>,
) {
    for event in toggle_equip.read() {
        let ToggleEquip { target, equipment } = *event;

        let Ok(to_equip) = obtained_items.get(equipment) else {
            error!("entity is carrying no items, has nothing equipped; can't (un)equip");
            return;
        };

        info!(
            "equipping {}  {:?}",
            to_equip.item_id.def(),
            to_equip.item_id.equip()
        );

        let equipped_itam =
            obtained_items.iter_many(equipped_items.map(|it| it.iter()).unwrap_or_default());

        commands.entity(to_equip.entity).insert(EquippedBy(target));
    }
}
