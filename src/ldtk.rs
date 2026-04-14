use bevy::{
    log::{info, warn},
    platform::collections::HashMap,
    prelude::*,
    utils::default,
};
use ldtk_json_rs::ldtk_json::{EntityInstance, FieldInstance, LDtk, LayerInstance, TileInstance};
use serde_json::Value;

use crate::{
    cell::Cell,
    tilemap::{Dimensions, StratumId, TilemapSpec},
    tiles::TileIdx,
};

#[derive(Default, Debug)]
pub struct Imported {
    pub all_tiles: HashMap<StratumId, Vec<(TileIdx, Cell)>>,
    pub all_entities: HashMap<StratumId, Vec<((String, Cell), Vec<FieldInstance>)>>,
    pub sizes: HashMap<StratumId, Dimensions>,
}

pub fn ldtk_to_wanderrust(ldtk: LDtk) -> TilemapSpec {
    let mut spec = TilemapSpec::default();
    let raw = import_raw(ldtk);

    spec.all_tiles = raw.all_tiles;

    for (stratum_id, all_instances) in raw.all_entities.iter() {
        for entity_inst in all_instances {
            info!("{}: {} {}", stratum_id.0, entity_inst.0.0, entity_inst.0.1);
        }
    }

    spec
}

pub fn import_raw(ldtk: LDtk) -> Imported {
    let mut imported = Imported::default();

    // LDtk (unlike Godot) uses either pixels for atlas or an index.
    let tile_idx_lookup = TileIdx::pairs()
        .iter()
        .copied()
        .collect::<HashMap<usize, TileIdx>>();

    // An LDtk map has one more [`Level`] which consists of two
    // [`LayerInstance`] typically: one for tiles and one for entities,
    // which in wanderrust we might call actors.
    for level in ldtk.levels.iter() {
        let Some(ref level_layers) = level.layer_instances else {
            warn!(
                "{}: level had no layer instances: {}",
                level.identifier, level.iid
            );
            continue;
        };

        let stratum_id = StratumId(level.world_depth as i32);

        // Different levels' layers can have different sizes. To avoid dealing with it,
        // collect the maximum width/height for this level and use that.
        let mut max_dim = Dimensions::default();
        for layer_inst in level_layers {
            let dim = Dimensions::from_layer_inst(&layer_inst);

            match layer_inst.layer_instance_type.to_ascii_lowercase().as_str() {
                "entities" => {
                    imported
                        .all_entities
                        .insert(stratum_id, get_entities(&layer_inst.entity_instances));
                    info!("{}: loaded entities", level.identifier);
                    get_entities_with_fields(&layer_inst.entity_instances);
                }
                "tiles" => {
                    imported.all_tiles.insert(
                        stratum_id,
                        get_level_tiles(&dim, &tile_idx_lookup, &layer_inst.grid_tiles),
                    );
                    info!("{}: loaded tiles", level.identifier);
                }
                other => {
                    warn!(
                        "{}: skipping layer_instance_type {}",
                        level.identifier, other
                    );
                }
            }

            max_dim.max_xy(dim.width, dim.height);
        }
        imported.sizes.insert(stratum_id, max_dim);
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

/// Infers [`TileIdx`] and [`Cell`] from each [`TileInstance`].
/// [`TileInstance::t`] is the tile ID, aka atlas index.
fn get_level_tiles(
    dim: &Dimensions,
    tile_idx_lookup: &HashMap<usize, TileIdx>,
    grid_tiles: &Vec<TileInstance>,
) -> Vec<(TileIdx, Cell)> {
    let mut level_tiles: Vec<(TileIdx, Cell)> = Vec::new();
    for tile_inst in grid_tiles.iter() {
        // The ID points directly to a tile index.
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

        info!("{:#?}", entity_inst);

        out.push((
            (entity_inst.identifier.clone(), cell),
            entity_inst.field_instances.clone(),
        ))
    }
    out
}

type Iid = String;
type EntityFields = HashMap<String, Option<Value>>;

#[derive(Debug, Default)]
pub struct LdtkEntity {
    pub iid: Iid,
    pub identifier: String,
    pub tile_idx_opt: Option<TileIdx>,
    pub cell: Cell,
    pub fields: EntityFields,
}

impl Cell {
    fn from_i64(x: i64, y: i64) -> Self {
        Self {
            x: x as i32,
            y: y as i32,
        }
    }
}

/// Maps each entity's [`Iid`] (instance ID) to [`LdtkEntity`].
/// Note that [`EntityInstance::tile`] is [`TilesetRect`], which is `px`.
fn get_entities_with_fields(entities: &Vec<EntityInstance>) -> HashMap<Iid, LdtkEntity> {
    let mut out = HashMap::new();
    for ref e_inst in entities.iter() {
        let mut fields = EntityFields::new();
        let cell = Cell::from_i64(e_inst.grid[0], e_inst.grid[1]);

        for e_inst_field in e_inst.field_instances.iter() {
            fields.insert(e_inst_field.identifier.clone(), e_inst_field.value.clone());
        }

        let tile_idx_opt = e_inst
            .tile
            .as_ref()
            .and_then(|it| TileIdx::from_px(it.x, it.y));
        let identifier = e_inst.identifier.clone();
        let iid = e_inst.iid.clone();

        out.insert(
            iid.clone(),
            LdtkEntity {
                iid,
                identifier,
                tile_idx_opt,
                cell,
                fields,
            },
        );
    }

    info!(
        "=== loaded entities start ===\n{:#?}\n=== loaded entities end ===",
        out
    );

    out
}
