use std::hash::Hash;

use bevy::ecs::{
    entity::Entity,
    resource::Resource,
    system::{Commands, Res},
};

use crate::{cell::Cell, map::MapSpec};

#[derive(Resource, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TileGrid {
    entities: Vec<Option<Entity>>,
    width: u32,
    height: u32,
}

impl TileGrid {
    pub fn new(size: Cell) -> Self {
        Self {
            entities: Vec::with_capacity(size.x as usize * size.y as usize),
            width: size.x as u32,
            height: size.y as u32,
        }
    }

    #[inline]
    pub fn idx_for(&self, cell: Cell) -> usize {
        cell.to_idx(self.width)
    }

    pub fn add(&mut self, cell: Cell, value: Entity) {
        let idx = self.idx_for(cell);
        self.entities[idx] = Some(value);
    }

    #[inline]
    pub fn get(&self, cell: &Cell) -> Option<Entity> {
        self.get_idx(self.idx_for(*cell))
    }

    #[inline]
    pub fn get_idx(&self, idx: usize) -> Option<Entity> {
        self.entities[idx]
    }

    pub fn set(&mut self, cell: Cell, entity: Entity) {
        let idx = self.idx_for(cell);
        if let Some(slot) = self.entities.get_mut(idx) {
            *slot = Some(entity)
        }
    }

    pub fn remove(&mut self, cell: Cell) {
        let idx = self.idx_for(cell);
        if let Some(slot) = self.entities.get_mut(idx) {
            *slot = None
        }
    }

    pub fn clear(&mut self) {
        self.entities.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (Cell, Entity)> + '_ {
        self.entities
            .iter()
            .enumerate()
            .filter_map(|(idx, &entity)| {
                entity.map(|e| {
                    let x = (idx as u32) % self.width;
                    let y = (idx as u32) / self.width;
                    (Cell::new(x as i32, y as i32), e)
                })
            })
    }
}

pub fn setup_tilegrid(mut commands: Commands, spec: Res<MapSpec>) {
    let tilegrid = TileGrid::new(spec.size.into());
    commands.insert_resource(tilegrid);
}
