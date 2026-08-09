# EQUIPMENT REDUX (DRAFT STATUS)

## SUMMARY

Equip/unequip is a simple relationship. Slots make it more complex because there's only one item per slot. If the player equips Armor, the previous armor should be un-equipped.

## OBJECTIVE

Make it simpler to add/remove items by slot and discern which slots are occupied.

## BACKGROUND

Item entities have `(ItemId, Quantity)`. When added to the player inventory, they receive `CarriedBy(pub Entity)`, and the relationship target for this is `Carrying(Vec<Entity>)`. 

We also have `EquippedBy(pub Entity)` that goes on an entity with an `(ItemId, Quantity)`. The relationship target receives `HasEquipped(Vec<Entity>)`. 

Only `ItemId` which can be equipped have `EquipDef` with a `&[&Slot]`.

## DESIGN


- want to be able to `populate()` from some data (at will)
- we can "just" use item entities: `(Entity, item_id, is_equipped)`. 
  - Entity is for `EquipmentRow(item_nt)`
  - `item_id` is for display




### STATUS QUO

Finding out which slots are occupied takes some work:
- Get `(&ItemId, &Quantity)` and `iter_many` using `&HasEquipped`.
- `item_id.equip()` returns an `Option<EquipDef>`.
- `EquipDef` defines `&[&Slot]`. We use `first_slot()`, so `Option<Slot>`. 

Finding out which slots are *not* occupied is not possible.

### NEXT DESIGN: RELATIONSHIPS GALORE

- Items: EquippedBy(Creature) and EquippedIn(Slot)
- Creatures: HasEquipped(Items), HasSlots(Slots)
- Slots: OwnedBy(Creature), EquippedWith(Item)

This controls only whether an item is equipped in a specific slot. 

```rust
// the item is equipped here
struct EquippedIn(pub Entity);
// the slot so occupied by the item
struct EquippedWith(Entity);
```

```rust
// slot points to its owner
struct OwnedBy(pub Entity);
// collection of slots so owned
struct HasSlots(Vec<Entity>);
```

```rust
// item equipped points to owner
struct EquippedBy(pub Entity);
// collection of items so equipped
struct HasEquipped(Vec<Entity>);
```

#### LIFECYCLE

Initialization of slots:

```rust
for slot in starting_slots {
    commands.spawn((
        OwnedBy(player_nt),
        Slot { slot },
    ));
}
```

Spawn new item:

```rust
// Spawn new item
let eq_nt = commands.spawn((
    item_id,
    Quantity(1),
)).id();
```

Add item to slot:

```rust
commands.entity(item_nt).insert((
    EquippedBy(player),
    EquippedIn(slot_nt)
));
```

Remove an item:

```rust
commands.entity(item_nt).remove::<(EquippedBy, EquippedIn)>();
```

Toggle an item???

- If we find it among equipped items: remove `EquippedIn` and `EquippedBy`.
- If we do not find it among equipped items: add both.


If we could have `EquippedIn` serve double duty as two relationships:

```rust
struct EquippedBy {
    #[relationship_uno]
    pub owner: Entity, // HasEquipped
    #[relationship_dos]
    pub equipped_in: Entity, // EquippedWith
}
```

Alternatively, maybe a layer of indirection.

```rust
struct EquipState {
    pub equipper: EquippedBy,
    pub slot: EquippedIn,
}
```

# SCRATCH

#### FIRST PROPOSAL

Two relationships, or a double relationship? Assume for now that the player is the only one with equippable slots.



#### initialization

When a player is initialized, we create `EquipSlot` three times: `Armor`, `MainHand`, and `Trinket`. As they are empty, each slot will have `None` for their `Option<Entity>`. 

At this point, `HasEquipSlots` will tell us that we have three slots, and they are each empty.

#### equip starting items

We have `Rags` (`Armor`) and `Stick` (`MainHand`). `ToggleEquip` accepts the player and the item, so we spawn an entity with `ItemId` first, then send the `ToggleEquip`. 

`ToggleEquip` iterates through each such message: `target`, `slot`, and `equipment`. We query `HasEquipSlots` for `target`, find `equip_slot.ty`, and now the thought-work begins.

Toggle means we will at least need to un-equip.






The `EquipSlot` component (a relationship) lives on its own entity. The relationship target is the player, who will have `HasEquipSlots`. 

I want one query to be able to look up either the owner of the slot or the occupants of the slot. So we would grab the `SlotsOnEntity` relationship target. This points to all the slots belonging to the entity, so this is how we would tell which slots are available.

`Slot` component has a pointer to the item which occupies it. 

Operations:

- Enumerate equipped item IDs: `Query<&HasEquipped, With<Player>>`
- Enumerate equip slot IDs: `Query<&EquippableSlots, With<Player>>`, x-ref with `Query<&EquipSlot>`

This would be the usual where we'd cross-reference

We want to be able to look at which slots the entity has available so we can simply replace


--

```rust
struct EquipSlot {
    #[relationship]
    pub owner: Entity,
    pub slot: Slot,
}

// struct HasSlot {
// }

// // On an item entity
// struct EquippedBy {
//     #[relationship]
//     pub owner: Entity,
//     pub slot: Entity,
// }

// struct HasEquipped {
// }
```
