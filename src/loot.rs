use crate::{inventory, tiles::TileIdx};
use bevy::prelude::*;
use rand::{RngExt, seq::IndexedRandom};
use std::collections::HashMap;

/// FixedLoot is used for deterministic loot drops.
#[derive(Component, Debug)]
pub struct FixedLoot(pub inventory::Inventory);

/// A LootTable represents a collection of potential "drops." Each drop
/// is a RandomQty of some item.
#[derive(Default, Clone)]
pub struct LootTable {
    entries: Vec<(inventory::Item, usize, usize)>,
}

impl LootTable {
    /// Rolls a random loot drop from the table and returns an Inventory with the result.
    pub fn roll(&self) -> inventory::Inventory {
        match self.entries.choose(&mut rand::rng()) {
            Some((item, min, max)) => {
                let qty = rand::rng().random_range(*min..=*max);
                inventory::Inventory::with_item(item.clone(), qty)
            }
            None => inventory::empty(),
        }
    }
}

/// [`MobLootTable`] maps [`TileIdx`] to [`LootTable`] for random loot (mob) drops.
#[derive(Resource, Default, Deref, Clone)]
pub struct MobLootTable(HashMap<TileIdx, LootTable>);

impl MobLootTable {
    /// Rolls a random loot drop for the given TileIdx and returns an Inventory with the result.
    /// Typically the quantity is also random, but not always.
    pub fn roll(&self, tile: TileIdx) -> inventory::Inventory {
        match self.0.get(&tile) {
            Some(loot_entry) => loot_entry.roll(),
            None => inventory::empty(),
        }
    }
}
