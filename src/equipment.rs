use bevy::prelude::*;

use crate::{enum_with_str, inventory::Item, parameters::Parameters};

#[derive(Message, Debug, Clone, Reflect)]
pub struct Equipped {
    pub parent: Entity,
    pub item: Equippable,
}

#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = HasEquipped)]
pub struct EquippedBy {
    #[relationship]
    pub parent: Entity,
    pub item: Item,
}

#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = EquippedBy, linked_spawn)]
pub struct HasEquipped(Vec<Entity>);

#[derive(Component, Default, Hash, Debug, Copy, Clone, Reflect, PartialEq, Eq)]
pub(crate) struct Modifiers(pub Parameters);

#[derive(Component, Reflect, Debug, Clone)]
pub(crate) struct Equippable(pub Item, pub Modifiers);

impl Equippable {
    pub fn modify(&self, params: Parameters) -> Parameters {
        params + self.1.0
    }
}

enum_with_str!(Equipment, [Stick, Rags, Leather, Chainmail, Shield]);

macro_rules! modifiers {
    ( $( $fieldn:tt: $fieldv:expr )* $(,)? ) => {
        Modifiers(Parameters {
            $( $fieldn: $fieldv, )*
            ..Default::default()
        })
    };
}
pub(crate) use modifiers;

impl Equipment {
    pub(crate) fn modifiers(&self) -> Modifiers {
        match self {
            Equipment::Unset => Modifiers::default(),
            Equipment::Stick => modifiers!(attack: 1),
            Equipment::Rags => modifiers!(defense: 1),
            Equipment::Leather => modifiers!(defense: 3),
            Equipment::Chainmail => modifiers!(defense: 5),
            Equipment::Shield => modifiers!(defense: 2),
        }
    }

    pub fn as_item(&self) -> Option<Item> {
        Self::pairs()
            .iter()
            .find(|(_, v)| self == v)
            .copied()
            .map(|(s, _)| s)
            .map(Item::from)
    }
}
