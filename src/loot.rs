//! Defines types to award fixed loot or random loot based on a table.
use crate::{
    inventory::Inventory,
    items::{self, Quantity},
};
use bevy::prelude::*;
use rand::{RngExt, seq::IndexedRandom};

/// FixedLoot is used for deterministic loot drops.
#[derive(Component, Debug, Clone)]
pub struct FixedLoot(pub Inventory);

/// A LootTable represents a collection of potential "drops." Each drop is a
/// RandomQty of some item, where each usize is minimum and maximum.
#[derive(Component, Default, Clone)]
pub struct LootTable {
    entries: Vec<TableEntry>,
}

#[derive(Clone, Debug)]
pub struct TableEntry {
    item: items::ItemId,
    min_dropped: usize,
    max_dropped: usize,
}

impl LootTable {
    /// Rolls a random loot drop from the table and returns an Inventory with
    /// the result.
    pub fn roll(&self) -> Inventory {
        match self.entries.choose(&mut rand::rng()) {
            Some(TableEntry {
                item,
                min_dropped,
                max_dropped,
            }) => {
                let qty = rand::rng().random_range(*min_dropped..=*max_dropped);
                Inventory::with_item(*item, Quantity(qty))
            }
            None => Inventory::empty(),
        }
    }
}
