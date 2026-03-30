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
