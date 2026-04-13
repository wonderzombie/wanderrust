use std::collections::HashMap;

use bevy::log::warn;
use ldtk_json_rs::ldtk_json::LDtk;

use crate::{cell::Cell, tilemap::Dimensions, tiles::TileIdx};

#[derive(Default, Debug)]
pub struct Imported {
    pub all_tiles: HashMap<Depth, Vec<(TileIdx, Cell)>>,
    pub size: Dimensions,
}

#[derive(Default, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Depth(i64);

pub fn import(ldtk: LDtk) -> Imported {
    let mut imported = Imported::default();

    let tile_idx_lookup = TileIdx::pairs()
        .iter()
        .copied()
        .collect::<HashMap<usize, TileIdx>>();

    for level in ldtk.levels.iter() {
        let Some(ref level_layers) = level.layer_instances else {
            continue;
        };

        let depth = Depth(level.world_depth);
        let level_tiles: &mut Vec<(TileIdx, Cell)> = imported.all_tiles.entry(depth).or_default();

        for layer_inst in level_layers {
            let dim = Dimensions {
                width: layer_inst.c_wid as u32,
                height: layer_inst.c_hei as u32,
                tile_size: 16,
            };
            for tile_inst in layer_inst.grid_tiles.iter() {
                let idx = tile_inst.t as usize;
                let tile_idx: TileIdx = tile_idx_lookup.get(&idx).copied().unwrap_or_default();
                // `TileInstance::px` is a `Vec<i64>`.
                if tile_inst.px.len() != 2 {
                    warn!(
                        "expected tile_inst.px to be two elements: {:?}",
                        tile_inst.px
                    );
                }
                let cell: Cell = dim.pos_to_cell(tile_inst.px.as_slice());

                level_tiles.push((tile_idx, cell));
            }
        }
    }

    imported
}
