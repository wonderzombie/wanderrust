# BIG TILES

- Some tiles could be 2x as large for to show a giant/ancient place. Think of the HUGE bricks in Siofra, Ainsel, and possibly also Nokron or Nokstella.

Technical thoughts:
- *Basic concept:* 4x logical cells map to the same entity. This should take care of systems relying strictly on **cells**, as they just see the same entity over and over. 

NB that some systems may make assumptions about same/different tile — but they should not! They should be using cells to get tiles and check properties on each entity without making assumptions about cardinality.

Systems relying on tiles/sprites should only need to know that, once in a while, a *sprite* is 2x as large. That way, when it loads, it comes up as 32x32. 

## relevant pieces

- `SpriteAtlas` - the source for sprites (atlas.rs)
- `spawn_tilemap()` calls `spawn_maptiles_from_spec()` (tilemap.rs)
- `TilemapSpec` contains `(TileIdx, Cell, Stratum)` (tilemap.rs)
- `TileIdx` encodes `atlas_index`, used by `sync_tiles()`. (tiles.rs)

Example: say we have a 4x4 area defined as [0, 0] to [3, 3]. The usual (flat) layout would be `Vec<Option<(TileIdx, Cell, Stratum)>>` with length 16. _Simplified_ it looks like `vec![Some(TileIdx::Blank); 16]`. 

## sketchin'

### how to define big tiles

I already had a couple of ideas here, for tiles that look like other tiles but have arbitrary, possibly one-off and/or ad-hoc properties (i.e. added later by/for some system or another).

With 1078 sprites, sprite at (offset, not ordinal) 1078 resolves to the same tile `0`. `TileIdx` can refer to arbitrary sprite indices. This was an idea I worked out already and it seems sound, just wasn't needed yet.

`impl TileIdx` defines constants like `WALKABLE`. Call it whatever: `JUMBO` or `BIG` or `GIANT` or `OUTSIZED` or _whatever_.

This covers how we create reference data.

### how to draw big tiles

We'll leave `SpriteAtlas` untouched. It just maps `impl Into<usize>` to `TextureAtlas::index` in `bevy_sprite::Sprite`. That's great.

- `Sprite` has `custom_size`: `Vec2::splat(TILE_SIZE * 2)`. 
- Don't forget `SpriteImageMode::Scale` — `SpriteScalingMode::FitStart` should mimic scaling from bottom right.

### how to sync big tiles

`sync_tiles()` is the logical place for this. We already check things like `TileIdx::is_walkable`. `TileIdx::is_outsized` checks `OUTSIZED`.

`OUTSIZED` has the alternative ID.

Presently, `tiles!` defines `StoneWall = atlas_idx(0, 13)` which is 637.

**Semi-real example:**
1. Define `atlas_idx_page(x: u32, y: u32, page: u32)`. 
1. `tiles!` gets a new line: `StoneWallBig = atlas_idx_page(0, 13, 1)`.

**Q:** Is it correct that `impl From<TileIdx> for usize` should change? 
**A:** Although I don't love the idea, it's simpler, yes. Also, the docs are unclear what happens if we exceed the number of textures implied by `TextureAtlasLayout`.

#### playground code: `atlas_idx_page`

```rust

const DIMENSIONS: [u32; 2] = [49, 22];
const NUM_TILES: u32 = DIMENSIONS[0] * DIMENSIONS[1];

const fn atlas_idx_page(x: u32, y: u32, page: u32) -> usize {
  let page_offset = NUM_TILES * page;
  ((y * DIMENSIONS[0] + x) + page_offset) as usize
}

const fn atlas_idx(x: u32, y: u32) -> usize {
    atlas_idx_page(x, y, 0)
}

pub struct TileIdx(usize);

impl From<TileIdx> for usize {
    fn from(value: TileIdx) -> Self {
        value.0 % NUM_TILES as usize
    }
}

fn main() {
    
    println!("Hello, world!");
    
    assert_eq!(atlas_idx_page(0, 0, 0), 0);
    assert_eq!(atlas_idx(0, 0), 0);
    assert_eq!(atlas_idx_page(1, 0, 0), 1);
    assert_eq!(atlas_idx(1, 0), 1);
    assert_eq!(atlas_idx_page(0, 1, 0), 49);
    assert_eq!(atlas_idx(0, 1), atlas_idx_page(0, 1, 0));
    assert_eq!(atlas_idx_page(1, 1, 0), 50);
    assert_eq!(atlas_idx_page(0, 13, 0), atlas_idx(0, 13));
    assert_eq!(atlas_idx_page(0, 0, 1), NUM_TILES as usize);
    
    let tile_idx: TileIdx = TileIdx(atlas_idx_page(0, 13, 1));
    let atlas_i: usize = tile_idx.into();
    assert_eq!(atlas_i, atlas_idx(0, 13));
    assert_ne!(atlas_i, tile_idx.0);    
}
```

