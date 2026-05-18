use bevy::prelude::*;
use bevy_northstar::prelude::*;

use crate::{
    actors::{Dead, Player},
    cell::Cell,
    combat::{Awareness, Parameters},
    fov::Fov,
    gamestate::Turn,
    inventory,
    loot::{FixedLoot, LootTable},
    tilemap::{ActiveLevel, Zone},
};

/// Checks each mob's status and alerts mobs when the player enters their FOV.
pub fn check_fov(
    mut commands: Commands,
    active_zone: Single<(&Fov, &Zone), With<ActiveLevel>>,
    active_mobs: Populated<(&Awareness, &Cell, &Parameters), (With<AgentOfGrid>, Without<Dead>)>,
    player_cell: Single<&Cell, With<Player>>,
) {
    let player_cell: (i32, i32) = (*player_cell).into();

    let (fov, entities) = active_zone.into_inner();

    for entity in entities.iter() {
        let Ok((awareness, cell, params)) = active_mobs.get(entity) else {
            continue;
        };

        let view = fov.from(cell.into(), params.vision.range());

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
