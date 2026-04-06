# STRATA


## concept 

- A map is divided into layers called strata.
- The player can see strata below them but not above them. Example: a cliff or a hole in the upper floor of a house.
- The player can move between strata at certain places.

Not in scope:
- Falling through holes
- Combat between strata 


Work level:
- Actually kind of a lot unless we try to centralize our component/entity hierarchies a bit first.

## summary

- Start with two strata: `Above` and `Below`. 
- When the player is on a stratum marked `Below`, player cannot see `Above`.
- When the player is on a stratum marked `Above`, the player _might_ see `Below`.

- When the player transitions (ostensibly via Portal) from one stratum to another:
  - move the player to the new stratum
  - if `Above`, `Below` changes visual appearance
  - If `Below`, `Above` disappears altogether.

- Deduce active stratum from player's Cell -> TileStorage -> tiles. 
- Then show/hide based on parent.

## background

- `TilemapSpec` and `SavedTilemap` represent a map — first one is at runtime and the other one is at save/load.
  - `SavedTilemap` does not explicitly include a cell. It means `tiles` will have `Dimensions::ntiles()` entries. 
- Presently the `Strata` datatype exists and sits in both of the types in the preceding point.
- We have `Portal` which contains an ID and points to another of its kind. The player will "teleport" between them. Each Portal component lives on a tile which we'll create as a ChildOf.

- `TileStorage` maps `Cell`s to `MapTile` entities and it's the only such – `Single<&TileStorage>`. 

- `TileIdx` draws no distinction between "empty" and "blank" — there is not a concept of a "missing" tile that would allow vision through the "floor."

- There isn't really a content pipeline as such, so prototyping will rely on the ASCII maps at first. Then we can save them as RON files and edit them by hand.

- Order of operations is going to be important when it comes to initialization.

- Visibility is important to get right. 

## systems needing updates

There are some `Resources` and/or `Components` that assume we'll have exactly one map. These need to change I mentioned one or two already. Off the top of my head:
- sync_tiles, update_tile_visuals
- LightMap, update_emitter_lights, sync_actor_light_levels
- Grid, 
- TileStorage
- SpatialIndex
- Fov
- Observers of MapTile (should use TileStorage)
- mobs::check_fov, pathfind, move_agents, etc

It's going to get even more interesting, though.

- save_map, load_map need to be aware of different strata when saving
- we may want to move to explicit (absolute) cells when saving. 
  - instead of having `ntiles` number of entries in `tiles`, we have everything that's not blank.
  - when we're initializing a new map, create `TileStorage` with `ntiles`. Then we fill in using `(TileIdx, Cell, Stratum)`. Many items in `TileStorage` will be `None`, and those can become `Blank`.
  - Later we'll want tiles that are not just blank but transparent, not opaque, but solid. We can do this through extended `TileIdx` — this is the proposal where we can define an arbitrary number of alternative tiles for any tile using "pages": 49x22 = 1078. 

## new structure

- TilemapId is the parent entity for almost every entity. 
- Every Stratum is a ChildoOf TilemapId
- Every MapTile is a ChildOf a Stratum
- The Stratum with ActiveStratum is the object of each system
- Portal lives on a MapTile which is a child of Stratum (for transitions)

We can start by putting each of these on a Stratum:

- TileStorage
- SpatialIndex
- Grid 
- LightMap
- Fov

**Does this mean we convert all those to use Query?** Not necessarily. See below.

Observers are another example.

**Using With<ActiveStratum> to narrow queries can work, but if it's in one place, it will need to be everywhere.** 

### could we still use Res<Fov>, for instance?

**Important:** the main draw of Res<Foo> is to keep existing systems from needing to query any `Stratum`. As soon as systems stop being able to use `With<MapTile>`, a bunch of logic will have to change. `Single<&TilemapStorage>` is inevitably going to change; that's OK. But maybe `Res<TileStorage>` is a better move overall. 

A system that sets the active stratum *could* update each `Res<T>` associated with a stratum. Example: an `Added<ActiveStratum>` system could replace a set of resources with ones from the active stratum when it changes.

There *is* one big advantage: none of the systems that use these resources would have to know about which is the active stratum. This means places that use `Single<&TileStorage>` _could_ be `Single<&TileStorage, With<ActiveStratum>>`. 

- Alternative: `Res<TileStorage>`, `Res<Fov>`, `Res<LightMap>`, `Res<SpatialIndex>`, `Res<Grid>`, et al, *always* describe the current stratum.
  - De-sync is a potential issue.