### how to serialize/deserialize big tiles

This part might actually end up being the dominant factor, maybe changing the preceding more than I would like. Let's get to it.

Recall that we store `(TileIdx, Cell, Stratum)` in `Vec`. For now we will ignore `Stratum`. 
We'll use the 4x4 example with the 2x2 starting at [0, 0]. 

Let's work through the "dumbest" scenario: we don't change the format _at all_. Without any special handling, the following would be undifferentiated from StoneWall, given a slightly altered implementation of `From<TileIdx> for usize`:

`vec![(BigStoneWall, (0, 0)), (BigStoneWall, (1, 0)), /* ... */, (Blank, (3, 3))]`

In `load_map`, as we go through the (cell, tile) pairs we can see which ones would be in `OUTSIZED` and we're declaring that this means instead of 1:1 tile/cell it's 1:4 tile/cell. 

Given all this, I can see two major approaches: a two-pass approach where, in some order or another, we load regular tiles as usual and outsized tiles get special handling; or an approach where `TilemapSpec` encodes this. 

I favor the latter slightly because it's data-driven.

#### TilemapSpc saves outsized tiles

`save_map` is pretty straightforward right now because `SavedTilemap`'s `tiles` field is `Vec<(TileIdx, Stratum)>` and we have `size`, so we can do `Cell::from_idx(width, i)`. Our goal is to keep this `outsized` field very small and simple and obvious.

So, when we have `tile_idx.is_outsized()` as a tile under consideration, how might we store the bare minimum information needed to reconstruct it? 

A very simple and obvious answer is to slot right into the existing index-based system. 

We do l-to-r and top-to-bot. In the 4x4 example, BigStoneWall at (0, 0) also covers (0, 1), (1, 0), and (1, 1), so we only need to store two things: the TileIdx of the outsized tile, and where the upper left corner is, probably as an index. We would not need to add a field to `(TileIdx, Stratum)` pair. 

Given `outsized: Vec<(TileIdx, usize)>`, there would be a 1:1 ratio of `outsized.len()` and the number of outsized tiles in the map. To wit, `(BigStoneWall, 0)`.

This addresses how we record such as far as future generations are concerned. The issue, of course, is that iteration doesn't stop there. The very next tile in iteration order would be `i = 1` and `BigStoneWall` so we can't store that without confusing the matter; we might accidentally describe a 3x2! 

> **Assumption:** we do not have overlapping outsized tiles.

Well, it's not so bad: when we find `(BigStoneWall, 0)`, we already know which tiles "belong" to this outsized tile. We wrote it in cell coordinates above. What the index is ultimately depends on the width of the map, which we have in hand already.

In our 4x4 example, we know immediately that the Vec indices for this particular outsized tile is (0, 1, 4, 5). But what do we _do_ with that information? 

Well, if index 1 ias BigStoneWall and index 2 is BigStoneWall, we need to know that 0 is the first one, and 2 is a second one. It's a similar story with index 5 and 6. What we've described so far is that we store the first index we find a big item, and we also know we need to avoid `outsized` looking like this: `vec![(BigStoneWall, 0), (BigStoneWall, 1), (BigStoneWall, 4), (BigStoneWall, 5)]`. 

I think the answer is that we can discard it when we're done, but we do need to know that index 4's outsized tile is spoken for so that we don't record it. 

Do we need to know *which* tile it belongs to? I don't see why we would; we just need to know what *not* to record as being the upper-left of an `outsized` tile.
