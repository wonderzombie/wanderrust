use bevy::prelude::*;

use crate::{
    equipment::{Equippable, HasEquipped},
    parameters::Parameters,
};

pub fn apply_params_modifiers(
    mut commands: Commands,
    affected: Query<(Entity, &HasEquipped, &Parameters), Changed<HasEquipped>>,
    equipment: Query<&Equippable>,
) {
    for (nt, has_equipped, base_params) in affected {
        let modified = has_equipped
            .iter()
            .filter_map(|e| equipment.get(e).ok())
            .fold(base_params.clone(), |acc, eq| eq.modify(acc));

        // commands.entity(nt).insert(DerivedParams::new(modified));
    }
}
