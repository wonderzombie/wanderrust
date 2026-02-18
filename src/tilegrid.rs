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
    fn new(size: (u32, u32)) -> Self {
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
    fn get(&self, cell: &Cell) -> Option<Entity> {
        let idx = self.to_idx(cell);
        self.tiles.get(idx).copied().flatten()
    }

    #[inline]
    fn get_idx(&self, idx: usize) -> Option<Entity> {
        self.tiles[idx]
    }

    fn set(&mut self, cell: &Cell, entity: Entity) {
        let idx = self.to_idx(cell);
        if let Some(slot) = self.tiles.get_mut(idx) {
            *slot = Some(entity);
        }
    }

    fn remove(&mut self, cell: &Cell) {
        let idx = self.to_idx(cell);
        if let Some(slot) = self.tiles.get_mut(idx) {
            *slot = None;
        }
    }

    fn iter(&self) -> impl Iterator<Item = (Cell, Entity)> + '_ {
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
    for (cell, entity) in tiles {
        let idx = tilegrid.to_idx(cell);
        info!("setting up {:?} {}", cell, idx);
        assert_ne!(tilegrid.tiles.len(), 0);
        tilegrid.set(cell, entity);
    }
}
