use bevy::{platform::collections::HashMap, prelude::*};
use bevy_northstar::prelude::*;

use crate::{
    actors::Player,
    cell::Cell,
    combat::{self, Awareness},
    gamestate::Turn,
    tilemap::{Depth, Level, WorldId, WorldSpec},
    tiles::Walkable,
};

/// A spatial index that tracks which cells are occupied by non-walkable
/// entities in the world.
#[derive(Resource, Component, Default, Debug, PartialEq, Eq, Reflect)]
#[reflect(Resource)]
pub struct SpatialIndex {
    occupied: HashMap<Cell, Entity>,
}

impl SpatialIndex {
    pub fn clear(&mut self) {
        self.occupied.clear();
    }

    pub fn insert(&mut self, cell: Cell, entity: Entity) {
        self.occupied.insert(cell, entity);
    }

    pub fn get(&self, cell: Cell) -> Option<Entity> {
        self.occupied.get(&cell).copied()
    }
}

/// Updates [SpatialIndex] resource based on the current [Cell] of non-walkable
/// entities in the world.
pub(crate) fn update_spatial_index(
    query: Populated<(&Children, &mut SpatialIndex)>,
    tiles: Query<&Cell, Without<Walkable>>,
) {
    for (children, mut index) in query {
        index.clear();
        for &child in children {
            if let Ok(cell) = tiles.get(child) {
                index.insert(*cell, child);
            }
        }
    }
}

pub(crate) fn setup_spatial_indices(
    mut commands: Commands,
    level_children: Populated<(&Level, &Children)>,
    unwalkable_cells: Populated<&Cell, Without<Walkable>>,
) {
    for (Level(level_entity, _), children) in level_children.iter() {
        let mut index = SpatialIndex::default();
        for child in children.iter() {
            if let Ok(cell) = unwalkable_cells.get(child) {
                index.insert(*cell, child);
            }
        }
        commands.entity(*level_entity).insert(index);
    }
}

pub fn spawn_grid(
    mut commands: Commands,
    world_spec: Res<WorldSpec>,
    world_entity: Single<(&WorldId, &Children)>,
) {
    let (WorldId(nt), _) = *world_entity;
    let Depth(max_depth) = world_spec.depths.iter().max().copied().unwrap_or_default();
    assert!(
        max_depth >= 0,
        "negative world depths (i32) are not yet supported; `bevy_northstar` needs u32"
    );
    let world_height: u32 = max_depth.cast_unsigned();

    commands.entity(*nt).insert(CardinalGrid::new(
        &GridSettingsBuilder::new_3d(world_spec.grid_width, world_spec.grid_height, world_height)
            .chunk_size(8)
            .chunk_depth(1)
            .default_impassable()
            .build(),
    ));
}

pub fn update_grid(
    mut nav_grid: Single<&mut CardinalGrid>,
    changed_tiles: Populated<(&Cell, Has<Walkable>), Changed<Walkable>>,
) {
    let mut grid_changed = false;
    for (cell, is_walkable) in changed_tiles {
        if !nav_grid.in_bounds(cell.as_vec3()) {
            error!(
                "Skipping attempt to update grid at out-of-bounds position {}; grid is {} x {}",
                cell,
                nav_grid.width(),
                nav_grid.height(),
            );
            error_once!("grid dumped: {:?}", nav_grid.view());
            continue;
        }

        let prev_nav = nav_grid.nav(cell.into());
        let next_nav = if is_walkable {
            Nav::Passable(1)
        } else {
            Nav::Impassable
        };

        if prev_nav != Some(next_nav) {
            grid_changed = true;
            nav_grid.set_nav(cell.into(), next_nav);
        }
    }

    if grid_changed {
        info!("updated world grid");
        nav_grid.build();
    }
}

pub fn init_agents(
    mut commands: Commands,
    grid: Single<Entity, With<CardinalGrid>>,
    query: Populated<(Entity, &Cell), (With<Awareness>, Without<AgentOfGrid>)>,
) {
    let grid_nt = grid.into_inner();
    for (entity, cell) in query {
        commands
            .entity(entity)
            .insert((AgentPos(cell.into()), AgentOfGrid(grid_nt)));
    }
}

pub fn pathfind(
    mut commands: Commands,
    player_cell: Single<&Cell, With<Player>>,
    query: Populated<(Entity, &Awareness, Option<&Pathfind>)>,
) {
    let player_cell: Cell = *player_cell.into_inner();
    for (entity, awareness, pathfind) in &query {
        if *awareness != Awareness::Alerted {
            continue;
        }

        if pathfind.is_none_or(|pf| pf.goal.eq(&player_cell.into())) {
            commands
                .entity(entity)
                .insert(Pathfind::new_2d(player_cell.x as u32, player_cell.y as u32));
        }
    }
}

pub fn move_agents(
    mut query: Populated<(Entity, &mut AgentPos, &NextPos, &mut Turn)>,
    player: Single<(Entity, &Cell), With<Player>>,
    mut attacks: MessageWriter<combat::Attack>,
    mut commands: Commands,
) {
    for (entity, mut agent_pos, next_pos, mut turn) in query.iter_mut() {
        if turn.complete() {
            trace!("not moving done/idle entity {:?}", entity);
            continue;
        }

        trace!(
            "🧭 entity {} moving from {:?} to {:?}",
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
