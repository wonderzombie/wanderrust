# NOTES ON SOME PATTERNS

Informal documentation of some useful patterns used in `wanderrust`.

## querying for mutable component w/o always triggering change detection

```rust
pub fn on_enter_ramifying(mut actors: Query<&mut Turn, With<Actor>>) {
    for mut turn in actors.iter_mut() {
        if turn.as_ref() != &Turn::Idling {
            turn.set_if_neq(Turn::Waiting);
        }
    }
}
```

## doing work under some condition

We like to avoid `Added` or `Changed` sometimes, or only run a system specifically when there is work to do. We have many options. Here are a few.

Solution: use `If<ResMut<Foo>>` as a `SystemParam` so that the system only runs when `Foo` exists. No query needed. Once `Foo` is removed/consumed, the system will not run again, so the logic within the system doesn't have to check elsewise. 

Solution: use `Populated<&Bar>` so the system only runs when there is one or more entities with `&Bar`. For small N, add/remove a marker struct is fine. This can allow a system to run in the same schedule as other systems, yet it will not run unless/until there's work. 

Solution: use `MessageWriter` and `MessageReader` to "post" information for another system to use. If there are multiple such messages, they will be handled in order. 

Solution: use `SystemCondition` like `resource_exists::<Foo>`. A plugin can ensure this condition is added rather than doing it in `main` or `lib`.

## stateful tuple struct

`insert()` or `remove()` changes archetypes which is more expensive than in-place mutation.

Solution: `Revealed(pub bool)`. Tiles that are revealed or not retain the `Revealed` component at all times; it is only the inner value that changes.

Solution: `LightLevel`, an enum which may be changed in place, such as from `LightLevel::Dim` to `LightLevel::Bright`.

## updating a useful, dependent component

For a component like `Cell`, we could see `Changed<Cell>`, but not what the old value was. If there's state that must be restored, we would want to know which `Cell` (entity) needs to be "reset." 

Solution: a newtype `PrevCell` and a system that uses `Changed<Cell>`. `&mut PrevCell` allows `set_if_neq(*curr_cell)`.

## on-boarding certain entities into a new system collection

A new system wants to know when an `Actor` entity has been spawned that may need a component-as-registration.

Solution: a system queries for entities without a marker component, gating it on `Populated`. 

```rust
    query: Populated<(Entity, &ChildOf), (With<Actor>, Without<DenizenOf>)>,
```

```rust
    combatants: Populated<
        (Entity, Option<&Name>, &TileIdx),
        (Added<Combatant>, Without<Parameters>),
```

## grouping entities logically but not visually

`ChildOf` and `Children` are both special components s/t entities with `Children` affect entities with a corresponding `ChildOf`. If the parent is `Visibility::Hidden``, children with `Visibility::Inherited`, for instance, will be hidden. 

Sometimes we do not want such a direct or hierarchical relationship. Sometimes that relationship is not specific enough. Sometimes there might be more layers of indirection. Sometimes the characteristic we're looking for requires a join across a few different components/queries, and we don't want each system inventing its own approach. Sometimes these properties rarely change.

Solution: a Relationship maintained by one system that runs when needed s/t most systems only need Relationship. Mob entities are `DenizenOf` a `Zone`, and `Zone` is attached to the `Level` entity. We can elevate who gets `DenizenOf` to its own system that runs when needed. We do not have to iterate through and filter a level's `Children`. Most importantly, we do not need to have a direct parent/child relationship with Level in order to surface mobs in a useful way.
