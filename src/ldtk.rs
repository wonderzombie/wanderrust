use std::collections::HashMap;

use bevy::{
    log::{info, warn},
    utils::default,
};
use ldtk_json_rs::ldtk_json::{EntityInstance, FieldInstance, LDtk, LayerInstance, TileInstance};

use crate::{cell::Cell, tilemap::Dimensions, tiles::TileIdx};

#[derive(Default, Debug)]
pub struct Imported {
    pub all_tiles: HashMap<Depth, Vec<(TileIdx, Cell)>>,
    pub all_entities: HashMap<Depth, Vec<((String, Cell), Vec<FieldInstance>)>>,
    pub sizes: HashMap<Depth, Dimensions>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Depth(i64);

pub fn import_raw(ldtk: LDtk) -> Imported {
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

        // Different levels' layers can have different sizes. To avoid dealing with it,
        // collect the maximum width/height for this level and use that.
        let mut max_dim = Dimensions::default();
        for layer_inst in level_layers {
            let dim = Dimensions::from_layer_inst(&layer_inst);
            if layer_inst.grid_tiles.len() > 0 {
                let level_tiles = get_level_tiles(&dim, &tile_idx_lookup, &layer_inst.grid_tiles);
                imported.all_tiles.insert(depth, level_tiles);
            }

            if layer_inst.entity_instances.len() > 0 {
                let entities = get_entities(&layer_inst.entity_instances);
                imported.all_entities.insert(depth, entities);
            }

            max_dim.max_xy(dim.width, dim.height);
        }
        imported.sizes.insert(depth, max_dim);
    }

    imported
}

impl Dimensions {
    pub fn from_layer_inst(layer_inst: &LayerInstance) -> Self {
        Self {
            width: layer_inst.c_wid as u32,
            height: layer_inst.c_hei as u32,
            ..default()
        }
    }
}

fn get_level_tiles(
    dim: &Dimensions,
    tile_idx_lookup: &HashMap<usize, TileIdx>,
    grid_tiles: &Vec<TileInstance>,
) -> Vec<(TileIdx, Cell)> {
    let mut level_tiles: Vec<(TileIdx, Cell)> = Vec::new();
    for tile_inst in grid_tiles.iter() {
        let idx = tile_inst.t as usize;
        let tile_idx: TileIdx = tile_idx_lookup.get(&idx).copied().unwrap_or_default();
        // `TileInstance::px` is a `Vec<i64>` which should have two values.
        if tile_inst.px.len() != 2 {
            warn!(
                "expected tile_inst.px to be two elements: {:?}",
                tile_inst.px
            );
        }
        let cell: Cell = dim.pos_to_cell(tile_inst.px.as_slice());

        level_tiles.push((tile_idx, cell));
    }
    level_tiles
}

fn get_entities(entities: &Vec<EntityInstance>) -> Vec<((String, Cell), Vec<FieldInstance>)> {
    let mut out = Vec::new();
    for entity_inst in entities.iter() {
        let cell = Cell {
            x: entity_inst.grid[0] as i32,
            y: entity_inst.grid[1] as i32,
        };
        info!(
            "{} @ {} = {:#?}",
            entity_inst.identifier, cell, entity_inst.field_instances
        );

        out.push((
            (entity_inst.identifier.clone(), cell),
            entity_inst.field_instances.clone(),
        ))
    }
    out
}
