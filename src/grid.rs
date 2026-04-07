use bevy::{platform::collections::HashSet, prelude::*};
use bevy_northstar::prelude::*;

use crate::{
    actors::{Alerted, Player},
    cell::Cell,
    combat,
    gamestate::Turn,
    tilemap::{Stratum, TilemapSpec},
    tiles::{TileIdx, Walkable},
};

type CardinalGrid = Grid<CardinalNeighborhood>;

pub fn spawn_grid(
    mut commands: Commands,
    spec: Res<TilemapSpec>,
    strata: Query<Entity, With<Stratum>>,
) {
    for stratum in strata {
        let grid_settings = GridSettingsBuilder::new_2d(spec.size.width, spec.size.height)
            .chunk_size(16)
            .default_impassable()
            .build();

        let grid = Grid::<CardinalNeighborhood>::new(&grid_settings);

        commands.entity(stratum).insert(grid);
    }
}

pub fn update_grid(
    mut grid: Query<(Entity, &mut CardinalGrid)>,
    changed_tiles: Query<(&Cell, &ChildOf, Option<&Walkable>), Changed<TileIdx>>,
) {
    let mut count = 0;
    let mut changed_grids: HashSet<Entity> = HashSet::new();

    for (cell, child_of, walkable_opt) in changed_tiles {
        let Some((entity, mut grid)) = grid.get_mut(child_of.0).ok() else {
            error!(
                "failed to get grid for cell {:?} child_of {:?}",
                cell, child_of
            );
            continue;
        };

        let prev_nav = grid.nav(cell.into());
        let next_nav = if walkable_opt.is_some() {
            Nav::Passable(1)
        } else {
            Nav::Impassable
        };

        // This handles the case where `prev_nav` is `None`, or when
        // `Some(prev_nav) != Some(next_nav)`.
        if prev_nav != Some(next_nav) {
            grid.set_nav(cell.into(), next_nav);
            changed_grids.insert(entity);
            count += 1;
        }
    }

    changed_grids.iter().for_each(|&entity| {
        if let Ok((_, mut grid)) = grid.get_mut(entity) {
            grid.build();
        }
    });

    if count > 0 {
        info!("ℹ️\tupdated grid, set {} tiles", count);
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
