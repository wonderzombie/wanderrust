# RESPAWNING

## SUMMARY

When the player dies (or possibly teleports), non-boss enemies should respawn as if the level has been loaded for the first time: full health, no status effects, original positions. 

Doors and chests should NOT reset.

## REQUIREMENTS

1. respawn enemies afresh: original positions, original stats.
2. ... including defeated enemies (but not bosses)
3. do not respawn chests and/or doors.

## RELEVANT PIECES

- Enemies which are alive need fresh `Parameters` and their original `Cell`.
- Defeated enemies receive `Dead` and lose a number of components: `AgentPos`, `AgentOfGrid`, `Turn`, and `Blocking`. These prevent them from showing up for most systems. They are NOT despawned or marked `Disabled`. 
- Enemies are `Interactable::Belligerent`, which drives `process_action()` so the player can attack them. 
- `detect_belligerents()` uses `Added<Interactable>` to add `Name` and `CombatantBundle`.
- `init_combatants()` adds acquires `Parameters` and `Health` to entities with `Added<Combatant>`. 

## DESIGN

### HIGH LEVEL

If possible, I'd like to have exactly one way to initialize combatants, and the most solid way to store that information in a place other than the WorldSpec is via spawn points. We need this concept for shrines and it works well for enemies, too — a spawn point can contain a reference to the type of enemy it needs to spawn without knowing much about it in the first place. 

The main point of contention is how to deal with how different Interactable::Belligerent actually *is* in practice. In our sketch here, it's the concept of a combatant *and* the respawn point for a combatant. How can we reconcile that? I have one idea.

Rough proposal:

**For LDtk**:
Ideally, we place spawn points instead of enemies. The data would stay the same as it does, so that `Bat` would not be an LDtk `Actor.Combatant` (or what have you). It would be an `EnemySpawn`. The tile and/or name would determine the enemy type such as stats.

**For WorldSpec**:
This is a little tricky since LDtk -> wanderrust uses `Interactable::Belligerent` directly, and `Interactable` is how we pivot on whether the player is attacking an entity. 

For the time being, we can leave this part as-is, and handle this downstream.

**For interactable.rs**:
This is a little hinky, I admit, and exposes how weird it is to have `Interactable::Belligerent` in the same bucket as non-respawning things like `Door` and `Chest`. For `Belligerent` specifically it's a two step process: spawn all the interactables and then `combat.rs` picks up `Belligerent` specifically.

But we're saying goodbye to spawning those entities directly: ostensibly we want `spawn_interxs()` to spawn *spawn points* for *one* kind of interactable. It's not a disaster but it is a sign. 


**For combat**:
- `detect_belligerents()` only sets a marker such as `NeedsInit` for `Interactable::Belligerent`.
- `init_combatants()` does largely the same work except:
  - Entities are init'd when they have `With<NeedsInit>`.
  - `Interactable::Belligerent` is used in lieu of `&TileIdx, &Name`.
  - `Parameters` not `insert_if_new()` but `insert()`.
  - `CombatantBundle` already includes `Awareness`.

**For pathfinding**:
- `init_agents()` processes entities with `Awareness` without `AgentOfGrid`.

**For equipment**:
- This mechanism only exists for the player presently, `on_player_added`. 

**For position**:
- lol TBD lol

The remaining issue: what `Cell` they originated.

## NOTES

Note the requirements: *original position*. The place this is stored is in WorldSpec and while it's organized very well for creating each such it is not meant for random access to a subset of interactables.

### NEW DATUM: ENEMY SPAWN POINT

Brainwave: let's not reinvent the wheel. Let's use spawn points. 

I am not sure how we will put them in just yet. But we have precedent already. I have some design sketches that look something like this:

```rust
// Component, Copy, Clone
pub struct Spawn {
    pub cell: Cell,
}

// alternatively
pub struct SpawnBundle {
    pub spawn: Spawn,
    pub cell: Cell,
    pub child_of: ChildOf,
}
```

#### COMPONENT OR RESOURCE?

In Bevy 0.19 this is a little confused because `Resource` and `Component` are undifferentiated except via API. However, there's one invariant for `Resource` that we get for free: singleton.

**COMPONENT**

We could modify what type of `Spawn` using marker structs, possibly namespaced under `mod spawn;` so we can keep the names this simple:

```rust
pub struct World; // initial spawn point for player
pub struct Current; // lives on the player's last chosen respawn point
```

