use bevy::{platform::collections::HashMap, prelude::*};
use bevy_northstar::prelude::*;

use crate::{
    actors::{Dead, Player},
    cell::Cell,
    parameters::Awareness,
    tilemap::{ActiveLevel, Level, WorldId, WorldSpec},
    tiles::{MapTile, TileIdx, Walkable},
};

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, sync_agent_pos.before(PathingSet));
}

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
    world_entity: Single<&WorldId>,
) {
    let WorldId(nt) = *world_entity;

    commands.entity(*nt).insert(CardinalGrid::new(
        &GridSettingsBuilder::new_2d(world_spec.grid_width, world_spec.grid_height)
            .chunk_size(8)
            .default_impassable()
            .build(),
    ));
}

pub fn rebuild_grid(
    mut nav_grid: Single<&mut CardinalGrid>,
    active_level: Single<(Ref<ActiveLevel>, &Children)>,
    map_tiles: Query<(&Cell, Has<Walkable>), With<MapTile>>,
    blockers: Query<
        &Cell,
        (
            With<TileIdx>,
            Without<MapTile>,
            Without<Awareness>,
            Without<Walkable>,
            Without<Dead>,
        ),
    >,
    changed_tiles: Query<(), (Changed<TileIdx>, Without<Awareness>)>,
) {
    let (active, children) = *active_level;

    if changed_tiles.is_empty() || !active.is_changed() {
        return;
    }

    for y in 0..nav_grid.height() {
        for x in 0..nav_grid.width() {
            nav_grid.set_nav(uvec3(x, y, 0), Nav::Impassable);
        }
    }

    let mut passable = 0;
    for (cell, is_walkable) in map_tiles.iter_many(children) {
        if !is_walkable {
            continue;
        }

        let nav_pos = cell.nav_pos();
        if !nav_grid.in_bounds(nav_pos) {
            error!(
                "Skipping attempt to update grid at out-of-bounds position {cell}; grid is {} x {}",
                nav_grid.width(),
                nav_grid.height(),
            );
            error_once!("grid dumped: {:?}", nav_grid.view());
            continue;
        }
        nav_grid.set_nav(nav_pos, Nav::Passable(1));
        passable += 1;
    }

    let mut blocked = 0;
    for cell in blockers.iter_many(children) {
        let nav_pos = cell.nav_pos();
        if nav_grid.in_bounds(nav_pos) {
            nav_grid.set_nav(nav_pos, Nav::Impassable);
            blocked += 1;
        }
    }

    nav_grid.build();
    info!("rebuild_grid: passable/blocked {passable}/{blocked}");
}

pub fn init_agents(
    mut commands: Commands,
    grid: Single<Entity, With<CardinalGrid>>,
    query: Populated<(Entity, &Cell, Has<Player>), (With<Awareness>, Without<AgentOfGrid>)>,
) {
    let grid_nt = grid.into_inner();
    for (entity, cell, is_player) in query {
        let mut e = commands.entity(entity);
        e.insert((AgentPos(cell.nav_pos()), AgentOfGrid(grid_nt)));
        if !is_player {
            e.insert(Blocking);
        }
    }
}

pub fn pathfind(
    mut commands: Commands,
    player_cell: Single<&Cell, With<Player>>,
    query: Populated<
        (
            Entity,
            &Awareness,
            Option<&Pathfind>,
            Has<PathfindingFailed>,
        ),
        Without<Player>,
    >,
) {
    let player_cell: Cell = *player_cell.into_inner();
    for (entity, awareness, pathfind, failed_pathfind) in &query {
        if *awareness != Awareness::Alerted {
            continue;
        }

        if failed_pathfind || pathfind.is_none_or(|pf| pf.goal.ne(&player_cell.nav_pos())) {
            commands
                .entity(entity)
                .insert(Pathfind::new_2d(player_cell.x as u32, player_cell.y as u32));
        }
    }
}

pub(crate) fn sync_agent_pos(agents: Populated<(&Cell, &mut AgentPos), Changed<Cell>>) {
    for (cell, mut pos) in agents {
        pos.set_if_neq(AgentPos(cell.nav_pos()));
    }
}
