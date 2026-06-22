use bevy::{platform::collections::HashMap, prelude::*};
use bevy_northstar::prelude::*;

use crate::{
    actors::{Dead, Player},
    cell::Cell,
    combat,
    gamestate::{NextTurn, Turn, WorldClock},
    parameters::{Awareness, Parameters},
    tilemap::{Depth, Level, WorldId, WorldSpec},
    tiles::{TileIdx, Walkable},
};

/// A spatial index that tracks which cells are occupied by non-walkable
/// entities in the world.
#[derive(Component, Default, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
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
    tiles: Query<&Cell, (Without<Walkable>, Without<Dead>)>,
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
    unwalkable_cells: Populated<(Entity, &Cell), Without<Walkable>>,
) {
    for (Level(level_entity, _), children) in level_children.iter() {
        let mut index = SpatialIndex::default();
        for (nt, cell) in unwalkable_cells.iter_many(children) {
            index.insert(*cell, nt);
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
    let Depth(max_depth) = world_spec.max_depth;
    let world_height: u32 = max_depth.cast_unsigned();

    commands.entity(*nt).insert(CardinalGrid::new(
        &GridSettingsBuilder::new_3d(
            world_spec.grid_width,
            world_spec.grid_height,
            world_height + 1,
        )
        .chunk_size(8)
        .chunk_depth(1)
        .default_impassable()
        .build(),
    ));
}

pub fn update_grid(
    mut nav_grid: Single<&mut CardinalGrid>,
    changed_tiles: Populated<(&Cell, Has<Walkable>), Changed<TileIdx>>,
) {
    let mut grid_changed = false;
    for (cell, is_walkable) in changed_tiles {
        if !nav_grid.in_bounds(cell.as_vec3()) {
            error!(
                "Skipping attempt to update grid at out-of-bounds position {cell}; grid is {} x {}",
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
    query: Populated<(Entity, &Cell, Has<Player>), (With<Awareness>, Without<AgentOfGrid>)>,
) {
    let grid_nt = grid.into_inner();
    for (entity, cell, is_player) in query {
        let mut e = commands.entity(entity);
        e.insert((AgentPos(cell.into()), AgentOfGrid(grid_nt)));
        if !is_player {
            e.insert(Blocking);
        }
    }
}

pub fn pathfind(
    mut commands: Commands,
    player_cell: Single<&Cell, With<Player>>,
    query: Populated<(Entity, &Awareness, Option<&Pathfind>), Without<Player>>,
) {
    let player_cell: Cell = *player_cell.into_inner();
    for (entity, awareness, pathfind) in &query {
        if *awareness != Awareness::Alerted {
            continue;
        }

        if pathfind.is_none_or(|pf| pf.goal.ne(&player_cell.into())) {
            commands
                .entity(entity)
                .insert(Pathfind::new_2d(player_cell.x as u32, player_cell.y as u32));
        }
    }
}

pub fn move_agents(
    mut query: Populated<(NameOrEntity, &mut AgentPos, &NextPos, &Parameters), With<Turn>>,
    player: Single<(Entity, &Cell), With<Player>>,
    mut attacks: MessageWriter<combat::Attack>,
    blocking: Res<BlockingMap>,
    mut commands: Commands,
    next_turn: If<Res<NextTurn>>,
    clock: Res<WorldClock>,
) {
    let (player, player_cell) = *player;

    let NextTurn(entity) = **next_turn;
    // Consume the current turn regardless.
    commands.remove_resource::<NextTurn>();

    let Ok((name, mut agent_pos, next_pos, params)) = query.get_mut(entity) else {
        error!("next turn is assigned to an entity that doesn't exist; skipping");
        return;
    };

    let target = next_pos.0;

    trace!("🧭 entity {name} {entity:?}: from {agent_pos:?} to {next_pos:?}",);

    // Since the player is non-Blocking, the path is allowed to land on the Player.
    // We process this as an attack and no movement occurs.
    if target == player_cell.as_vec3() {
        commands
            .entity(name.entity)
            .insert(clock.recovery_after(params.attack_speed));
        attacks.write(combat::Attack {
            attacker: entity,
            target: player,
        });
        return;
    }

    if blocking.0.get(&target).is_some() {
        // Skip movement for this turn. The pathfinding pipeline will have
        // the opportunity to recompute the next step in case that's needed.
        // This avoids a stale NextPos that may no longer be valid the next
        // time this system runs. A full move_speed recovery cost is charged
        // so the clock still advances and the scheduler isn't monopolized.
        commands
            .entity(entity)
            .remove::<NextPos>()
            .insert(clock.recovery_after(params.move_speed));
        return;
    }

    // The target position is not blocked by another non-player agent.
    // Set the AgentPos (the grid's model of occupancy) to the new position.
    agent_pos.0 = target;
    // Remove `NextPos` and update the Cell using the new AgentPos, thus
    // syncing the bevy_northstar grid with the wanderrust grid, thus
    // actuating the move from wanderrust's perspective.
    commands
        .entity(entity)
        .remove::<NextPos>()
        .insert(clock.recovery_after(params.move_speed))
        .insert(Cell::at_grid_coords(agent_pos.as_ref()));
}
