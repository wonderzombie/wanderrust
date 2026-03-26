use bevy::prelude::*;
use bevy_northstar::prelude::*;


use crate::{
    actors::{Alerted, Dead, Player},
    cell::Cell,
    combat,
    fov::{Fov, Vision},
    gamestate::Turn,
    inventory,
    loot::{FixedLoot, RandLootTable},
    tiles::TileIdx,
};

pub fn check_fov(
    mut commands: Commands,
    fov: Res<Fov>,
    visions: Query<(Entity, &TileIdx, &Cell, &Vision), (With<AgentOfGrid>, Without<Player>)>,
    player: Query<&Cell, With<Player>>,
) {
    let Some(player_cell) = player.single().ok() else {
        return;
    };
    for (entity, tile, mob_cell, mob_vision) in visions.iter() {
        let view = fov.from(mob_cell.into(), mob_vision.0);
        if view.has(player_cell.into()) {
            commands
                .entity(entity)
                .insert(Alerted)
                .insert(Turn::Waiting);
            info!(
                "{:?} @ {} detected player at {}",
                tile, mob_cell, player_cell
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
    mut attacks: MessageWriter<combat::AttackAttempt>,
    mut commands: Commands,
) {
    for (entity, mut agent_pos, next_pos, mut turn) in query.iter_mut() {
        if turn.complete() {
            info!("not moving done/idle entity {:?}", entity);
            continue;
        }

        info!(
            "entity {} moving from {:?} to {:?}",
            entity, agent_pos, next_pos
        );

        let (player, player_cell) = *player;

        if next_pos.0 == player_cell.as_vec3() {
            attacks.write(combat::AttackAttempt {
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
    mut commands: Commands,
    mob_loot: Res<RandLootTable>,
    query: Query<(Entity, &TileIdx, Option<&FixedLoot>), (With<Dead>, With<Turn>)>,
    mut acquisitions: MessageWriter<inventory::Acquisition>,
) {
    for (entity, tile_idx, loot_opt) in &query {
        commands.entity(entity).remove::<Turn>();

        let mut acquired = inventory::Inventory::default();

        if let Some(loot) = loot_opt {
            acquired += loot.0.clone();
        }

        if mob_loot.contains_key(tile_idx) {
            let loot = mob_loot.roll(*tile_idx);
            acquired += loot;
        }

        if !acquired.is_empty() {
            acquisitions.write(inventory::Acquisition { items: acquired });
        }
    }
}
