# RESPAWNING MONSTERS

## SUMMARY

In Souls-like games, when the player dies, the game reinitializes the scenario (the level and its interactable bits) so the player can try again. The player spawns at the last respawn point with which they interacted and the enemies reappear anew. This also happens when a player interacts with a respawn point that's already been interacted with once.

## OBJECTIVE

We want to restore a/the level to its state as if it was loaded for the first time, with numerous exceptions. This document is primarily about restoring enemies.

## BACKGROUND

Most enemies in the scenario reset to their original state before having been engaged, defeated, or otherwise interacted with. This means they have all their HP, return to their fixed positions, resume their patrol routes. 

There are many interactive aspects of a level that aren't enemies, and these *do not* typically reset. They are incremental forms of progression:

- Chests remain open and looted
- Items in the player's inventory remain there
- Doors that were locked remain unlocked
- Levers which were pulled or engaged remain so

Sometimes interactive NPCs can be tricky. They will retain their _internal_ state — i.e. if the player has talked to them — *and* having talked to them may set a flag such that when the area loads again the player won't see the NPC.

With unlocked doors, one-way doors, and levers, we can create shortcuts or alternate routes that the player can only exploit once they've progressed past a certain point in the level. A classic example is the one-way door, or the elevator which can't be summoned via lever until it's been used once — by making it to the other side.

Most importantly, all of this means we cannot just "reload" the level as if it had never been interacted with because that would close doors, chests, and so on. 

No, we must specifically focus on enemies.

## DESIGN

We have two components used in identifying and cataloguing enemy entities: `Interactable::Belligerent` is what an enemy spawns with, and `combat::Combatant` represents an enemy that has been initialized.

`LevelSpec` is the wanderrust-native definition of a level, and defines interactables thus:

```rust
type InterxSpec = (Interactable, Cell);
```

With that in mind, we need a few things:

1. A way to know where an enemy needs to be
2. A way to know what kind of enemy it should be
3. A way to know when we need to reinitialize enemies
4. A way to do the preceding even when an enemy is dead

A proposal:

- Every `Belligerent` will get a `Component` that's a newtype for a `Cell` called `SpawnCell`.
  - This is initialized from `InterxSpec`. The enemy will return here when respawning.
  - This component nor Belligerent ever changes. They *may* be copied/cloned from one entity to the next since they are very simple.
- `bestiary.rs` allows us to re-initialize a monster's stats using its `TileIdx` or `Name`, which are two fields already present in `Belligerent`, to obtain the enemy's `Parameters`.
- We know we need to reinitialize enemies when we exit a `GameState::Defeat` state and enter `GameState::AwaitingInput`.

To reinitialize:

```rust
fn reinit_enemies(
    mut commands: Commands,
    // we may not need &Interactable since we have &TileIdx
    enemies: Query<(Entity, &TileIdx, &Interactable, &SpawnCell), With<Combatant>>,
    zone: Query<&Zone, With<ActiveLevel>>
) {
    // only iterate on entities in the currently active level. this assumes that
    // the game has reset the ActiveLevel to where the player respawns.
    for (nt, tile_idx, interx, spawn_cell) in enemies.iter(zone.iter()) {
        // handle "this ain't a thing" more intelligently than this
        let params = Bestiary::from_tile(tile_idx).or_else(|| Parameters::default());

        
    }
}
```

### elaborating the respawn mechanism

Add a new `Component` tied to `Belligerent` *or* add a field on `Belligerent`. The meaning is `SpawnCell` or `OriginCell`. We'll use this to return enemies to their place.

We'll trigger enemies respawning when the game's state changes from Defeat to AwaitingInput. 

`Commands` are deferred in a queue, so we `despawn()` each combatant and we will still have all of its components in local scope: 

```rust
// the following is a sketch
for (nt, tile_idx, cell, interx, child_of, spawn_cell) in query.iter() {
    commands.entity(nt).despawn();
    
    let params = Bestiary::from_tile(tile_idx).unwrap_or_default();
    // TODO: sprite
    commands.spawn((
        *tile_idx,
        *cell,
        *interx,
        *child_of,
        params,
        *spawn_cell,
        CombatantBundle::default(),
    ));
}
```

It's more than likely that we would want a `QueryData` for those fields rather than a long destructuring tuple. In this way we can encode what we need without worrying whether or not we have everything.

Sketch:

```rust
#[derive(QueryData, Clone)]
#[query_data(derive(Debug))]
pub struct RespawnData {
    _t: &'static Actor,
    tile_idx: &'static TileIdx,
    cell: &'static Cell,
    interx: &'static Interactable,
    // spawn_cell: &'static SpawnCell,
    sprite: &'static Sprite,
    child_of: &'static ChildOf,
}

impl RespawnData {
    fn as_bundle(self) -> impl Bundle {
        let Self {
            _t,
            tile_idx,
            cell,
            interx,
            sprite,
            child_of,
        } = self;

        (
            *tile_idx,
            *cell,
            interx.clone(),
            sprite.clone(),
            child_of.clone(),
        )
    }
}

// elsewhere
for (nt, respawn_data) in query.iter() {
    commands.entity(nt).despawn();

    commands.spawn(respawn_data.as_bundle());
}
```

### or opt-in cloning to mirror initial spawn?

[`EntityCommands`](https://docs.rs/bevy_ecs/latest/bevy_ecs/system/struct.EntityCommands.html) has what appears to be a much simpler option:

```rust
pub fn respawn_combatants(
    mut commands: Commands,
    combatants: Query<Entity, With<Combatant>>,
    zone: Single<&Zone, With<ActiveLevel>>,
) {
    for nt in combatants.iter_many(zone.iter()) {
        commands.entity(nt).hpawn_with_opt_in(|builder| {
            // we can choose to enable moving components *if* we want
            builder.allow::<(
                // This is the key component that will cause combat.rs to initialize
                // this "new" entity as a combatant, complete with Name and Parameters.
                InterxBundle,
                SpawnCell,
                ChildOf,
            )>();
        });
    }
```

`InterxBundle` is essentially how the combatant was spawned in the first place — it is effectively the bare minimum set of components for most `Actor` entities. 

In this situation, shorter the allow list, the better. There is some risk of carrying over components we haven't even invented yet that don't belong *if* those components are added when a thing spawns *and* is not needed when respawning. 

The scenario to be concerned about: What if there is a component that 1) belongs on an entity spawned the very first time; so it is 2) included as part of the default bundle; but is 3) *not* supposed to go on an entity that has *respawned*? 

I think the main risk would be components that are not frequently set or unset. `Visibility` is one of those. If we copy `Visibility` over and the entity is `Visibility::Hidden`, I think it could stay that way.

In practice, `sync_actor_light_levels()` is one of the few systems that sets `Actor` visibility, and it runs unconditionally — no `Populated<D, F>` or `Changed<T>`.

### digression: invisibility

If we need an Invisibility effect, we may have to adjust this:

```rust
        actor_vis.set_if_neq(if revealed.0 {
            actor_sprite.color = Color::WHITE.with_alpha(lighting.into());
            Visibility::Inherited
        } else {
            actor_sprite.color = Color::BLACK.with_alpha(0.0);
            Visibility::Hidden
        });
```

I think we should pick a lane: Visibility or alpha. Game effects can then use alpha such that the actor really is there, just transparent. Visibility is closer to a rendering concept; if the tile isn't rendered, then the actor on it isn't, either.

That said, it bears investigation, since the lighting/transparency effect is important. It may be that we reset alpha to 1.0 when we hide the actor.
