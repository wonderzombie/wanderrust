use bevy::prelude::*;

use crate::{inventory::Item, parameters::Parameters};

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

#[derive(Component, Copy, Clone)]
pub(crate) struct ParamsModifiers(pub Parameters);

#[derive(Component)]
pub(crate) struct Equippable(pub Item, pub ParamsModifiers);

impl Equippable {
    pub fn modify(&self, params: Parameters) -> Parameters {
        params + self.1.0
    }
}