This enables patterns like `Single<&Spawn, With<spawn::Current>>`. Or, if we use `SpawnBundle`, we may want `Single<&Cell, With<spawn::Current>>`. 

Enemy spawns can be defined in LDtk much like enemies are: use a name and/or tile. 

```rust
// could be a tuple struct instead
pub struct Enemy {
    pub tile: TileIdx,
    pub name: Option<String>, // for overrides, e.g. BossBat
}
```

Then we need just `Query<(&spawn::Enemy, &ChildOf), With<spawn::Pending>>`. 

For insertion/removal, this might require a SystemParam for the known-to-be-current spawn and another for "all spawns." It's possible that we could use `Has<spawn::World>`:

```rust
pub foo(
    all_spawns: Populated<(&Spawn, &Cell, Has<spawn::World>),
) {
    if let Some(extant) = all_spawns.iter().find(|(_, _, has)| has) {
        commands.entity(extant).remove::<spawn::World>;
    }

    commands.entity(next_spawn).insert(spawn::World);
}
```

This is fine but we're handling datatypes with way more information than we need.

**RESOURCE**

For the preceding reason, I lean toward `Resource` for `World` and `Current`. There IS and there MUST only be one, and the engine can enforce that for us with no muss or fuss.

```rust
// derive resource
pub struct Current(pub Entity);
pub struct World(pub Entity);

// SystemParam is like this:
pub fn respawn(
    current: Ref<spawn::Current>) {}

// Example: initialize Current with World
let World(spawn_nt) = *world;
commands.insert_resource(spawn::Current(spawn_nt));

// Example: set to next spawn point. Hand-wave `Shrine`:
let Shrine { spawn_entity } = *shrine;
commands.insert_resource(spawn::Current(spawn_entity));
```

There *is* an extra step to get the `Cell`.

```rust
pub fn respawn(
    mut commands: Commands,
    current: Ref<spawn::Current>,
    spawns: Query<&Spawn>,
    player: Query<Entity, With<Player>>,
) {
    let Current(spawn_nt) = *current;
    let Ok(Spawn{ cell }) = spawns.get(spawn_nt) else {
        // handle error
    };

    // approximate
    commands.entity(*player).insert(*cell);
}
```
#### spawn points or belligerents?

The main point of contention is how to deal with how different Interactable::Belligerent actually *is* in practice. In our sketch here, it's the concept of a combatant *and* the respawn point for a combatant. How can we reconcile that? If we want to have interesting data on types like Bat or Skeleton, it's somewhat counterintuitive to treat them like spawn points, and then have data like patrol routes living on spawn points.

On the other hand, a belligerent is ephemeral. It *will* be respawned along with all the others, so the notion that we create it once at the beginning is false. 

Well, we might keep them separate — place a belligerent, place a spawn point for it — but nobody wants to do that. It should be very simple: put an enemy down *here* and then when the level resets we do that again. 

I can see many answers. The topmost one — putting SpawnCell on Belligerent — honestly seems like the most elegant and direct solution _for now_. 

1. Each `Belligerent` will actually have a `Cell`. We don't add a spawn point for an enemy; it's implicit in the definition of an instance of the entity in the level editor.
2. `spawn_worldmap()` will create spawn points for `Interactable::Belligerent` by iterating through `interxs`.
3. `ldtk_loader.rs` will, in the clause where it's matching on `Interactable`, it adds `Belligerents` to a separate field in `LevelSpec` like `belligerents()`. (This could be a bridge to Belligerent as a separate type.) `detect_belligerents` could be the part that creates the spawn points, and initially marks them as `NeedsSpawn` or what have you. `init_combatants()` would operate on `NeedsSpawn` entities. 
4. `spawn_interxs` takes two steps: spawn everything but belligerents, and then create EnemySpawn points with the name/tile/cell (either as components or as fields directly on it), which we can then mark as `NeedsSpawn`. `init_combatants()` would do the needful.

### NEW FLOW: belligerents with cells

The notion is that we don't ever despawn `Interactable` entities, so we never "lose" `Interactable::Belligerent` in actuality — they're just dead. Respawning is a matter of re-initializing the entities based on the data already there in `Belligerent`. We *don't* remove `Combatant` either. Sketch a flow:

