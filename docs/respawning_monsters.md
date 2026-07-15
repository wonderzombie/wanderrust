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
