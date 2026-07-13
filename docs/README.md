# wanderrust

Design docs and working notes for **wanderrust**, a roguelike built in Rust
with Bevy. Successor to `wanderlust`, which was built in Godot.

These are working documents, not specifications. They're written to think
with, and most of them include the dead ends and the reasoning that got
discarded — that's on purpose. The conclusion is usually near the top; the
argument is usually below it.

## Status at a glance

| Doc | Status | About |
|---|---|---|
| [Strata](strata.md) | done | Maps divided into vertical layers; visibility and hierarchy |
| [Strata: lightmaps](strata_lightmap.md) | done | Per-emitter and per-stratum light maps, merged and diffed |
| [wanderl2r](wanderl2r.md) | done | Converting Godot `TileMapLayer` maps into wanderrust RON |
| [Levers and gates](levers_and_gates.md) | building | Doors opened by a specific lever |
| [Active stratum](active_stratum.md) | proposed | `ActiveStratum` resource; `StandingOn` / `StoodOn` |
| [Load next map](load_next_map.md) | proposed | Tearing down and rebuilding strata to move between areas |
| [Level-wise spawning](levelwise_spawning.md) | proposed | Multiple LDtk levels at one world depth |
| [LDtk as pipeline](ldtk_as_pipeline.md) | proposed | Using LDtk project entities as the item catalog |
| [Alternative tiles](alt_tiles.md) | proposed | "Pages" of tiles: same sprite, different properties |
| [Big tiles](big_tiles.md) | proposed | 2x2 tiles for giant/ancient spaces |
| [Tile flipping](flip_tiles.md) | proposed | `TileTransform` for horizontal/vertical/diagonal flips |
| [Mini health bars](mini_health_bars.md) | proposed | Small HP indicator near the tooltip |
| [Items as entities](items_as_entities.md) | proposed | Item catalog, kinds, and equipment slots |
| [Effects](effects.md) | draft | NetHack-style intrinsic/extrinsic properties |
| [Notes on some patterns](notes_on_some_patterns.md) | reference | Bevy/ECS patterns used throughout |

<!--
Statuses are my best guess from reading the docs — fix any I got wrong.
The vocabulary is: draft | proposed | building | done | superseded | abandoned

This table is hand-maintained, which is fine at this size. If it ever
stops being fine, that's the signal to move to a generator that reads
frontmatter (Zola, Astro) instead of mdBook.
-->

## Writing a new doc

- Design docs: copy `_template.md`
- Pattern notes: copy `_template_pattern.md`
- Add the file to `SUMMARY.md` or mdBook won't render it
