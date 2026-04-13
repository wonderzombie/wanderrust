# LOAD NEXT MAP

**Summary**: To go from one area to another, we will want to load another map from disk.

## formats & types

**Some alases:**
- type TileCell = (TileIdx, Cell)
- type PortalCell = (Portal, Cell)
- type StratTiles = HashMap<StratumId, Vec<TileCell>>
- type StratPortals = HashMap<StratumId, Vec<PortalCell>>

**Types:**
- A "map package" consists of a TilemapSpec with `(TileIdx, Cell)` and Portals `(Portal, Cell)`, saved separately, loaded separately (but unified in memory).
- `TileStorage` is `Vec<Option<Entity>>` with `storage.len() == spec.size.ntiles()`. `TileStorage::get()` maps `Cell` to an index for retrieval.
- Tile entities *don't need to be de/re-spawned* — keep Transform, Cell, and ChildOf as-is. Can safely replace Stratum *Component*; keep entity intact.

These types will need to be re-initialized and exist on a per-statum basis _at least_:
- SpatialIndex
- Fov
- Grid
- Light (emitters have LightMaps; strata have StratumLightMaps)

## re-initializing tiles

**Problem:** we have thousands of entities and we need to reset them to a known good state.

**Answer:** after removing `ChildOf` from the player, we choose to despawn each Stratum in turn. This is the cleanest most guaranteed way to start fresh. It also puts us in a position where it's just "next map" with no special "first map" case. 

`spawn_tilemap` in that case would be just fine. It may in fact be worth adding `spawn_tilemap`, `initialize_tile_storage`, and `setup_portals` to `PreUpdate` or `First`. They are chained in `PreSetup` at present. 

Maybe we can even run `GameSystem::SetupTiles`. 

## re-initializing actors
