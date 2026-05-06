# LEVEL-WISE SPAWNING

The design(s) for the `Tilemap` family was based somewhat on the implementation in Godot that used `TileMapLayer`. LDTk *can* work the same, and that's roughly how the prototype has used it until now. When we add in another area at the same world depth and its origin is below an existing level, some assumptions no longer hold.

## BACKGROUND

We have a pipeline that's two steps. The step hop is from JSON to Rust, starting with `serde_json` and ending in various `LdtkFoo` types. The second hop is from Rust to wanderrust (incl Bevy), where we go from `LdtkLevel` to `Stratum` and so on.

The LDtk format has approximately three levels of depth: the "world," "level," and "layer." Example in the UI, we have two layers in our project (entities and tiles), and our world has three levels: one (`level_0`) at depth 0 and two more (`level_1`, `level_2`) at depth 1. On disk, our project defines some world properties; and it defines three levels. Each level has two layers: one with entities, and one with tiles. 

Levels 0 and 1 each have their origin at [0, 0]; so grid [1, 1] is [16, 16] is — in Bevy — the bottom left, aka one away from the origin. Levels 0 and 1 are, in fact, stacked directly on top of one another. Levels have their own coordinate system, so Level 2 has its origin at [0, 256] because level 1 ends at grid [0, 15] and level 2 begins at grid [0, 16]. Level 2 is also 32x32, whereas Level 1 and 2 are each 16x16. 

With TileMapLayer, I used Godot's node hierarchy: a Stratum was a special TileMapLayer, and its children were entities and suchlike. Most were related to Node2D one way or another and this allowed me to treat a stratum as a whole unit, hiding or disabling it (and everything on it) or applying a visual effect as needed. We had one TileMapLayer per stratum because the game was divided into zones, typically from a world map to a more immediate, point-of-interest view, so a zone had-many strata.

Levels 1 and 2 would not be separate levels if they were on the same stratum. This is essentially the status quo in `wanderrust`, too, until I added level 2 and put it below level 1 to see what would happen. It did not work. _And it's probably fine for now!_

But I had given it some thought, and since LDtk is a real thing, there's a chance this could be useful later. 

## DESIGN

This *could* be a big change but I am not so sure. The advantages would be things like "neighbor levels" and the ability to divide larger areas into much smaller pieces. If our outdoor area is as large as 300x300, let's say, some light chunking would surely help. (Note that `bevy_ecs_tilemap` does actually render tiles to a material, so future self, you should strongly consider using it.) 

I've considered a few approaches. This is more like a walkthrough. We have a pipeline that's two steps (arrows): `wanderrust.ldtk` -> `LdtkProject` -> `WorldSpec`. The first hop is from JSON to Rust. The second hop is from Rust to wanderrust (incl Bevy).

To keep it simple, I wanted to use the world grid. We allocate slots in `Vec<Option<Entity>>` big enough for `world_grid_x * world_grid_y`, and we remember `world_grid_x` so we can translate between coordinates (`Cell`) and index in the lookup (`usize`). Systems can access `TileStorage` for a given stratum as usual.

To put level 2 where it's supposed to go, we'd need `LdtkLevel` need to surface `world_x` and `world_y` (in pixels). Internally, its coordinates [0, 0] map to position (0, 0), so it's going to say [25, 13] for the coordinates of an entity — relative to world position (0, 256). In _world grid coordinates_, the position of the same entity is [25, 29]. In other words, we need either to keep these systems straight, or pick one as early in the pipeline as possible.

Or maybe we can just short-circuit that: `TilemapBundle` is used to create the `Stratum` entity, which is itself the parent to all the tiles and entities. Moving the stratum entity to `world_x` and `world_y` should let us have it both ways. Any changes to the transform for a given entity will be relative to its parent.
