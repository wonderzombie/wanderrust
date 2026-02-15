use bevy::{ecs::resource::Resource, platform::collections::HashMap};



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