- Alternative: `TilemapSpec` loses `tiles`, gains `storage`, `fov`, `light_map`, `spatial_index`, and `grid` fields.
  - TilemapSpec becomes closer to an all-in-one rather than a description of how to make a Tilemap
- Alternative: `TilemapSpec` loses `tiles`, gains `storage`; the rest remain as Components
  - Not the worst mainly because `storage` only sort of makes sense without `TilemapSpec` -- we need size, et al.

### should we just use Components for these since they are associated with a Stratum?

This *is* one of the most straightforward approaches.

The trick is that we'd like to avoid `With<ActiveStratum>` all over the place. Literally anywhere we use `MapTile` would have to change.

### a hybrid approach

1. Keep almost everything as a component. Maybe still do `Res<TileStorage>`?
1. `Res<Stratum>` designates the active stratum. In this respect it's like `Res<TilemapSpec>`.

Systems only need `active: Res<Stratum>`, `query: Query<&SpatialIndex>`, and `query.get(active)` to get `SpatialIndex` — it is one extra param. This would be the same story as `TileStorage`.

But is this much better than `Query<&TileStorage, With<ActiveStratum>>`? Yes, in a few ways. `Res<Stratum>` change detection is useful; systems will read this parameter more often than they will modify it. 

### yet another approach

Some as-yet undesigned type *like* TileStorage, but Stratum-aware. Setting the active stratum also updates TileStorage so that `Single<&TileStorage>` will always operate on the active stratum.

## seralizing & deserializing

This is almost a whole thing. :P

Originally, we had `Stratum` as a Component on a MapTile so we could have `(TileIdx, Cell, Stratum)` as an "absolute" position: this is a tile at Cell on Stratum. We can tell just by looking at the triple where it goes. This makes saving and loading easier.

### aside: SavedTilemap

There's one major difference between SavedTilemap and TilemapSpec: SavedTilemap's `tiles` are `Vec<(TileIdx, Stratum)>`, meaning that for any given SavedTilemap, `saved.size.ntiles() == tiles.len()`, which allows something like `tiles.iter().enumerate()` to use `i` and `size` to derive Cells. What's stored on disk is `ntiles` number of tiles, even if 90% of them are blank. It also has `Vec<(Portal, Cell)>`.

There's not much reason why TilemapSpec couldn't take this over.

### save & load map

Save and load both use a combination of TilemapSpec, TileStorage, and SavedTilemap.

- TilemapSpec is a/the "pristine" version of the map. It tells you how to make it, what the defaults are, etc.

- TileStorage contains `ntiles` worth of `Option<Entity>` and includes `size` so we can map cells like `(1, 2)` to `Some(entity)` or `None`.
- TileStorage represents the map as it is at any given moment
- Save: TilemapSpec -> TileStorage -> SavedTilemap
- Load: SavedTilemap -> TileStorage -- **no TilemapSpec involved**, just `size` and others

- SavedTilemap represents tiles as `Vec<TileIdx, Stratum>` and in this way it mirrors `TileStorage`. 
- The rest of it mirrors `TilemapSpec`: size, light_level, layer.

### aside: strata in Godot

My Godot implementation used a Node2D at the root, like `SmugglersCave`, and had another Node2D for a stratum like `TheCave`, and in `TheCave`, we expect `_level`, the `TileMapLayer`. Everything interesting belonging to a map — actors, monsters, interactables — was a child of TileMapLayer.

In this way it was very simple to limit processing just to the visible stratum, the TileMapLayer.


## learning from prototypes: okay fine it's a HashMap

With the prototypes doing their job, we now move on to a less invasive approach to implementation, and honestly one of my favorite approaches: **no functional changes**. :P 

If we build for N strata and keep it at 1 it simplifies our problem considerably. 

A major principle here is to avoid significantly changing the way the system acquires and interacts with its inputs. 

1. Going from `Res<Fov>` to `Query<(&Children, &Fov)>` is totally fine. 
1. *Except* for the `setup_foo` example where we insert `Foo` on a stratum, we can avoid thinking about such altogether.
1. To find out the stratum for the player, `Single<&ChildOf, With<Player>>`. `With<Foo>` does some very heavy lifting here & elsewhere. `ChildOf` gives us an Entity suitable for `Query`'s `get()`.  
1. Detect active stratum changes via `Single<&ChildOf, (With<Player>, Changed<ChildOf>)>`.
1. Change active stratum via `Single<&mut ChildOf, With<Player>>`. **NB: `ChildOf` is the source of truth; `Children` is populated by Bevy.**
1. Toggle between `Visibility::Hidden` and `Visibility::Inherited`. 

