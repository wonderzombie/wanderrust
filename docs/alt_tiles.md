# ALTERNATIVE TILES

**nsprites = 1078**

The proposal is real simple and we can keep it confined to `tiles.rs`. Only the systems that want/use extended tiles need to name them explicitly, and these tiles will translate themselves into the usual `atlas_index` — the difference is primarily that we might have something like

`Transparent = atlas_idx_alt(0, 0, 1)`

- That resolves to 1078 since it's one more than the last tile. 
- `TileIdx`'s `From` implementation will yield the correct `usize` to match the sprite sheet using `%`, et al.
- Systems that care about `TileIdx::Transparent` don't care that it's actually `atlas_index = 0`.

And I have a prototype for this lying around somewhere; it's very simple but I had yet to document this.

## impl notes

```rust
pub const fn atlas_idx(x: u32, y: u32) -> usize {
    (y * SHEET_SIZE_G.x + x) as usize
}
```

This is the current implementation, where `atlas_idx(1, 2)` is equivalent to `atlas_idx_alt(1, 2, 0)`.

Here's the prototype from the rust playground.

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
    assert_eq!(tile_idx.0, NUM_TILES as usize + 49 * 13);
    let atlas_i: usize = tile_idx.into();
    assert_ne!(atlas_i, NUM_TILES as usize + 49 * 13);
    assert_eq!(atlas_i, atlas_idx(0, 13));
    
}
```
