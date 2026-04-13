# TILEIDX: FLIP

A Component would work well: `pub struct TileTransform { flip_v: bool, flip_h: bool, flip_d: bool, }` then Query `Option<&TileTransform>` alongside `TileIdx`.

This is cleaner than flipping bits.

## WHEN TO FLIP

Lots of ideas but many impractical.

- Run a ptable phase after. Use the same seed each time. Flip tiles in a noisy way.
- Put flips in `procgen` itself. Use some part of "random" noise.
- Some maps don't want these at all.
