# ALTERNATIVE TILES

## SUMMARY

TileIdx is a component that describes a tile in sprite sheet wanderrust is using: a `usize` corresponding to the `TextureAtlas` data-type in Bevy, specifically the `index` field. Each tile index can be defined *exactly once.* `tiles.rs` define every `TileIdx`, and its implementation (`impl TileIdx`) defines various game-mechanical properties for each tile.  

There is only one `StoneWall` and systems that handle `Opaque` or `Walkable` use `tiles.rs` to adjust those marker structs, which other systems then use for things like FOV or pathfinding. This means we can't create obfuscated paths hiding in plain sight using the same visual representation because the TileIdx variant is tied to semantic properties. (I regret nothing.)

Instead of complicating the `TileIdx` datatype, we can use `usize` higher than the total number of tiles in our spritesheet to effectively create "pages" of alternative tiles. These tiles need different `TileIdx` variants and we can define their properties without affecting the "original."

The motivating use case is being able to define a transparent tile (i.e. tile 0 in `colored_packed-transparent.png`) as being solid. This would permit tiles underneath that tile to be visible to create a layered effect without allowing traversal.

## OBJECTIVE

- Allow arbitrary tiles to look the same as other tiles, but with different properties.
- Very strongly avoid making changes to the `TileIdx` API.

## BACKGROUND

*See [TileSetAtlasSource](https://docs.godotengine.org/en/stable/classes/class_tilesetatlassource.html) for the canonical documentation of the Godot features discussed here.*

Godot has alternative tiles and it uses a triple of (source_id, atlas_coords, alternative_id). Assume for now that there's just one `source_id`, probably `0`. 

`atlas_coords` corresponds a tile in the atlas, and in the most elementary case, we can imagine the triple always starts and ends with `0`: `(0, atlas_coords, 0)`. atlas_coords can map to an index with some simple arithmetic, so it's close enough to `TileIdx`. In orher words, wanderrust's status quo is like `(0, atlas_index, 0)`.

`alternative_id` adds another degree of freedom. The user can create an alternative, varying as few or as many properties of the alternative as they like, and only `alternative_id` changes, such as from 0 to 1. Tile alternative 0 might be solid whereas 1 is barely visible, and 2 just has a different color.

In wanderrust, `tiles!` defines enum variants of `TileIdx` using identifiers like `Grass` and an atlas index like `atlas_idx(1, 0)` or `1`, and a series of `const` lists impart qualities like walkable or opaque. Tiles are numbered from 0 to 1077:

```rust
/// The size of the tile sheet in grid units.
pub const SHEET_SIZE_G: UVec2 = UVec2::new(49, 22);
```

## DESIGN

For any given `usize`, `usize % NUM_TILES` will point to a valid tile. `From<usize>` is the single source of truth for what index to use, so it becomes:

```rust
impl From<TileIdx> for usize {
    fn from(value: TileIdx) -> Self {
        value.0 % NUM_TILES as usize
    }
}
```

We define a helper in addition to `atlas_idx`. Here they are side by side:

```rust
const fn atlas_idx_page(x: u32, y: u32, page: u32) -> usize {
  let page_offset = NUM_TILES * page;
  ((y * DIMENSIONS[0] + x) + page_offset) as usize
}

const fn atlas_idx(x: u32, y: u32) -> usize {
    atlas_idx_page(x, y, 0)
}
```

Only the systems that want/use extended tiles need to name them explicitly, and these tiles will translate themselves into a valid `atlas_idx` for rendering. For instance, look at this definition:

```rust
tiles! {
    Blank = atlas_idx_alt(0, 0, 0),
    Transparent = atlas_idx_alt(0, 0, 1),
}
```

- `TileIdx::Transparent` resolves to 1078 b/c `NUM_TILES`.
- `value.0 % NUM_TILES as usize` yields `0`.
- `TileIdx::Transparent` is distinct from `TileIdx::Blank` despite sharing an atlas index.

### PROTOTYPE

Here's the prototype from the rust playground. It *is* different from wanderrust. 

See also [https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=ae5da8344df5fd3bc21ff10abff80418]().

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
    
    // something like a worked example: item at coordinates (0, 13), or index 637.
    let tile_idx: TileIdx = TileIdx(atlas_idx_page(0, 13, 1));
    // this is the real public API
    let atlas_i: usize = tile_idx.into();
    // only `impl TileIdx` would have this kind of direct access
    assert_eq!(tile_idx.0, NUM_TILES as usize + 49 * 13);
    // and side by side we can see they are not equal
    assert_ne!(atlas_i, NUM_TILES as usize + 49 * 13);
    assert_eq!(atlas_i, atlas_idx(0, 13));
}
```
