use bevy::prelude::*;

use crate::{inventory::Item, parameters::Parameters};

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
#[relationship_target(relationship = EquippedBy)]
pub struct HasEquipped(Vec<Entity>);

#[derive(Component, Default, Debug, Copy, Clone, Reflect)]
pub(crate) struct ParamsModifiers(pub Parameters);

#[derive(Component, Reflect, Debug, Clone)]
pub(crate) struct Equippable(pub Item, pub ParamsModifiers);

impl Equippable {
    pub fn modify(&self, params: Parameters) -> Parameters {
        params + self.1.0
    }
}
