use crate::{inventory, tiles::TileIdx};
use bevy::prelude::*;
use rand::{RngExt, seq::IndexedRandom};
use std::collections::HashMap;

#[derive(Component, Debug)]
pub struct FixedLoot(pub inventory::Inventory);

/// A LootTable represents a collection of potential "drops." Each drop
/// is a RandomQty of some item.
#[derive(Default, Clone)]
pub struct LootTable {
    entries: Vec<LootEntry>,
}

impl LootTable {
    pub fn roll(&self) -> inventory::Inventory {
        match self.entries.choose(&mut rand::rng()) {
            Some(entry) => entry.roll(),
            None => inventory::empty(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct LootEntry {
    pub item: inventory::Item,
    pub min: usize,
    pub max: usize,
}

impl LootEntry {
    pub fn roll(&self) -> inventory::Inventory {
        let qty = rand::rng().random_range(self.min..=self.max);
        inventory::Inventory::with_items(&[(self.item.clone(), qty)])
    }
}

#[derive(Resource, Default, Deref, Clone)]
pub struct RandLootTable(HashMap<TileIdx, LootTable>);

impl RandLootTable {
    pub fn roll(&self, tile: TileIdx) -> inventory::Inventory {
        match self.0.get(&tile) {
            Some(loot_entry) => loot_entry.roll(),
            None => inventory::empty(),
        }
    }
}
