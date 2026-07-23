use bevy::{
    ecs::message::{Message, MessageReader},
    log::info,
    prelude::*,
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::{
    actors::Player,
    items::{ItemId, Quantity},
};

/// ItemEntry is a representation of an Item and its Quantity.
/// Modifying this has no impact on item-related components or relationships;
/// this is a type that makes many type signatures substantially simpler.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Reflect, Eq)]
pub struct ItemEntry(pub ItemId, pub Quantity);

impl From<(ItemId, Quantity)> for ItemEntry {
    fn from(value: (ItemId, Quantity)) -> Self {
        ItemEntry(value.0, value.1)
    }
}

impl From<(&ItemId, &Quantity)> for ItemEntry {
    fn from(value: (&ItemId, &Quantity)) -> Self {
        Self::from((*value.0, *value.1))
    }
}

/// Inventory is a colleciton of items constituting a view, typically of the
/// player's current inventory. It's an abstraction over Carrying/CarriedBy,
/// used as a resource primarily for the Player's inventory. Write `Acquisition`
/// messages to modify what a player is carrying. Modifying an Inventory has
/// *no* effect on Relationships like Carrying or CarriedBy.
#[derive(Resource, Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq, Reflect)]
pub struct Inventory(Vec<ItemEntry>);

impl From<Vec<ItemEntry>> for Inventory {
    fn from(value: Vec<ItemEntry>) -> Self {
        Self(value)
    }
}

impl From<&[(ItemId, Quantity)]> for Inventory {
    /// Creates a new [Inventory] from a slice of [Item]s and their quantities.
    fn from(items: &[(ItemId, Quantity)]) -> Self {
        items.iter().cloned().collect()
    }
}

impl From<Vec<(&ItemId, &Quantity)>> for Inventory {
    fn from(value: Vec<(&ItemId, &Quantity)>) -> Self {
        Inventory(value.iter().map(|(it, q)| (*it, *q).into()).collect())
    }
}

impl Extend<(ItemId, Quantity)> for Inventory {
    fn extend<I: IntoIterator<Item = (ItemId, Quantity)>>(&mut self, iter: I) {
        self.0
            .extend(iter.into_iter().map(|it| ItemEntry::from(it)));
    }
}

impl Extend<ItemEntry> for Inventory {
    fn extend<I: IntoIterator<Item = ItemEntry>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl FromIterator<(ItemId, Quantity)> for Inventory {
    fn from_iter<I: IntoIterator<Item = (ItemId, Quantity)>>(iter: I) -> Self {
        let mut inv = Inventory::default();
        inv.extend(iter);
        inv
    }
}

impl IntoIterator for Inventory {
    type Item = ItemEntry;

    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Inventory {
    pub fn with_item(itam: ItemId, q: Quantity) -> Self {
        Inventory(vec![(itam, q).into()])
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn include_item(&mut self, itam: ItemId, q: Quantity) -> &mut Self {
        self.0.push((itam, q).into());
        self
    }

    pub fn has_item(&self, want_itam: &ItemId) -> bool {
        self.0.iter().any(|ItemEntry(it, _)| it == want_itam)
    }

    /// Returns a summary of [Inventory] [Item]s as a vector of strings. Each
    /// item will have `prefix` prepended to it.
    pub fn summarized(&self, prefix: &str) -> Vec<String> {
        self.0
            .iter()
            .map(|ItemEntry(k, v)| format!("{} {} {}", prefix, v, k.def()))
            .collect::<Vec<_>>()
    }

    pub fn empty() -> Self {
        Self(vec![])
    }

    pub fn from_str(item_spec: impl AsRef<str>) -> Option<Inventory> {
        let spec: String = item_spec.as_ref().into();
        if spec.is_empty() {
            return None;
        }

        let (item, qty) = ItemId::from_spec(item_spec);
        Some(Inventory::with_item(item, qty))
    }

    pub fn from_str_array<S, I>(item_specs: I) -> Option<Inventory>
    where
        S: AsRef<str> + Clone + std::fmt::Debug,
        I: IntoIterator<Item = S> + std::fmt::Debug,
    {
        Some(
            item_specs
                .into_iter()
                .map(|it| ItemId::from_spec(it.as_ref()))
                .collect(),
        )
    }
}

#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = Carrying)]
pub struct CarriedBy(pub Entity);

#[derive(Component, Reflect, Debug, Serialize, Deserialize)]
#[relationship_target(relationship = CarriedBy)]
pub struct Carrying(Vec<Entity>);

impl IntoIterator for Carrying {
    type Item = Entity;

    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<&[ItemId]> for Inventory {
    /// Creates a new [Inventory] from a slice of [Item]s, counting each item's occurrences.
    fn from(items: &[ItemId]) -> Self {
        items.iter().cloned().map(|it| (it, Quantity(1))).collect()
    }
}

// impl<'a> IntoIterator for &'a Inventory {
//     type Item = (&'a ItemId, &'a Quantity);

//     type IntoIter = hash_map::Iter<'a, ItemId, Quantity>;

//     fn into_iter(self) -> Self::IntoIter {
//         self.0.iter()
//     }
// }

// /// Returns the default [Inventory] with no items.
// pub fn empty() -> Inventory {
//     Inventory::default()
// }

// impl Inventory {
//     pub fn has_item(&self, item: &ItemId) -> bool {
//         self.0.contains_key(item)
//     }

//     /// Returns a summary of [Inventory] [Item]s as a vector of strings. Each
//     /// item will have `prefix` prepended to it.
//     pub fn summary(&self, prefix: &str) -> Vec<String> {
//         self.0
//             .iter()
//             .map(|(k, v)| format!("{} {} {}", prefix, v, k.def()))
//             .collect::<Vec<_>>()
//     }

//     pub fn is_empty(&self) -> bool {
//         self.0.is_empty()
//     }

/// A message representing the acquisition of [Inventory] items by an actor,
/// such as the player picking up items from a chest or loot.
#[derive(Message, Debug, Clone, Reflect, Serialize, Deserialize, PartialEq, Eq)]
pub struct Acquisition {
    pub items: Vec<ItemEntry>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Change {
    Acquired,
    Removed,
}

#[derive(Message, Debug, Clone, Copy, Eq, PartialEq)]
pub struct InventoryChange {
    /// Change the items belonging to this entity.
    entity: Entity,
    /// Whether the change adds or removes inventory.
    typ: Change,
    /// The item whose quantity will change.
    item_id: ItemId,
    /// The quantity to add or remove from inventory.
    delta: Quantity,
}

impl InventoryChange {
    #[inline]
    pub(crate) fn q(&self) -> usize {
        self.delta.0
    }
}

/// Merges [`Inventory`] items into the player's inventory.
pub fn process_acquisitions(
    mut commands: Commands,
    mut acquisitions: MessageReader<Acquisition>,
    player: Single<Entity, With<Player>>,
) {
    for acquisition in acquisitions.read() {
        info!("Player acquires items: {:?}", acquisition.items);
        for ItemEntry(itam, q) in acquisition.items.iter() {
            commands.spawn((*itam, *q, CarriedBy(*player)));
        }
    }
}

pub fn process_inventory_changes(
    mut commands: Commands,
    mut changes: MessageReader<InventoryChange>,
    all_carrying: Query<&Carrying>,
    all_carried: Query<(Entity, &ItemId, &Quantity), With<CarriedBy>>,
) {
    for inv_change in changes.read() {
        let Ok(target_carrying) = all_carrying.get(inv_change.entity) else {
            error!("process_inventory_changes: unknown entity in change {inv_change:?}");
            continue;
        };

        let carried_items = all_carried.iter_many(target_carrying.iter());
        match inv_change.typ {
            Change::Acquired => acquire(&mut commands, inv_change, carried_items),
            Change::Removed => remove(&mut commands, inv_change, carried_items),
        }
    }
}

fn remove<'a>(
    commands: &mut Commands,
    change: &InventoryChange,
    carried_items: impl IntoIterator<Item = (Entity, &'a ItemId, &'a Quantity)>,
) {
    let Some((item_nt, item_id, Quantity(curr_q))) = carried_items
        .into_iter()
        .find(|(_, it, _)| change.item_id == **it)
    else {
        error!("remove: could not find item: {change:?}");
        return;
    };

    if *curr_q < change.q() {
        error!(
            "remove: insufficient quantity of item: have {item_id:?} {curr_q}, needed at least {change:?}"
        );
        return;
    }

    let new_q = curr_q.saturating_sub(change.q());

    if new_q == 0 {
        info!("remove: despawned {item_id:?}");
        commands.entity(item_nt).despawn();
    } else {
        info!("remove: deducted {} from {item_id:?} {curr_q}", change.q());
        commands.entity(item_nt).insert(Quantity(new_q));
    }
}

fn acquire<'a>(
    commands: &mut Commands,
    change: &InventoryChange,
    carried_items: impl IntoIterator<Item = (Entity, &'a ItemId, &'a Quantity)>,
) {
    let (item_nt, item_id, curr_q) = carried_items
        .into_iter()
        .find(|(_, it, _)| change.item_id == **it)
        .unwrap_or_else(|| (commands.spawn_empty().id(), &change.item_id, &change.delta));

    commands
        .entity(item_nt)
        .insert_if_new((*item_id, *curr_q))
        .insert(CarriedBy(change.entity));
}

fn snapshot_inventory(
    mut inventory_cache: ResMut<Inventory>,
    player_carrying: Single<&Carrying, With<Player>>,
    all_items: Query<(&ItemId, &Quantity), With<Carrying>>,
) {
    inventory_cache.set_if_neq(
        all_items
            .iter_many(player_carrying.iter())
            .map(|(it, q)| ItemEntry(*it, *q))
            .collect::<Vec<ItemEntry>>()
            .into(),
    );
}
