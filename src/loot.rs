use crate::{
    inventory,
    items::{self, Quantity},
};
use bevy::prelude::*;
use rand::{RngExt, seq::IndexedRandom};

/// FixedLoot is used for deterministic loot drops.
#[derive(Component, Debug, Clone)]
pub struct FixedLoot(pub inventory::Inventory);

/// A LootTable represents a collection of potential "drops." Each drop is a
/// RandomQty of some item, where each usize is minimum and maximum.
#[derive(Component, Default, Clone)]
pub struct LootTable {
    entries: Vec<(items::ItemId, usize, usize)>,
}

impl LootTable {
    /// Rolls a random loot drop from the table and returns an Inventory with
    /// the result.
    pub fn roll(&self) -> inventory::Inventory {
        match self.entries.choose(&mut rand::rng()) {
            Some((item, min, max)) => {
                let qty = rand::rng().random_range(*min..=*max);
                inventory::Inventory::with_item(*item, Quantity(qty))
            }
            None => inventory::Inventory::empty(),
        }
    }
}
