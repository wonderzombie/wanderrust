use bevy::prelude::*;
use bevy_northstar::prelude::*;

use crate::{
    actors::{Alerted, Dead, Player},
    cell::Cell,
    combat,
    fov::{Fov, Vision},
    gamestate::Turn,
    inventory,
    loot::{FixedLoot, LootTable},
};

pub fn check_fov(
    mut commands: Commands,
    all_fov: Query<&Fov>,
    active_mobs: Query<
        (Entity, &ChildOf, &Cell, &Vision),
        (
            With<AgentOfGrid>,
            Without<Alerted>,
            Without<Dead>,
            Without<Player>,
        ),
    >,
    player_cell: Single<&Cell, With<Player>>,
) {
    let player_cell: (i32, i32) = (*player_cell).into();
    for (mob_entity, mob_child_of, mob_cell, mob_vision) in active_mobs.iter() {
        let Some(fov) = all_fov.get(mob_child_of.parent()).ok() else {
            warn!("No Fov found for entity {:?}", mob_child_of.parent());
            continue;
        };

        let view = fov.from(mob_cell.into(), mob_vision.range());

        if view.has(player_cell) {
            commands
                .entity(mob_entity)
                .insert(Alerted)
                .insert(Turn::Waiting);
            info!(
                "{:?} @ {} detected player at {:?}",
                mob_entity, mob_cell, player_cell
            );
        }
    }
}

pub fn pathfind(
    mut commands: Commands,
    player: Single<&Cell, With<Player>>,
    query: Query<Entity, (Without<Pathfind>, With<Alerted>)>,
) {
    for entity in &query {
        commands
            .entity(entity)
            .insert(Pathfind::new_2d(player.x as u32, player.y as u32));
    }
}

pub fn move_agents(
    mut query: Query<(Entity, &mut AgentPos, &NextPos, &mut Turn)>,
    player: Single<(Entity, &Cell), With<Player>>,
    mut attacks: MessageWriter<combat::Attack>,
    mut commands: Commands,
) {
    for (entity, mut agent_pos, next_pos, mut turn) in query.iter_mut() {
        if turn.complete() {
            info!("not moving done/idle entity {:?}", entity);
            continue;
        }

        trace!(
            "ℹ️ entity {} moving from {:?} to {:?}",
            entity, agent_pos, next_pos
        );

        let (player, player_cell) = *player;

        if next_pos.0 == player_cell.as_vec3() {
            attacks.write(combat::Attack {
                attacker: entity,
                target: player,
            });
        } else {
            agent_pos.0 = next_pos.0;
            commands
                .entity(entity)
                .remove::<NextPos>()
                .insert(Cell::at_grid_coords(agent_pos.as_ref()));
        }
        *turn = Turn::Done;
    }
}

pub fn handle_dead(
    query: Query<(Option<&FixedLoot>, Option<&LootTable>), (With<Dead>, With<Turn>)>,
    mut acquisitions: MessageWriter<inventory::Acquisition>,
) {
    for (fixed_loot_opt, loot_opt) in &query {
        let mut acquired = inventory::Inventory::default();

        if let Some(loot) = loot_opt {
            acquired += loot.roll();
        }

        if let Some(fixed) = fixed_loot_opt {
            acquired += fixed.0.clone();
        }

        if !acquired.is_empty() {
            acquisitions.write(inventory::Acquisition { items: acquired });
        }
    }
}
