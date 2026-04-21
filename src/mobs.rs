use bevy::prelude::*;
use bevy_northstar::prelude::*;

use crate::{
    actors::{Dead, Player},
    cell::Cell,
    combat::Awareness,
    fov::{Fov, Vision},
    gamestate::Turn,
    inventory,
    loot::{FixedLoot, LootTable},
};

/// Checks each mob's status and alerts mobs when the player enters their FOV.
pub fn check_fov(
    mut commands: Commands,
    all_fov: Query<&Fov>,
    active_mobs: Populated<
        (Entity, &Awareness, &ChildOf, &Cell, &Vision),
        (With<AgentOfGrid>, Without<Dead>, Without<Player>),
    >,
    player_cell: Single<&Cell, With<Player>>,
) {
    let player_cell: (i32, i32) = (*player_cell).into();
    for (mob_entity, awareness, mob_child_of, mob_cell, mob_vision) in active_mobs.iter() {
        let Some(fov) = all_fov.get(mob_child_of.parent()).ok() else {
            warn!("No Fov found for entity {:?}", mob_child_of.parent());
            continue;
        };

        let view = fov.from(mob_cell.into(), mob_vision.range());

        if view.has(player_cell) && awareness < &Awareness::Alerted {
            commands
                .entity(mob_entity)
                .insert(Awareness::Alerted)
                .insert(Turn::Waiting);
            info!(
                "{:?} @ {} detected player at {:?}",
                mob_entity, mob_cell, player_cell
            );
        }
    }
}

pub fn handle_dead(
    query: Populated<(Option<&FixedLoot>, Option<&LootTable>), (With<Dead>, With<Turn>)>,
    mut acquisitions: MessageWriter<inventory::Acquisition>,
) {
    for (fixed_loot_opt, loot_opt) in &query {
        let mut acquired = inventory::Inventory::default();

        if let Some(loot) = loot_opt {
            acquired.extend(loot.roll());
        }

        if let Some(FixedLoot(fixed)) = fixed_loot_opt {
            acquired.extend(fixed.clone());
        }

        if !acquired.is_empty() {
            acquisitions.write(inventory::Acquisition { items: acquired });
        }
    }
}
