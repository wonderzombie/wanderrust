# ITEMS: GOING FROM HASHMAP TO ENTITIES

## SUMMARY

Replace `Inventory` (where `struct Inventory(pub HashMap<Item, usize>)`, `struct Item(pub String)`) such that `inventory.rs` becomes the public API. 

Items become entities with components:
- CarriedBy
- ItemId
- Quantity
- Modifiers?

`Inventory` becomes `Acquisition`, the Bevy `Message` that `inventory.rs` should use to add items to the player's inventory. 

Ownership of an item is represented by a Relationship `Carrying` and `CarriedBy`.

## WORK ITEMS

- Pipeline from LDtk to `inventory.rs` needs to adjust
  - esp `interactable::Chest` type(s) need to change
- `Inventory` helper methods should live on `inventory.rs` where applicable
  - `from_str_array()` needs to turn `["gold:2", "strange_key"]` into such as `vec![(ItemId::Gold, Quantity(2)), (ItemId::StrangeKey, Quantity(1))]`.

Some API methods may not translate *quite* as cleanly.

- `has_item()`
- `is_empty()`
- `summary()`
- `with_item()` (mobs.rs)
- `extend()` (mobs.rs)

Notes:
- Where Acquisitions is used as a thin wrapper around `Inventory`, we can use `Acquisitions`
- **Change detection:** whether it would be worth maintaining `Inventory` as a snapshot type, a la `snapshot_cells` and `snapshot_denizens`
- **Removing items:** 
- consider a generic system for "item used" and dispatch based on Kind s/t `Consumable` includes removal.

## DESIGN

### ADDING/REMOVING ITEMS

We shouldn't change `Acquisition` to allow "acquired negative number of items." 

Propose:
1. `inventory::ItemsChange` as a `Message`
1. `enum Change { Acquired, Removed }` as field on `ItemsChange`
1. `ItemsChange` contains *one* change to *one* ItemId at a time
2. `Quantity` remains `usize`; `Removed` is subtraction between two positive `usize`
3. `ItemsChange` uses `ItemId`; `inventory.rs` manages `Carrying`

Ex:

```rust
pub enum Change {
    Acquired,
    Removed,
}

#[derive(Message)]
pub struct ItemsChange {
    pub change: Change,
    pub item_id: ItemId,
    pub delta: Quantity,
}

// This example shows me that, perhaps, `snapshot_items` with a `Local<Carrying>` could
// be used to drive change detection.
fn handle_items_change(
    player_items: Single<&Carrying, With<Player>>,
    all_items: Query<(&Entity, &ItemId, &Quantity), With<CarriedBy>>,
    mut item_messages: MessageReader<ItemsChange>,
) {
    let player_inventory = all_items.iter_many(*player_inventory.iter());

    for item_change in item_messages.read() {
        match item_change.change {
            Change::Acquired => acquire(commands, item_change, player_inventory),
            Change::Removed => remove(commands, item_change, player_inventory),
        }    
    }
}
```


**Who is `Inventory` for?**

I think it is a little-f facade. The acquire/remove system maintains the entity relationships, *especially* the write path. Any read path can use `Inventory` for convenience.

The consumers are those systems that benefit from having just one inventory-related input: 

- Logic for checking for keys
- Logic for quests
- Inventory display

Any of these can sign up for "write" by using `ItemChanges` which will use `ItemId`, maintaining `ItemId` and not `Carrying`, et al, as the "public" API for `inventory.rs`.

**Change detection**

It can be fiddly logic to detect certain changes the inventory. Remember CRUD. 

Create and Update are covered by `Changed<T>` and `Added<T>`, although these are NOT archetypes (meaning they aren't columns that are/aren't present, unlike `With<T>`).

Delete requires `RemovedComponent<T>` and it is not a `QueryFilter`. It has `Message`-like semantics. From the docs:

```rust
fn react_on_removal(mut removed: RemovedComponents<MyComponent>) {
    removed.read().for_each(|removed_entity| println!("{}", removed_entity));
}
```

Instead of managing persistent state, we can simplify by updating the "read only" list once per frame. While we could centralize 

```rust
// run once per frame
pub fn snapshot_inventory(
    items: Single<&Carrying, With<Player>>,
    snapshot: ResMut<Inventory>,
    all_items: Query<(&Entity, &ItemId, &Quantity), With<CarriedBy>>,
) {
    let player_inventory = all_items.iter_many(*curr_items.iter());
    let new_inventory = Inventory::new();
    for (nt, item, n) in player_inventory {
        // populate `Inventory` anew
    }
    *snapshot = new_inventory;
}
```
