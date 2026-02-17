use bevy::ecs::query::QueryData;

use crate::{cell, tiles};

#[derive(QueryData)]
pub struct MapCellData {
    pub cell: &'static cell::Cell,
    pub tile_idx: &'static tiles::TileIdx,
}

impl<'w, 's> MapCellDataItem<'w, 's> {
    pub fn xy(&self) -> (i32, i32) {
        (self.cell.x, self.cell.y)
    }

    pub fn tile(&self) -> &tiles::TileIdx {
        self.tile_idx
    }
}
