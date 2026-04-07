# Strata: LightMaps

## `lights.rs` works (without strata)

1. The basic primitive is an Emitter. `Emitter::light_cells()` accepts a cell and returns a HashMap of cells to their light levels. If we have a source with radius 1 (simplified example) and we put in (3, 3), we'd get a HashMap which would define a rectangle with UL at (2, 2) and BR at (4, 4). It's `radius * 2 + 1` because the provided cell is the "origin" from which the light "radiates."
1. When we merge LightMaps, the brightest cell wins; they don't combine.
1. Local<LightMap> is the light map calculated the time the system previously ran. This allows us to skip modifying cells that didn't change and we don't leak that info to other systems.

The first part of the procedure is like:

- We gather *all emitters* and prepare a fresh LightMap `new_map`: 
  - each Emitter generates a LightMap; 
  - each has a LightMap inserted; 
  - and we use LightMap::merge_with().
  
The second part calculates which cells need to be updated based on the two maps. Some tiles gain light. Some tiles stay the same. Some tiles need lighting removed.

    Example:
    - We have a torch near a candle. 
    - The torch is Bright with radius 2 at (10, 10), so it lights a 5x5 area. 
    - The candle is Lit with radius 1 at (12, 12), so it lights a 3x3 area. 
    
    If we move the Bright torch so its light doesn't overlap the candle's:
    - the Lit candle's tiles (the ones it lights) will remain lit at a lower level.
    - the Bright torch's tiles will light a bunch of tiles which were previously unlit.
    
    If we move the Lit candle out of the Bright torch's radius:
    - the tiles lit by the torch don't change; the torch is brighter
    - some tiles lit by the candle will change

## with strata?

Before, all Emitters were in the same space, so we only needed the one LightMap and TileStorage. Now we have two different places emitters can be: >1 LightMap and >1 TileStorage. We want to combine LightMaps selectively, independently of stratum.

**High level steps:**

1. Insert Emitter for any TileIdx for that `is_emitter()`. -- `setup_emitters` via `Changed<TileIdx>` -> insert `Emitter`.
2. When an Emitter changes or a Cell changes, build a new LightMap for that Emitter. -- `update_emitter_maps` raises entities whose Emitters and/or Cells have changed (such as when they're or inserted).
3. When any Emitter's LightMap changes, find its stratum and update the LightMaps accordingly.
4. When a Stratum's LightMap changes, re-apply lighting.

3 conceals a lot of detail.

## LightMaps go on Emitters -> combine at Stratum level

### a first stab

We have LightMaps at two levels now: at the Emitter level and at the Stratum level. We have, as I see it, a few  choices for how we want to iterate to build a new LightMap before we compare it to the old one.

Ultimately I think iterating over the number of emitters is OK. We don't want to do it every time, so the `Changed<Foo>` was mixing up the logic. For now we can say 

```rust
update_strata_lights.run_if(any_match_filter::<(Changed<LightMap>, With<Emitter>)>) 
```

And then our job is easier:

```rust
// ...
all_emitter_maps: Query<(&ChildOf, &LightMap), With<Emitter>>,
all_strata_maps: Query<(&Stratum, &mut LightMap, Option<&mut PrevLightMap>)>,
// ...
```

Here there is a fork in the road, however:

1. `HashMap<StratumId, LightMap>` because `all_emitter_maps` will have many strata; or
1. `itertools` has various group-related operations, so something like `all_emitter_maps.iter().into_grouping_map<ChildOf, LightMap>().collect()`.
