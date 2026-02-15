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
pub struct Item(pub String);

#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub struct Inventory(HashMap<Item, usize>);

impl From<HashMap<Item, usize>> for Inventory {
    fn from(items: HashMap<Item, usize>) -> Self {
        Inventory(items)
    }
}

impl From<Vec<Item>> for Inventory {
    fn from(items: Vec<Item>) -> Self {
        let mut inventory = HashMap::new();
        for item in items {
            *inventory.entry(item).or_insert(0) += 1;
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
}

#[derive(Message, Debug)]
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