### done

- `TileCell` is (TileIdx, Cell) and `PortalCell` is (Portal, Cell).
- `TilemapSpec` has `all_tiles: HashMap<StratumId, Vec<TileCell>>` and `all_portals: HashMap<StratumId, Vec<PortalCell>>`.
- A `Stratum` is defined as `Stratum(Entity, StratumId)`. `StratumId` is defined as `Stratum(i32)`. 
- A `Stratum` entity is parent to its tiles and portals.
- `TileStorage`, `Fov`, and `SpatialIndex` are Components on `Stratum` because each stratum will have different geometry.
- Tiles get `Visibility::Inherited` when "visible" of `Visibility::Visible`. 

One characteristic of this approach that's fallen out is that queries have gotten _considerably_ simpler. I think the hierarchical model is underrated. `Children` and `ChildOf` allow us to switch between "views" of the problem very easily. Both QueryData and QueryFilter become a **lot** simpler.

#### example: update_spatial_index

```rust
fn update_spatial_index(
    query: Query<(&Children, &mut SpatialIndex)>,
    tiles: Query<&Cell, Without<Walkable>>,
) {
```

1. query for the child entities of any entity with a spatial index (i.e. stratum)
2. enumerate the cells for all not-walkable entities
1. `query` drives iteration and mutation; `tiles` is the read-only lookup. 

#### example: setup_fov

```rust
pub fn setup_fov(
    mut commands: Commands,
    spec: Res<TilemapSpec>,
    stratum_children: Query<(&Stratum, &Children)>,
    tiles: Query<(&Cell, &TileIdx), With<MapTile>>,
) {
```

1. Read-only `stratum_children` gives us 1) something to attach Fov to as well as 2) all the child entities of same, MapTile or otherwise. 
1. Read-only `tiles` implicitly maps a Stratum's child entities to `(&Cell, &TileIdx)`. -- In theory, this should be `With<Opaque>`, but that's too close to a functional change in `setup_fov`.
2. `fov` is initialized based on size in `spec`, populated via `is_transparent()` and attached to stratum.

#### example: update_fov_markers

```rust
pub fn update_fov_markers(
    all_fov: Query<(&Children, &Fov)>,
    player_query: Single<(&Cell, &ChildOf), With<Player>>,
    player_stats: Res<PlayerStats>,
    mut tiles: Query<(&Cell, &mut Revealed), With<MapTile>>,
) {
```

1. `all_fov` has every `fov` for every stratum-entity alongside all the tiles which are children of that same stratum.
1. `player_query` with `ChildOf` gives us a stratum-entity to retrieve from `all_fov`.
1. `tiles` is a comprehensive list of cells and revealed status. child_tiles, aka `all_fov.0`, drives iteration and `tiles` is the means of mutation.



### pending

**`Grid` and `Fov` and `mobs::check_fov`**. Still uses `Res<Fov>`. `spawn_grid` and `update_grid` should operate on children of strata. 

Consider moving specifically grid-related functionality from `mobs` and `main` to something like `nav`.

**`process_actions`**. This relies on `Res<SpatialIndex>`. A naive query might be: `Query<&SpatialIndex>` paired with `Single<&ChildOf, With<Player>>` into `indices.get(player_child_of)`. *This is an area where ActiveStratum or some equivalent could make sense*: higher-level gameplay concepts like actions shouldn't really care about this.

**`LightMap` and `update_emitter_lights`.** This is straightforward with one exception: `Local<LightMap>`. The prior impl made great use of this. Now there exists one for each Stratum. `Query<(&Emitter, &ChildOf, &Cell, Option<&PreviousCell>), Changed<Cell>>` gives us the emitter, where it is now, and where it was before, if any.

Consider an abstraction that contains both `Cell` and `PreviousCell`. If `Cell` changes, `PreviousCell` must exist/change — they don't make much sense without each other. Maybe it's as simple as a `QueryData` type like `Mover`.

Consider `AgentPos`, `NextPos`, and/or `AgentOfGrid` could benefit from something similar (lower priority because `mobs.rs` is the only thing that uses it.) Or, again, QueryData type or type alias.
