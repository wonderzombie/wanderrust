# PROBABLY NEXT

## REVISIT PARENTING ENTITIES. 

If we have all entities as a child of the map entity, the initial part becomes way easier. IF we're Below, we hide Above. If we're Above, we show Below but Dim or Night. 

-- In the end I have both. For Visibility, I use the parent/child relationship. For differentiating which tiles belong to which stratum, we have the `Stratum` enum.

## LANTERN ABILITY

Maybe you can *throw "flares."* We could even *treat the flare like it has real physics.*

## TRAVERSAL

I think traversal is important – it can be more or less spectacular, like the ladder versus the boat, as in Zelda. Animal Crossing has some related tools. 

- Horses
- Boats
- Magic tool
- "hookshot"
- whip
- ender pearl (i.e. throw it and tp to landing tile)

## HAZARDS LIKE EXPLODING BARRELS

For wanderrust we can just put a number over it? But a floating damage number. The item itself may have a color change so you're not hosed, but if you are paying attention the number will help you more. 

Think also of how Divinity II does this sort of thing. They have text that floats up. There's also an icon probably but we don't need to go there just yet. *Or* we go ahead
and look at those tiny status icons we made in Piskel. 

## COMBAT LIKE IN ENSHROUDED

We have enemies with a *block meter*. Break it and you can do merciless attack. This is to say it is a critical. The block meter can be a very small N, like 6 high, 8 - 10 is a boss? 

