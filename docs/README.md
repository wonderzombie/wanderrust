# wanderrust

Design docs and working notes for **wanderrust**, a soulslike-roguelike built in
Rust with Bevy. Successor to `wanderlust`, which was built in Godot.

These are working documents, not specifications. They're written to think
with, and most of them include the dead ends and the reasoning that got
discarded — that's on purpose. The conclusion is usually near the top; the
argument is usually below it.

## Status at a glance

| Doc | Status | About |
|---|---|---|
| [Strata](strata.md) | done | Maps divided into vertical layers; visibility and hierarchy |
| [Strata: lightmaps](strata_lightmap.md) | done | Per-emitter and per-stratum light maps, merged and diffed |
| [wanderl2r](wanderl2r.md) | superseded | Converting Godot `TileMapLayer` maps into wanderrust RON |
| [You died screen](you_died_screen.md) | building | Players respawn after death |
| [Levers and gates](levers_and_gates.md) | proposed | Doors opened by a specific lever |
| [Active stratum](active_stratum.md) | superseded | `ActiveStratum` resource; `StandingOn` / `StoodOn` |
| [Level-wise spawning](levelwise_spawning.md) | proposed | Multiple LDtk levels at one world depth |
| [LDtk as pipeline](ldtk_as_pipeline.md) | draft | Using LDtk project entities as the item catalog |
| [Alternative tiles](alt_tiles.md) | building | "Pages" of tiles: same sprite, different properties |
| [Big tiles](big_tiles.md) | draft | 2x2 tiles for giant/ancient spaces |
| [Tile flipping](flip_tiles.md) | building | `TileTransform` for horizontal/vertical/diagonal flips |
| [Mini health bars](mini_health_bars.md) | draft | Small HP indicator near the tooltip |
| [Items as entities](items_as_entities.md) | building | Item catalog, kinds, and equipment slots |
| [Bevy UI Message Log](bevy_ui_message_log.md) | building | Replace egui gameplay log with Bevy UI |
| [Effects](effects.md) | done | NetHack-style intrinsic/extrinsic properties |
| [Notes on some patterns](notes_on_some_patterns.md) | reference | Bevy/ECS patterns used throughout |
| [Respawning monsters](respawning_monsters.md) | proposed | Resetting monster status |
| [Respawning](respawning.md) | abandoned | Respawning monsters; shrines |
| [You Died screen](you_died_screen.md) | building | Defeat interstitial and reset scenario |
| [Equipment Redux](equipment_redux.md) | draft | Add mini-UIs and improve APIs for items & equipment |

<!--
The vocabulary is: draft | proposed | building | done | superseded | abandoned

This table is hand-maintained, which is fine at this size. If it ever
stops being fine, that's the signal to move to a generator that reads
frontmatter (Zola, Astro) instead of mdBook.
-->

## Writing a new doc

- Design docs: copy `_template.md`
- Pattern notes: copy `_template_pattern.md`
- Add the file to `SUMMARY.md` or mdBook won't render it
