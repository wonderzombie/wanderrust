use bevy::prelude::*;

use crate::parameters::Parameters;

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
