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
    tilemap::{ActiveLevel, Zone},
};

/// Checks each mob's status and alerts mobs when the player enters their FOV.
pub fn check_fov(
    mut commands: Commands,
    all_fov: Query<&Fov>,
    active_zone: Single<&Zone, With<ActiveLevel>>,
    active_mobs: Populated<
        (Entity, &Awareness, &ChildOf, &Cell, &Vision),
        (With<AgentOfGrid>, Without<Dead>, Without<Player>),
    >,
    player_cell: Single<&Cell, With<Player>>,
) {
    let player_cell: (i32, i32) = (*player_cell).into();

    let entities = active_zone.into_inner().iter();

    for entity in entities {
        let Ok((_, awareness, child_of, cell, vision)) = active_mobs.get(entity) else {
            continue;
        };
        let Some(fov) = all_fov.get(child_of.parent()).ok() else {
            warn!("no Fov found for entity {:?}", child_of.parent());
            continue;
        };

        let view = fov.from(cell.into(), vision.range());

        if view.has(player_cell) && awareness < &Awareness::Alerted {
            commands
                .entity(entity)
                .insert(Awareness::Alerted)
                .insert(Turn::Waiting);
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