1. First run: spawn `Belligerents` as normal, not touching `spawn_interxs()` except perhaps to add `NeedsSpawn`.
2. `init_combatants()` adds the components to the correct entities — regardless of whether they have them or not, so that the logic stays "flat." In particular `init_combatants()` will insert `Cell` from `Belligerent`. 
3. Reloading the area? Start by flagging each combatant with `NeedsSpawn`. `init_combatants()` engages, refreshing `Parameters` and removing `Dead` (since `NeedsSpawn` means they aren't supposed to be dead).

It's *also* possible for `init_combatants()` to despawn all entities that have `NeedsRespawn` *and* since it will still have the Components in memory it can spawn fresh entities — **no special handling, VERY specifically** — before spawning a fresh set of entities using `Belligerent` as the blueprint. 

To be explicit: this works because when we use `Commands`, the effects aren't visible until after the system runs. That means that we can use these constructs:

```rust
pub fn foo(
    interxs: Query<(Entity, &Interactable, &ChildOf), With<NeedsSpawn>)>,
) {
    for (nt, interx, child_of) in interxs {
        // Later maybe we can extricate Belligerent from Interactable somehow,
        // such as by storing this as a spawn point (i.e. `Spawn { name,
        // tile_idx, cell }`) and rely on `Combatant` to determine if an entity
        // is fight-able. This would move the logic from `process_interactions` to
        // `process_actions`. As it is, Examine -> Attack in some cases, which is a bit silly.
        let Interactable::Belligerent { name, tile_idx, cell } = interx else {
            continue;
        }

        // We don't need to figure out how to "repair" or "refresh" the old entity.
        // No need to remember to re-add Turn or remove Dead or NeedsSpawn.
        commands.entity(nt).despawn();

        // hand-wave params and health; we get these from Bestiary
        
        // Logic that wants to know when any of these components are added shoudl work OK.
        // TODO: consider whether a subset of these could go into actors.rs or mobs.rs
        // since we may want similar logic to this for NPCs that wander but don't fight.
        // The entry point for mobs.rs or similar might be nonexistent yet, though. 
        commands.spawn((
            Name::new(name.clone()),
            // CombatBundle adds Awareness which `grid.rs` will use to add `AgentOfGrid`, et al.
            // This also adds Turn and Combatant.
            // TODO: add PieceBundle to CombatBundle?
            piece_bundle: PieceBundle {
                cell: *cell,
                sprite: atlas.sprite(),
                // TODO: add FOO_LAYER_XFORM for (0., 0., *FOO_LAYER) to TileMap
                transform: *tile_map::ACTOR_LAYER_XFORM,
                ..default()
            },
            CombatBundle::default(),
            // Possibly this should move to PieceBundle or CombatBundle
            *tile_idx,
            params,
            health,            
            child_of,
        )).observe(on_attacked);
    }
}
```


#### SHRINES

A `Shrine` might be a little tricky, but perhaps not. What are the access patterns? 

- We will want to set the spawn point associated with a shrine separately from the shrine, much like an `EntityRef`, so a designer can/must set a "known good point" for each shrine, avoiding the need for logic.
- We will want to have shrines as a variant of `Interactable`.

`process_interactions()` would be the entry point. We destructure the shrine and set it up so that we have only  `commands.entity(shrine.spawn_nt).insert(spawn::Current)`. 

#### NEW FLOW: SPAWN POINTS

- despawn all `Combatant`; 
- tag each spawn as `spawn::Pending`;
- use such as `init_combatants()` for to:
  - `Query<(&Cell, &Spawn), (With<spawn::Enemy>, With<spawn::Pending>)>`
  - spawn the entity in that `Cell`;
  - add `Parameters` and `CombatantBundle`;
  - optionally spawn equipment?

Q: should `CombatantBundle` require `Parameters`?


#### SOME TODOS

- `process_actions()` could stipulate `ActiveLevel` and thus use `Single<&grid::SpatialIndex>`.
- `process_actions()` could possibly (?) use `Zone` with `ActiveLevel`
- `process_actions()` should figure out if an entity is among combatants and dispatch attacks — it is getting the entity from the `SpatialIndex` anyway
- change `interaction_attempts` from a `Message` to something like `Res<Examine>`; `process_interactions()` can then use `If<Res<Examine>>`. This eliminates a SystemParam.
- `process_interactions()` should stop using `Belligerent` once `process_actions()` dispatches attacks; this will allow us to move spawning of combatants out of `interactions.rs`
- add convenience "zero-value except for Z" transforms to `tilemap.rs`, like `actor_layer()` if const won't work