We have *backstab damage* when enemies are *flanked*. (Flanking damage? Maybe it's something dangerous like +1 damage for every ally for some creatures.)

You have effective/ineffective. 

You have *floating damage numbers* alongside bars.

## HIDDEN STAMINA

Hidden stat: Stamina. You get a color. Or a short meter. Or a really long meter. 

You might also get a prompt when you get past a certain point. If the actions are arranged in a table, they would be unadorned: lt attack | hvy attack | dodge | block. 

The notions are that we could color them, we could add punctuation to them, something. 

Measuring is not the point, so we must remove the temptation. The test version might be a swatch with a color gradient from green to yellow to red, or just green to red. 

Like if you're about to spend your last bit of stamina, instead of `hvy attack` it's `[hvy attack]`, or vice versa. Or have brackets all the time but color them. Whatever.

The math should be pretty simple. I am not against using a low N like 5 or even 3 for a starting character's stamina. Let's try 5. 

You can queue actions. The number of queued actions depends on Acumen. Alternatively, maybe Acumen gives you half of what you spend, rounded down, when you cancel a move. Alacrity makes you go sooner. Grit gives you more stamina.



## TILEIDX: ALTERNATIVE TILES

We don't need an arbitrary number of alternative tiles; just one per tile should be good enough. THe goal is for alternative tiles to retain the appearance of the original without retaining the properties of the original — walkable, opaque, etc. 

One answer is just `alt_atlas_idx`. We would use this to define enums that use the same atlas coordinates without using the same number. The easiest trick is to keep counting: 49 x 22 = 1078. If 0 is the first tile, 1078 is also the first tile. 

The tile itself is exactly like any other tile. Add it to WALKABLE or OPAQUE or whatever.

## TILEIDX: FLIP

A Component would work well: `pub struct TileTransform { flip_v: bool, flip_h: bool, flip_d: bool, }` then Query `Option<&TileTransform>` alongside `TileIdx`.

# DESIGNS



====
If needed:

somecave.map.ron <--- lots and lots of tiles
somecave.data.ron <--- human editable; small-ish

This would mean serializing tiles separately from the others. 



# MAYBE NEXT

- Inventory becomes a Component alongside Interactable::Chest 

- Finish implementing attacks w/ Damage -- done

- Dead mobs get a marker struct and a system handles them
- When you kill the mob, you get the loot; that's all — could even write Acquisition

- The inventory list is along the right-hand side
- It shows you what you have equipped, your HP, and inventory item names
- When you press Shift-I, an inventory UI appears; it's not interactive yet

- Equipment could also be a component?

- Stamina is a non-numeric stat. You get some back each combat tick.

- XP lets you raise your stats. We are going to use the same three: Alacrity, Acumen, and Grit. 
- To raise a stat, spend XP equal to the new value. e.g. 2 -> 3 costs 3 XP.

- Consider implementing something like the Shroud. It's not damage over time; it's a countdown. 

- How about an interface for populating a Chest that is a text box. Items are formatted like this (proposal): `gold:10 sword torch:3 mail`. 
- A key is a separate box; leave empty for no key

- start with something like hold tab to highlight and click on interesting tiles.

## APPENDIX: OLD: STRATA: TWO LAYERS? OR TWO MAPS?

- A stratum is any number of layers, but for now 1:1.
- Stick to two strata. 
- Special tile marker: `NoneTile` should get `Hidden`.


- First most dumbest way is to put two maps next to one another. 
- How will they not conflict? They will conflict for sure. 
  - We don't use `tiles` to drive every tile operation, so every system would work on every map at once.
  - `MapTile` is undifferentiated across tilemaps

- Newtypes are one way to handle it, like `struct LayerOne` that implements `Component`, something like `LayerOne` and `LayerTwo` to start with. We put a trait `Layer` on this type — could this help? `<L: Layer>`

- TileStorage def needs to be updated with this. 

- Yeah, let's just start with two layers. Above and Below. 
- ZoneSpec? Has two TilemapSpec? 

- Add `middle` ring to emitter


## APPENDIX: WANDERL2R

### Sprites in wanderlust (Godot) and wanderrust (Bevy)

`tile_replacer.ruins.json` has a mapping from a Godot-friendly representation of a tile in a spritesheet to a list of cells using that particular tile.

In other words, within a map like `Ruins/GroundFloor/_level`, we have a list of objects with
three fields: `alternative_id`, `atlas_coords`, and `source_id`. In Godot these uniquely identify a sprite in a particular sheet.

wanderrust uses Bevy which uses indices. The Godot identifier could be said to look like `(source_id, atlas_coordinates, alternative_id)`:

```
(0, (48, 21), 0)
```

identifies it as belonging to the first "source" (e.g. image), column 48 row 21, and the 0th (first) alternative ID, so it's whatever the TileSet's "plain" means. Maybe it's opaque and solid, and we want another version that's opaque but NOT solid, for secrets™. 

Well, since the sprite sheet has a fixed width no matter what (49x22), using an index is not hard:

```
    /// Converts this cell to an index given a width, treating the cell as a 2D grid index.
    pub fn to_idx(self, width: u32) -> usize {
        width
            .saturating_mul(self.y as u32)
            .saturating_add(self.x as u32) as usize
    }
```

### Maps exported from wanderlust

The format for a map is something like this, in pseudo-code:

```
{

  "ZoneName/SomeStratum/_level": [
    {
      "alternative_id": 0,
      "atlas_coords": [16, 0],
      "source_id": 1,
      "cells": [
        [18, 5],
        [22, 10],
        // ...
      ]
    }
  ],
  "ZoneName/OtherStratum/_level": [
    // as above
  ]
}
```

In `wanderl2r` we transpose (?) it into `HashMap<Cell, TileIdx>`. 

### Maps imported from wanderlust

*wanderrust maps are square.* Instead of using a map from cells to tiles, we map `Cell` to/from an index in a `Vec<(TileIdx, Stratum)>`. *The width having a fixed size for any given `y` is critical.*

*wanderlust maps are not square.* In the Godot APIs, we use `get_used_cells()` to export all cells with a tile. In other words, if we tried `tml.get_cell_atlas_coords(cell)` on an empty cell, we'd get `(-1, -1)`. This requires no particular arrangemnt of a map, square or otherwise.

*wanderrust maps have `[0, 0]` as the bottom left.* Negative coordinates are not allowed. 

*wanderlust maps have `[0, 0]` as the upper left*. Negative coordinates are allowed. 

### What to do

Well, this is why I am here. My prototype worked fine. Now I am nailing down something that is not a hack, and I've noticed that simply faking a larger map didn't work the way I did it.

#### example

If we have a list of cells, we can take the upper left bound and the lower right bound and use each to describe a rectangular map. When we treat this as the size, we encompass every tile. 

Straw man: use `HashMap<Cell, (TileIdx, Stratum)>` as the starting point. If we only used these, we would not be able to use an index since rows may be non-contiguous. Trivially, if we have just one tile `[2, 2]` and the next `y` is populated from `[0, 2]` to `[64, 2]`, and the one after that is `[0, 3]` to `[60, 3]`, we have an irregularity in the number of columns. 

### normalizing to a square

My first instinct was to stick with `Vec<(TileIdx, Stratum)>`. It won't work for wanderlust maps as-is, so we try a little fudging. Taking the upper left and bottom right gives us coordinates for a rectangle encompassing all cells with a tile *and* cells without a tile.

We've started with `fill_map`: produce a datum which combines cells with tiles and cells without tiles, ostensibly using `Vec<T>` so we can map freely between indices and coordinates.

```
[+] LEVEL: SmugglersCave/TheCave/_level
SmugglersCave/TheCave/_level: offset: Cell(-10,-11)
SmugglersCave/TheCave/_level: bottom_right: Cell(54,43)
SmugglersCave/TheCave/_level: effective map size: Cell(64,54)
SmugglersCave/TheCave/_level: cells / total = 1904 / 3456
```

*As long as the width is regular, the height doesn't matter* for the purposes of index calculations. For the purpose of populating the map, though, we need to know when to stop adding rows (`y`).

So we iterate essentially from offset (upper left) to lower right. Any coordinates that aren't in `transposed_map` receive a `TileIdx::default()` which presently is `TileIdx::default()`.

### negative coordinates

We had to deal with this before since `MRPAS` does not allow negative coordinates. The key insight is that if we want to ensure any map *starts* at `[0, 0]`, we treat the upper left as an offset. 

If the upper left is `[-10, -11]` (wanderlust), we *subtract offset from a cell* to obtain its position in a `[0, 0]`-based map (wanderrust). The `transposed_map` has cells verbatim. We want to use `wanderlust` map coordinates — *no offset* - to read the "old," and contrariwise `wanderrust` tiles *need the offset* to write the "new."

### putting it together

1. transpose map JSON to map from `Cell` to `TileIdx`.
2. for all JSON-provided cells: measure upper left using min(x) and min(y)
3. for all JSON-provided cells: measure bottom right using max(x) and max(y) 
4. generate a rect that will include every single cell in the original data *and* potentially blank tiles
5.  `offset.y..=bottom_right.y`, et al, to iterate through every cell in the map
6. use cell as lookup into `transposed_map` and default to `TileIdx::default()`
7. `cell - offset` to map coordinates like `[-10, -11]` to `[0, 0]`
8. insert offset cell into the HashMap

That last point is maybe not my favorite.

### loading the map

This is the other half of the equation. We're trying to load a `(64, 54)` map into a `(100, 100)` grid. The way each of these map to a `Vec<T>` is going to be different. 

#### example

Translating `[60, 2]` to an index depends on the width — `100x100` vs `64x54` yields a different index. 100x100 is a sequence of numbers from 0 to 10000, and 64x54 is 3456. 

Put more simply: 

- a 5x5 map has 25 tiles. `[0, 1]` maps to index `5`, the 6th tile in the list. `[4, 4]` maps to index `24`. 
- a 10x10 map has 100 tiles. `[0, 1]` maps to index `10`, the 11th tile in the list. `[4, 4]` maps to index `44`. 
- naively: writing a 5x5 sequence into a 10x10 map means the first two rows in the 5x5 will have the same `y`.

#### the answer (example cont'd)

When *reading* from the 5x5, we keep the cell coordinates; they are absolute positioning because `[0, 0]` and `[4, 4]` are the same for any grid large enough.

When *writing* to the 10x10, we need to map the 5x5 coordinates to indices using `10` as the width.

*SavedTilemap doesn't need to change as long as it's internally consistent.*

### summarized

- `fill_map` takes the incoming map's size and generates a vector that holds all possible cells _by index_. 
- It doesn't matter whether the map is `[0, 0]` or not; we use relative positioning based on `size`.


### living upside-down

Let's say we have a 10x10 and we want to load a 5x5 that's upside-down.

- [0, 0] means the 0th cell. 
- (0, 0) means screen position. 

- 10x10 has [0, 0] at the bottom left. Let's say that's position (0, 0).
- 5x5 has [0, 0] at the upper left. Let's say that's position (80, 80).

We want the vertical axis to be reversed:

```
let bevy_y = (map_height - 1) - tiled_y;
```


#### scratchpad

In both systems, counting up in cells is counting up in pixels. Obvs.

When we measure the source map, we will get a measurement in cells. For our 5x5, what we want is something like this: 

```
[0, 0] => (0, 0)
[1, 0] => (16, 0)
// ...
[3, 3] => (48, 48)
[4, 4] => (64, 64)
```

Let's start with tracking size via "the last cell" ignoring negatives. The last cell in 5x5 is [4, 4], so for now `let size_y = 4`. 

```
[0, 0] => [0, size_y - 0] => (0, 64)
[1, 1] => [1, size_y - 1] => (16, 48)
[2, 1] => [2, size_y - 1] => (32, 48)
[1, 3] => [1, size_y - 3] => (16, 16)
[3, 3] => [3, size_y - 3] => (48, 16)
[4, 4] => [4, size_y - 4] => (64, 0)
```
