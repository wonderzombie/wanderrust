use std::fmt::Display;

use bevy::{
    ecs::{
        message::{Message, MessageReader},
        resource::Resource,
        system::ResMut,
    },
    log::{info, warn},
    platform::collections::{HashMap, hash_map},
    prelude::*,
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

/// A simple wrapper around a string to represent an item in the game world.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, Reflect, Serialize, Deserialize)]
pub struct Item(pub String);

impl Item {
    pub fn from(name: impl AsRef<str>) -> Self {
        Item(name.as_ref().to_string())
    }

    pub fn from_spec(item_spec: impl AsRef<str>) -> (Self, usize) {
        if let Some((it, n)) = item_spec.as_ref().split_once(':') {
            let item = Item::from(it);
            let qty = n.parse().unwrap_or(1);
            (item, qty)
        } else {
            (Item::from(item_spec.as_ref()), 1)
        }
    }
}

impl Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A resource representing the player's inventory, which is a mapping of items to their quantities.
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq, Reflect, Serialize, Deserialize)]
#[reflect(Resource)]
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
        items.iter().cloned().map(|it| (it, 1)).collect()
    }
}

impl From<&[(Item, usize)]> for Inventory {
    /// Creates a new [Inventory] from a slice of [Item]s and their quantities.
    fn from(items: &[(Item, usize)]) -> Self {
        items.iter().cloned().collect()
    }
}

impl FromIterator<(Item, usize)> for Inventory {
    fn from_iter<I: IntoIterator<Item = (Item, usize)>>(iter: I) -> Self {
        let mut inv = Inventory::default();
        inv.extend(iter);
        inv
    }
}

impl IntoIterator for Inventory {
    type Item = (Item, usize);

    type IntoIter = hash_map::IntoIter<Item, usize>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Inventory {
    type Item = (&'a Item, &'a usize);

    type IntoIter = hash_map::Iter<'a, Item, usize>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Extend<(Item, usize)> for Inventory {
    fn extend<I: IntoIterator<Item = (Item, usize)>>(&mut self, iter: I) {
        for (it, n) in iter {
            self.add_item(it, n);
        }
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

    pub fn from_str(item_spec: impl AsRef<str>) -> Option<Inventory> {
        let spec: String = item_spec.as_ref().into();
        if spec.is_empty() {
            return None;
        }

        let (item, qty) = Item::from_spec(item_spec);
        Some(Inventory::with_item(item, qty))
    }

    pub fn from_str_array<T, I>(item_specs: I) -> Option<Inventory>
    where
        I: IntoIterator<Item = T> + std::fmt::Debug,
        T: AsRef<str> + Clone + std::fmt::Debug,
    {
        let mut inv = Inventory::default();
        for s in item_specs.into_iter() {
            if s.as_ref().is_empty() {
                warn!("skipping empty item spec: {:?}", &s);
                continue;
            }
            let (item, qty) = Item::from_spec(s.as_ref());
            inv.add_item(item, qty);
        }
        Some(inv)
    }
}

/// A message representing the acquisition of [Inventory] items by an actor, such as the player picking up items from a chest or loot.
#[derive(Message, Debug, Reflect)]
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
        player_inventory.extend(acquisition.items.clone());
    }
}
