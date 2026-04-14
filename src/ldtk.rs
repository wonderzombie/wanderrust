use bevy::{
    log::{info, warn},
    platform::collections::HashMap,
    prelude::*,
    utils::default,
};
use ldtk_json_rs::ldtk_json::{EntityInstance, LDtk, LayerInstance, TileInstance};
use serde_json::Value;

use crate::{
    cell::Cell,
    tilemap::{Dimensions, StratumId, TilemapSpec},
    tiles::TileIdx,
};

#[derive(Default, Debug)]
pub struct Imported {
    pub levels: HashMap<StratumId, LevelItems>,
}

#[derive(Default, Debug)]
pub struct LevelItems {
    // pub id: StratumId,
    pub tiles: Vec<(TileIdx, Cell)>,
    pub entities: HashMap<Iid, LdtkEntity>,
    pub size: Dimensions,
}

pub fn ldtk_to_wanderrust(ldtk: LDtk) -> TilemapSpec {
    let mut spec = TilemapSpec::default();
    let raw = import_raw(ldtk);

    for (stratum_id, items) in raw.levels.iter() {}

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

        info!("{}: level at {}", level.identifier, level.world_depth);

        let stratum_id = StratumId(level.world_depth as i32);
        let mut level_items = LevelItems::default();
        // Different levels' layers can have different sizes. To avoid dealing with it,
        // collect the maximum width/height for this level's layers.
        let mut max_dim = Dimensions::default();

        for layer_inst in level_layers {
            let dim = Dimensions::from_layer_inst(&layer_inst);
            info!(
                "{}: {}: layer size: {}",
                level.identifier, layer_inst.identifier, dim
            );

            match layer_inst.layer_instance_type.to_ascii_lowercase().as_str() {
                "entities" => {
                    level_items.entities = get_entities_with_fields(&layer_inst.entity_instances);
                    info!("{}: loaded entities", level.identifier);
                }
                "tiles" => {
                    level_items.tiles = get_tiles(&dim, &tile_idx_lookup, &layer_inst.grid_tiles);
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

        level_items.size = max_dim;
        imported.levels.insert(stratum_id, level_items);
    }

    info!("=== {} ===\n{:#?}", ldtk.iid, imported);

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
fn get_tiles(
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

type Iid = String;
type EntityFields = HashMap<String, Option<Value>>;

#[derive(Debug, Default)]
pub struct LdtkEntity {
    pub iid: Iid,
    pub identifier: String,
    pub tile_idx_opt: Option<TileIdx>,
    pub cell: Cell,
    pub fields: EntityFields,
    pub tags: Vec<String>,
}

impl LdtkEntity {
    const INTERACTABLE: &'static str = "interactable";

    pub fn into_bundle(&self) -> Option<impl Bundle> {
        let tile_idx = self.tile_idx_opt.unwrap_or_default();
        Some((Name::new(self.identifier.clone()), tile_idx, self.cell))
    }
}

impl Cell {
    fn from_i64(x: i64, y: i64) -> Self {
        Self {
            x: x as i32,
            y: y as i32,
        }
    }
}

/// Maps each entity's [`Iid`] (instance ID) to [`LdtkEntity`]. Note that
/// [`EntityInstance::tile`] is [`ldtk_json_rs::ldtk_json::TilesetRectangle`], aka pixels.
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
        let tags = e_inst.tags.clone();

        out.insert(
            iid.clone(),
            LdtkEntity {
                iid,
                identifier,
                tile_idx_opt,
                cell,
                fields,
                tags,
            },
        );
    }
    out
}
