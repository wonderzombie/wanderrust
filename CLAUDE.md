# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**wanderrust** is a tile-based, fake-retro 2D game built with [Bevy](https://bevyengine.org/) (v0.18) and `bevy_egui`. While some aspects of it are procedural, the world uses a fixed seed by default when choosing terrain tiles. It uses a tile-based map rendered from a sprite atlas (Kenney 1-bit pack), with field-of-view, procedural map generation, an inventory system, and an in-game map editor with save/load via the `rfd` file dialog.

The workspace has two members:
- `.` — the main game crate (`wanderrust`)
- `mrpas/` — a local FOV library (port of [godot-mrpas](https://github.com/matt-kimball/godot-mrpas))

## Commands

```bash
# Build
cargo build

# Run the game
cargo run

# Run tests (unit tests in procgen and ptable modules)
cargo test

# Run tests for a specific module
cargo test --lib procgen
cargo test --lib ptable

# Check without building
cargo check

# Lint
cargo clippy
```

## Architecture

### ECS Structure (Bevy)

The game is built around Bevy's ECS. Key resources and their roles:

- **`SpriteAtlas`** — wraps the texture handle + atlas layout for the Kenney tileset; used everywhere sprites are created
- **`MapSpec`** — defines the map's probability table, tile selection function, size, and start position; used during tilemap generation
- **`SpatialIndex`** — `HashMap<Cell, Entity>` tracking which cells are blocked by non-walkable entities; rebuilt every `PostUpdate`
- **`Inventory`** (player) — player's item store, a `Resource`
- **`EditorState`** — tracks editor mode, selected tile, pending load/save dialog tasks
- **`MessageLog`** — a fixed-length ring of colored text messages rendered by egui

### Module Responsibilities

| Module | Purpose |
|---|---|
| `main.rs` | App setup, `Player`, `Actor`, `SpatialIndex`, `PieceBundle`, `Interactable`, input handling, camera |
| `map.rs` | `MapSpec` (map config resource), `sync_tiles` (spawns new tile entities from spec), `update_tile_visuals` (tint/alpha for FOV) |
| `tilemap.rs` | `TileStorage`, `MapDimensions`, `SavedTilemap`; spawns map tiles, serializes/deserializes maps to/from RON |
| `tiles.rs` | `TileIdx` enum (atlas indices for every tile type), marker components (`MapTile`, `Walkable`, `Opaque`), and bool-carrying components (`Revealed(bool)`, `Highlighted(bool)`) |
| `procgen.rs` | Procedural generation via bilinear noise sampling and `ProbabilityTable`; `biome_ptable()` and `tile_idx_for_cell()` are the key entry points |
| `ptable.rs` | `ProbabilityTable` and `TableBuilder` — weighted random tile selection |
| `fov.rs` | `Fov`/`View` resources wrapping `mrpas::Mrpas`; `update_fov_model` sets transparency, `update_fov_markers` updates `Revealed` on all tiles each frame |
| `editor.rs` | `EditorPlugin` — tile picker UI (egui), zoom, click-to-paint tiles, async save/load dialogs via `rfd` |
| `cell.rs` | `Cell` — a simple `(x, y)` grid coordinate component |
| `colors.rs` | Named color constants (Kenney palette) |
| `inventory.rs` | `Item`, `Inventory`, `Acquisition` message |
| `event_log.rs` | `MessageLog` resource, egui rendering of messages, font setup |
| `player.rs` | `PlayerStats` resource |

### Key Patterns

**Cell coordinates vs. world space**: Map tiles live at `Cell(x, y)`. World transform is `x * TILE_SIZE_PX, y * TILE_SIZE_PX`. The camera follows the player cell.

**Message passing**: `ActionAttempt` and `Acquisition` are Bevy messages (via `.add_message::<T>()`), not events. Systems use `MessageReader`/`MessageWriter`.

**Tile visual state**: FOV visibility is applied in `map::update_tile_visuals` (runs in `Last`) by reading `Revealed(bool)` and `Highlighted(bool)` components. All map tiles carry `Revealed` (spawned as `Revealed(false)`); `update_fov_markers` sets its value each frame. `Revealed(false)` tiles are hidden, `Revealed(true)` tiles are visible, and `Highlighted(true)` tiles (editor hover) are full-brightness gold. The bool-in-component approach is intentional: mutating a bool avoids per-frame archetype changes that `insert`/`remove` would cause. `Highlighted` is only present on editor-mode tiles and is toggled by `Pointer<Over>`/`Pointer<Out>` observers in `editor.rs`.

**Map serialization**: Maps are saved/loaded as RON files via `tilemap::SavedTilemap`, which stores a flat `Vec<TileIdx>` with dimensions. The `data/` directory contains example saved maps.

**Async dialogs**: `editor.rs` uses `rfd` with Bevy tasks (`AsyncComputeTaskPool`) for non-blocking file picker dialogs. Tasks are polled each frame via `poll_load_dialog` / `poll_save_dialog`.

### `mrpas` crate

The local `mrpas/` crate provides `Mrpas` — a grid-based symmetric shadowcasting FOV algorithm. Usage: call `set_transparent(pos, bool)` to mark occluders, `clear_field_of_view()`, then `compute_field_of_view(origin, range)`. Check visibility with `is_in_view(pos)`.
