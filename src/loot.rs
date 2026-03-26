use crate::inventory;
use bevy::prelude::*;
use rand::{RngExt, seq::IndexedRandom};

/// FixedLoot is used for deterministic loot drops.
#[derive(Component, Debug)]
pub struct FixedLoot(pub inventory::Inventory);

/// A LootTable represents a collection of potential "drops." Each drop
/// is a RandomQty of some item.
#[derive(Component, Default, Clone)]
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
