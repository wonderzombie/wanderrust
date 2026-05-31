# wanderrust: Northstar, LDTK, World Depth

To make our combination of Northstar and LDtk work, we're going to need to use one 3D grid for all layers. For a map that has two levels, we might have one level at Z=0 and another at Z=2. When cells are `default_impassible()`, it's not possible to move freely between layers *if* there's an impassible layer between.

We can continue to use our own Portals as `bevy_northstar` does not require their use. 

## changes

- We use one grid; `default_impassible()`; with `(n*2)-1` layers (1 -> 1; 2 -> 3; 3 -> 5; etc).
- `spawn_grid` creates the grid based on `Query<&Level>` and `Res<WorldSpec>`. 


### SCRATCHPAD

`spawn_grid()` is going to want an accurate count of the number of distinct depths. this is fairly specialized so I think it's on `spawn_grid()`. 

The canonical example is at depth 0: `level_0`; at depth 1: `(level_1, level_2)`. `world_spec.maps` has `Level(level_nty, _)`. Maybe filter_map `levels.get(level_nty)`, maybe error out if there's a mismatch. The Purpose: collect distinct depths to ensure that the grid we create has at least one or two layers that are functionally impassible.  

Overarching issue: if we have a 3D grid and layers are on top of one another, up/down movement becomes legal; there's no "floor" between a grid at z=0 and z=1, so we will make sure there's at least one. Level 9 at `grid_z=9` and level 2 at `grid_z=2` don't need handling, right? Nah, I think it will be easier on us if we *start* with normalizing. In the canonical example, depths are 0 (`level_0`) and 1 (`level_1`, `level_2`). If levels 1, 4, and 99 are at Z=level, the depths are 0, 1 and 2. 

That makes the math easier and we can emit a warning when we normalize.
