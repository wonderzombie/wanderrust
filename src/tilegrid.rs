use bevy::{
    ecs::{
        entity::Entity,
        query::With,
        resource::Resource,
        system::{Commands, Query, Res, ResMut},
    },
    log::info,
};

use crate::{cell::Cell, map::MapSpec, tiles::MapTile};

#[derive(Resource, Debug)]
pub struct TileGrid {
    width: usize,
    height: usize,
    tiles: Vec<Option<Entity>>,
}

impl TileGrid {
    pub(crate) fn new(size: (u32, u32)) -> Self {
        info!(
            "TileGrid: {} by {} ({} tiles)",
            size.0,
            size.1,
            size.0 * size.1
        );
        Self {
            width: size.0 as usize,
            height: size.1 as usize,
            tiles: vec![None; size.0 as usize * size.1 as usize],
        }
    }

    #[inline]
    pub fn to_idx(&self, cell: &Cell) -> usize {
        cell.y as usize * self.width + cell.x as usize
    }

    #[inline]
    pub(crate) fn get(&self, cell: &Cell) -> Option<Entity> {
        let idx = self.to_idx(cell);
        self.tiles.get(idx).copied().flatten()
    }

    #[inline]
    pub(crate) fn get_idx(&self, idx: usize) -> Option<Entity> {
        self.tiles.get(idx).copied().flatten()
    }

    pub(crate) fn set(&mut self, cell: &Cell, entity: Entity) {
        let idx = self.to_idx(cell);
        if let Some(slot) = self.tiles.get_mut(idx) {
            *slot = Some(entity);
        }
    }

    pub(crate) fn remove(&mut self, cell: &Cell) {
        let idx = self.to_idx(cell);
        if let Some(slot) = self.tiles.get_mut(idx) {
            *slot = None;
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (Cell, Entity)> + '_ {
        self.tiles.iter().enumerate().filter_map(|(idx, entity)| {
            entity.map(|e| {
                let x = (idx as u32) % self.width as u32;
                let y = (idx as u32) / self.width as u32;
                (Cell::new(x as i32, y as i32), e)
            })
        })
    }
}

pub fn init_tilegrid(
    mut commands: Commands,
    spec: Res<MapSpec>,
    tiles: Query<(&Cell, Entity), With<MapTile>>,
) {
    info!("init tilegrid");
    let mut tilegrid = TileGrid::new((spec.size.x, spec.size.y));
    for (cell, entity) in tiles {
        tilegrid.set(cell, entity);
    }
    commands.insert_resource::<TileGrid>(tilegrid);
}

pub fn setup_tilegrid(
    mut tilegrid: ResMut<TileGrid>,
    tiles: Query<(&Cell, Entity), With<MapTile>>,
) {
    info!("setup tilegrid");
    assert_ne!(tilegrid.tiles.len(), 0);
    for (cell, entity) in tiles {
        let idx = tilegrid.to_idx(cell);
        info!("setting up {:?} {}", cell, idx);
        tilegrid.set(cell, entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::entity::Entity;

    fn entity(index: u32) -> Entity {
        Entity::from_bits(index as u64)
    }

    fn make_grid(w: u32, h: u32) -> TileGrid {
        TileGrid {
            width: w as usize,
            height: h as usize,
            tiles: vec![None; (w * h) as usize],
        }
    }

    #[test]
    fn test_to_idx_origin() {
        let grid = make_grid(10, 10);
        assert_eq!(grid.to_idx(&Cell::new(0, 0)), 0);
    }

    #[test]
    fn test_to_idx_row_major() {
        let grid = make_grid(10, 10);
        // row 2, col 3 => 2*10 + 3 = 23
        assert_eq!(grid.to_idx(&Cell::new(3, 2)), 23);
    }

    #[test]
    fn test_set_and_get() {
        let mut grid = make_grid(5, 5);
        let e = entity(42);
        let cell = Cell::new(2, 3);
        grid.set(&cell, e);
        assert_eq!(grid.get(&cell), Some(e));
    }

    #[test]
    fn test_get_empty_returns_none() {
        let grid = make_grid(5, 5);
        assert_eq!(grid.get(&Cell::new(0, 0)), None);
    }

    #[test]
    fn test_get_idx_matches_to_idx() {
        let mut grid = make_grid(5, 5);
        let e = entity(7);
        let cell = Cell::new(3, 1);
        grid.set(&cell, e);
        let idx = grid.to_idx(&cell);
        assert_eq!(grid.get_idx(idx), Some(e));
    }

    #[test]
    fn test_get_idx_out_of_bounds_returns_none() {
        let grid = make_grid(5, 5);
        assert_eq!(grid.get_idx(999), None);
    }

    #[test]
    fn test_remove_clears_cell() {
        let mut grid = make_grid(5, 5);
        let cell = Cell::new(1, 1);
        grid.set(&cell, entity(1));
        grid.remove(&cell);
        assert_eq!(grid.get(&cell), None);
    }

    #[test]
    fn test_remove_empty_cell_is_noop() {
        let mut grid = make_grid(5, 5);
        // should not panic
        grid.remove(&Cell::new(2, 2));
        assert_eq!(grid.get(&Cell::new(2, 2)), None);
    }

    #[test]
    fn test_set_out_of_bounds_is_noop() {
        let mut grid = make_grid(5, 5);
        // should not panic, and grid should remain unmodified
        grid.set(&Cell::new(10, 10), entity(99));
    }

    #[test]
    fn test_iter_yields_only_occupied_cells() {
        let mut grid = make_grid(3, 3);
        grid.set(&Cell::new(0, 0), entity(1));
        grid.set(&Cell::new(2, 2), entity(2));
        let collected: Vec<_> = grid.iter().collect();
        assert_eq!(collected.len(), 2);
    }

    #[test]
    fn test_iter_roundtrips_coordinates() {
        let mut grid = make_grid(5, 5);
        let cell = Cell::new(3, 4);
        grid.set(&cell, entity(7));
        let (found_cell, _) = grid.iter().next().unwrap();
        assert_eq!(found_cell, cell);
    }

    #[test]
    fn test_iter_empty_grid_yields_nothing() {
        let grid = make_grid(4, 4);
        assert_eq!(grid.iter().count(), 0);
    }

    #[test]
    fn test_overwrite_cell() {
        let mut grid = make_grid(5, 5);
        let cell = Cell::new(1, 1);
        grid.set(&cell, entity(10));
        grid.set(&cell, entity(20));
        assert_eq!(grid.get(&cell), Some(entity(20)));
    }
}
