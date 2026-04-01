# STRATA


## concept 

- A map is divided into layers called strata.
- The player can see strata below them but not above them. Example: a cliff or a hole in the upper floor of a house.
- The player can move between strata at certain places.

Not in scope:
- Falling through holes
- Combat between strata 

## relevant pieces

- `TilemapSpec` and `SavedTilemap` represent a map — first one is at runtime and the other one is at save/load.
- Presently the `Strata` datatype exists and sits in both of the types in the preceding point.
- We have `Portal` which contains an ID and points to another of its kind. The player will "teleport" between them.
- `TileIdx` draws no distinction between "empty" and "blank" — there is not a concept of a "missing" tile that would allow vision through the "floor."
- There isn't really a content pipeline as such, so prototyping might be challenging. 
- `TileStorage` maps `Cell`s to `MapTile` entities and it's the only such – `Single<&TileStorage>`. 

## requirements / affordances

- Start with two strata: `Above` and `Below`. 
- When the player is on a stratum marked `Below`, player cannot see `Above`.
- When the player is on a stratum marked `Above`, the player _might_ see `Below`.

- When the player transitions (ostensibly via Portal) from one stratum to another:
  - move the player to the new stratum
  - if `Above`, `Below` changes visual appearance
  - If `Below`, `Above` disappears altogether.

- Deduce active stratum from player's Cell -> TileStorage -> tiles. 
- Then show/hide based on parent.

## proposal one

### stratum is a concept and a `MapTile` component

- Keep TilemapSpec (mostly?) as-is.
- Keep Stratum as a marker for tiles, and keep enum (for now).
- Keep (TileIdx, Cell, Stratum) for now.
- Portals gets Stratum, too: (Portal, Cell, Stratum).


### initialization / creation / layout:

- Each stratum's entities are children of `TilemapLayer` which is a child of `TilemapId`.
- `TilemapLayer` entity gets `Visibility` for show/hide.
- `TilemapLayer` _might_ look like `TilemapLayer(Stratum)`.

### usage / flow


#### one 
- The active stratum _might_ be deduced from the cell the player is standing on.

- Get player cell.
- Get tile entity.
- Get "active" stratum for tile.
- Get TilemapLayer for active-and-not

```rust
pub fn update_strata(
    cell: Single<(&Cell, Option<&PreviousCell>), With<Player>>,
    storage: Single<&TileStorage>,
    tiles: Query<(&Stratum, &ChildOf), With<MapTile>>,
    layers: Query<&TilemapLayer, With<TargetedBy>>,
) {
    
}
```

This approach involves fairly minimal changes to the existing code.

#### two

It _might_ be simpler to:
- mark `TilemapLayer` with `ActiveLayer` _and_ `Stratum`.
- keep `Stratum` on disk for now, but not in memory
- use Relationships to get `TilemapLayer`


Initialization from `TilemapSpec` or `SavedTilemap` will have to change. We can start from the perspective of `tiles`. 

- create TilemapId as usual
- partition tiles by Stratum
- create each `TilemapLayer(Stratum)` as child of `TilemapId`
- spawn that stratum in a batch as child of `TilemapLayer`
- NEW: tag `TilemapLayer` with `ActiveLayer`. 

To discern whether to update active stratum or not:

- use player &Cell to get Entity from TileStorage
- use tile Entity to lookup into `<ChildOf, With<MapTile>>`.
- use `Single<&TilemapLayer, With<ActiveLayer>>`.
