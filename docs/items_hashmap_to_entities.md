# ITEMS: HASHMAP TO ENTITIES

## SUMMARY

Replace `Inventory` — currently `struct Inventory(pub HashMap<Item, usize>)` with
`struct Item(pub String)` — with items as entities. Each item entity carries
`ItemId`, `Quantity`, and (eventually) `Modifiers`. Ownership is expressed as the
Bevy Relationship `Carrying` / `CarriedBy`.

`inventory.rs` becomes the public API. Writes go through a `Message` type;
`Inventory` survives only as a per-frame read-only snapshot for convenience.

## OBJECTIVE

A `HashMap<Item, usize>` can't hold state. Items can't carry modifiers, durability,
or uses; they can't be queried, observed, or targeted; and a stringly-typed `Item`
gives us no way to check that "red salve" means anything. Making items entities puts
them in the ECS where the rest of the game already lives.

Done looks like: `inventory.rs` owns the write path, systems that need to read an
inventory get one cheap input, and item entities can grow components without
touching every consumer.

**Not in scope:**

- Equipment slots and the `Equippable` question (see `items_as_entities.md`)
- The item catalog / definitions problem — where descriptions and stats live
- `uses` and consumable depletion, beyond leaving room for it

## BACKGROUND

- `Inventory` is a HashMap of item to quantity, and a singleton on the player.
- The LDtk pipeline feeds items in via `interactable::Chest`.
- `Inventory` is also used as a thin wrapper in a few places where what's really
  meant is "a batch of items just acquired."
- Existing helper surface: `has_item()`, `is_empty()`, `summary()`, `with_item()`
  and `extend()` (both in `mobs.rs`), `from_str_array()`.

## DESIGN

### Item entities

Items become entities with:

- `ItemId`
- `Quantity`
- `Modifiers` (later)
- `CarriedBy` — the Relationship back to the carrier

The carrier gets `Carrying`, populated by Bevy.

`ItemId` — not `Carrying`, not `CarriedBy` — is the public vocabulary of
`inventory.rs`. Consumers name items; `inventory.rs` maintains relationships.

### Adding and removing items

`Acquisition` shouldn't grow the ability to express "acquired a negative number of
items." Instead, one message type with an explicit direction:

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
```

Constraints:

1. One `ItemsChange` describes **one** change to **one** `ItemId`.
2. `Quantity` stays `usize`; `Removed` is subtraction between two positive `usize`.
3. `ItemsChange` speaks `ItemId`; `inventory.rs` manages `Carrying`.

```rust
fn handle_items_change(
    player_items: Single<&Carrying, With<Player>>,
    all_items: Query<(&Entity, &ItemId, &Quantity), With<CarriedBy>>,
    mut item_messages: MessageReader<ItemsChange>,
) {
    let player_inventory = all_items.iter_many(*player_items.iter());

    for item_change in item_messages.read() {
        match item_change.change {
            Change::Acquired => acquire(commands, item_change, player_inventory),
            Change::Removed => remove(commands, item_change, player_inventory),
        }
    }
}
```

### Who is `Inventory` for?

It's a little-f facade. The acquire/remove system maintains the entity
relationships — *especially* the write path. Any read path can use `Inventory` for
convenience.

The consumers are systems that benefit from having exactly one inventory-shaped
input:

- Key checks
- Quest logic
- Inventory display

Any of these can opt into the write path via `ItemsChange`.

### Change detection

Remember CRUD. Create and Update are covered by `Added<T>` and `Changed<T>` — note
these are *not* archetype filters, unlike `With<T>`. Delete needs
`RemovedComponents<T>`, which isn't a `QueryFilter` at all; it has `Message`-like
semantics:

```rust
fn react_on_removal(mut removed: RemovedComponents<MyComponent>) {
    removed.read().for_each(|removed_entity| println!("{}", removed_entity));
}
```

Rather than manage persistent state, rebuild the read-only view once per frame —
the same shape as `snapshot_cells` and `snapshot_denizens`:

```rust
pub fn snapshot_inventory(
    items: Single<&Carrying, With<Player>>,
    mut snapshot: ResMut<Inventory>,
    all_items: Query<(&Entity, &ItemId, &Quantity), With<CarriedBy>>,
) {
    let player_inventory = all_items.iter_many(*items.iter());
    let mut new_inventory = Inventory::new();
    for (nty, item, n) in player_inventory {
        // populate `Inventory` anew
    }
    *snapshot = new_inventory;
}
```

## ALTERNATIVES CONSIDERED

### Keep `Inventory` as the source of truth, sync entities from it

Rejected. Two sources of truth, and the HashMap is the one that can't hold the
state we actually want. Inverting it — entities authoritative, HashMap derived —
gets the same ergonomics with none of the desync.

### `Acquisition` with signed deltas

Rejected. "Acquired -2 gold" is a lie in the type system. `Change::Removed` keeps
`Quantity` unsigned and the intent legible.

## OPEN QUESTIONS

- **Snapshot vs. persistent `Inventory`.** The `handle_items_change` sketch above
  suggests `snapshot_items` with a `Local<Carrying>` could drive change detection
  instead of a `Res<Inventory>` rebuilt per frame. Not sure which is better; the
  snapshot is simpler and I don't yet know if per-frame rebuild costs anything.
- **Removing items — the general case.** Consider a generic "item used" system that
  dispatches on `Kind`, where `Consumable` implies removal. Unclear whether that's
  this doc or a separate one.
- **`from_str_array()`.** Needs to turn `["gold:2", "strange_key"]` into
  `vec![(ItemId::Gold, Quantity(2)), (ItemId::StrangeKey, Quantity(1))]`. Where does
  the string-to-`ItemId` mapping live? This bumps into the catalog problem, which is
  explicitly out of scope, which is suspicious.
- Some API methods won't translate cleanly: `has_item()`, `is_empty()`, `summary()`,
  `with_item()` and `extend()` (both `mobs.rs`).

## WORK ITEMS

- [ ] Adjust the LDtk → `inventory.rs` pipeline; `interactable::Chest` types need to change
- [ ] Move `Inventory` helper methods onto `inventory.rs` where they still apply
- [ ] Rewrite `from_str_array()` against `ItemId`
- [ ] Replace `Acquisition` uses where it was standing in for `Inventory`
- [ ] Implement `ItemsChange`, `acquire`, `remove`
- [ ] Implement `snapshot_inventory`

## OUTCOME

_Not started._
