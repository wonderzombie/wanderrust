use std::fmt::Display;

use bevy::{
    ecs::{
        entity::Entity,
        message::{Message, MessageReader},
        query::With,
        resource::Resource,
        system::{Query, ResMut},
    },
    log::{info, warn},
    platform::collections::HashMap,
};

use crate::Player;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
/// A simple wrapper around a string to represent an item in the game world.
pub struct Item(pub String);

impl Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
/// A resource representing the player's inventory, which is a mapping of items to their quantities.
pub struct Inventory(HashMap<Item, usize>);

impl From<HashMap<Item, usize>> for Inventory {
    fn from(items: HashMap<Item, usize>) -> Self {
        Inventory(items)
    }
}

impl From<&[Item]> for Inventory {
    fn from(items: &[Item]) -> Self {
        let mut inventory = HashMap::new();
        for item in items.iter() {
            *inventory.entry(item.clone()).or_insert(0usize) += 1usize;
        }
        Inventory(inventory)
    }
}

impl Inventory {
    pub fn add_item(&mut self, item: Item, count: usize) {
        *self.0.entry(item).or_insert(0) += count;
    }

    pub fn merge(&mut self, rhs: Inventory) {
        for (item, count) in rhs.0 {
            self.add_item(item, count);
        }
    }

    pub fn with_item(item: Item, count: usize) -> Self {
        let mut inventory = HashMap::new();
        inventory.insert(item, count);
        Inventory(inventory)
    }

    pub fn with_items(items: &[(Item, usize)]) -> Self {
        let mut inventory = HashMap::new();
        for (item, count) in items {
            *inventory.entry(item.clone()).or_insert(0) += *count;
        }
        Inventory(inventory)
    }

    pub fn has_item(&self, item: &Item) -> bool {
        self.0.contains_key(item)
    }

    pub fn summary(&self, prefix: &str) -> Vec<String> {
        self.0
            .iter()
            .map(|(k, v)| format!("{} {} {}", prefix, v, k))
            .collect::<Vec<_>>()
    }
}

#[derive(Message, Debug)]
/// A message representing the acquisition of items by an actor, such as the player picking up items from a chest or loot.
pub struct Acquisition {
    pub acquirer: Entity,
    pub items: Inventory,
}

pub fn process_acquisitions(
    mut acquisitions: MessageReader<Acquisition>,
    player_query: Query<Entity, With<Player>>,
    mut player_inventory: ResMut<Inventory>,
) {
    let Ok(player_entity) = player_query.single() else {
        warn!("No player entity found in the world.");
        return;
    };

    for acquisition in acquisitions.read() {
        if acquisition.acquirer == player_entity {
            info!("Player acquires items: {:?}", acquisition.items);
            player_inventory.merge(acquisition.items.clone());
        }
    }
}
