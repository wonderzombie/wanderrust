use std::{fmt::Display, ops::AddAssign};

use bevy::{
    ecs::{
        message::{Message, MessageReader},
        resource::Resource,
        system::ResMut,
    },
    log::info,
    platform::collections::HashMap,
};

/// A simple wrapper around a string to represent an item in the game world.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct Item(pub String);

impl Item {
    pub fn from(name: impl AsRef<str>) -> Self {
        Item(name.as_ref().to_string())
    }
}

impl Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A resource representing the player's inventory, which is a mapping of items to their quantities.
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub struct Inventory(HashMap<Item, usize>);

impl From<HashMap<Item, usize>> for Inventory {
    /// Creates a new [Inventory] from a [HashMap] of [Item]s and their quantities.
    fn from(items: HashMap<Item, usize>) -> Self {
        Inventory(items)
    }
}

impl From<&[Item]> for Inventory {
    /// Creates a new [Inventory] from a slice of [Item]s, counting each item's occurrences.
    fn from(items: &[Item]) -> Self {
        let mut inventory = HashMap::new();
        for item in items.iter() {
            *inventory.entry(item.clone()).or_insert(0usize) += 1usize;
        }
        Inventory(inventory)
    }
}

impl From<&[(Item, usize)]> for Inventory {
    /// Creates a new [Inventory] from a slice of [Item]s and their quantities.
    fn from(items: &[(Item, usize)]) -> Self {
        let mut inventory = HashMap::new();
        for (item, count) in items.iter() {
            *inventory.entry(item.clone()).or_insert(0usize) += *count;
        }
        Inventory(inventory)
    }
}

impl AddAssign<Inventory> for Inventory {
    fn add_assign(&mut self, rhs: Inventory) {
        for (item, count) in rhs.0 {
            self.add_item(item, count);
        }
    }
}

impl AddAssign<(Item, usize)> for Inventory {
    fn add_assign(&mut self, rhs: (Item, usize)) {
        self.add_item(rhs.0, rhs.1);
    }
}

/// Returns the default [Inventory] with no items.
pub fn empty() -> Inventory {
    Inventory::default()
}

impl Inventory {
    /// Adds an [Item] to this [Inventory], incrementing its count if it already exists.
    pub fn add_item(&mut self, item: Item, count: usize) -> &mut Self {
        *self.0.entry(item).or_insert(0) += count;
        self
    }

    /// Merges another [Inventory] into this one, adding each [Item]'s count.
    pub fn merge(&mut self, rhs: Inventory) {
        for (item, count) in rhs.0 {
            self.add_item(item, count);
        }
    }

    /// Creates a new [Inventory] with a single [Item] and count.
    pub fn with_item(item: Item, count: usize) -> Self {
        let mut inventory = HashMap::new();
        inventory.insert(item, count);
        Inventory(inventory)
    }

    pub fn has_item(&self, item: &Item) -> bool {
        self.0.contains_key(item)
    }

    /// Returns a summary of [Inventory] [Item]s as a vector of strings.
    /// Each item will have `prefix` prepended to it.
    pub fn summary(&self, prefix: &str) -> Vec<String> {
        self.0
            .iter()
            .map(|(k, v)| format!("{} {} {}", prefix, v, k))
            .collect::<Vec<_>>()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A message representing the acquisition of [Inventory] items by an actor, such as the player picking up items from a chest or loot.
#[derive(Message, Debug)]
pub struct Acquisition {
    pub items: Inventory,
}

/// Merges [Inventory] items into the player's inventory.
pub fn process_acquisitions(
    mut acquisitions: MessageReader<Acquisition>,
    mut player_inventory: ResMut<Inventory>,
) {
    for acquisition in acquisitions.read() {
        info!("Player acquires items: {:?}", acquisition.items);
        player_inventory.merge(acquisition.items.clone());
    }
}
