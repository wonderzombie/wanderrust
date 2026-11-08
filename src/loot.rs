//! Defines types to award [`FixedLoot`] or random loot based on a [`LootTable`].
use crate::{
    inventory::Inventory,
    items::{self, Quantity},
};
use bevy::prelude::*;
use rand::{RngExt, seq::IndexedRandom};

/// [`FixedLoot`] is used for deterministic loot drops as [`Inventory`].
#[derive(Component, Debug, Clone)]
pub struct FixedLoot(pub Inventory);

/// A [`LootTable`] represents a collection of potential "drops" as [`TableEntry`].
#[derive(Component, Default, Clone)]
pub struct LootTable {
    entries: Vec<TableEntry>,
}

/// A [`LootTable`] entry with an item. Minimum and maximum determine the upper
/// and lower bounds for the quantity of the item dropped.
#[derive(Clone, Debug)]
pub struct TableEntry {
    /// The item to be dropped.
    item: items::ItemId,
    /// The lowest quantity of the item dropped.
    min_dropped: usize,
    /// The highest quantity of the item dropped.
    max_dropped: usize,
}

impl LootTable {
    /// Chooses a [`TableEntry`] at random, rolls how many according to
    /// minimum/maximum, and returns an [`Inventory`] with the result.
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
