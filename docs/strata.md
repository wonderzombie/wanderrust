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
