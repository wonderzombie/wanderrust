use std::path::PathBuf;

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    cell::Cell,
    gamestate::GameState,
    light::LightLevel,
    tilemap::{Dimensions, StratumId, TileCell, TilemapSpec},
    tiles::{self, TileIdx},
};

pub trait FieldMapExt {
    fn get_bool(&self, key: &str) -> bool;
    fn get_string(&self, key: &str) -> Option<String>;
    fn get_str_array(&self, key: &str) -> Option<Vec<String>>;
}

impl FieldMapExt for HashMap<String, Value> {
    fn get_bool(&self, key: &str) -> bool {
        self.get(key).and_then(|v| v.as_bool()).unwrap_or_default()
    }

    fn get_string(&self, key: &str) -> Option<String> {
        self.get(key).and_then(|v| v.as_str()).map(String::from)
    }

    fn get_str_array(&self, key: &str) -> Option<Vec<String>> {
        self.get(key).and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
    }
}

pub trait LdtkEntityExt<T> {
    fn from_ldtk(value: &LdtkEntity) -> Option<T>;
}

#[derive(Debug, Deserialize, Resource)]
pub struct LdtkProject {
    pub levels: Vec<LdtkLevel>,
}

#[derive(Debug, Deserialize)]
pub struct LdtkLevel {
    pub identifier: String,
    pub iid: String,
    #[serde(rename = "fieldInstances", default)]
    pub field_instances: Vec<LdtkField>,
    #[serde(rename = "layerInstances", default)]
    pub layer_instances: Vec<LdtkLayer>,
    #[serde(rename = "pxWid")]
    pub px_width: f32,
    #[serde(rename = "pxHei")]
    pub px_height: f32,
}

#[derive(Debug, Deserialize)]
pub struct LdtkLayer {
    #[serde(rename = "__identifier")]
    pub identifier: String,
    pub iid: String,
    #[serde(rename = "__type")]
    pub layer_type: String,
    #[serde(rename = "gridTiles", default)]
    pub grid_tiles: Vec<LdtkGridTile>,
    #[serde(rename = "entityInstances", default)]
    pub entities: Vec<LdtkEntity>,
    #[serde(rename = "__cWid")]
    pub c_width: i32,
    #[serde(rename = "__cHei")]
    pub c_height: i32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LdtkField {
    #[serde(rename = "__identifier")]
    pub identifier: String,
    #[serde(rename = "__type")]
    pub field_type: String,
    #[serde(rename = "__value")]
    pub val: Value,
}

#[derive(Debug, Deserialize, Default)]
pub struct LdtkEntity {
    #[serde(rename = "__identifier")]
    pub identifier: String,
    pub iid: String,
    #[serde(rename = "__grid")]
    pub cell: LdtkCell,
    #[serde(rename = "__tile", default)]
    pub tile: LdtkPxTile,
    #[serde(rename = "fieldInstances", default)]
    pub field_instances: Vec<LdtkField>,
    #[serde(rename = "__tags")]
    pub tags: Vec<String>,
}

const LDTK_ENTITES_ENUM: &str = "Actor";

impl LdtkEntity {
    pub fn field_map(&self) -> impl FieldMapExt {
        self.field_instances
            .clone()
            .into_iter()
            .map(|it| (it.identifier, it.val))
            .collect::<HashMap<String, Value>>()
    }

    pub fn ty(&self) -> Option<LdtkActor> {
        self.field_instances
            .iter()
            .find(|f| f.identifier == LDTK_ENTITES_ENUM)
            .and_then(|v| v.val.as_str())
            .and_then(LdtkActor::from_str)
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct LdtkGridTile {
    #[serde(rename = "t")]
    pub atlas_idx: usize,
    #[serde(rename = "a")]
    pub alpha: f64,
    #[serde(rename = "f")]
    pub flip_bits: i32,
    #[serde(rename = "px")]
    pub px: Vec2,
}

#[derive(Debug, Deserialize, Default)]
pub struct LdtkPxTile {
    #[serde(rename = "x")]
    atlas_x_px: i32,
    #[serde(rename = "y")]
    atlas_y_px: i32,
}

impl From<LdtkPxTile> for Cell {
    fn from(value: LdtkPxTile) -> Self {
        Cell::new(value.atlas_x_px, value.atlas_y_px)
    }
}

#[derive(Deref, Debug, Deserialize, Default, Clone, Copy)]
pub struct LdtkCell(Cell);

pub fn load_and_import(fname: PathBuf) -> Result<LdtkProject, BevyError> {
    let serialized = std::fs::read_to_string(fname)?;
    let project = serde_json::from_str::<LdtkProject>(&serialized)?;
    info!("project levels count: {}", project.levels.len());
    Ok(project)
}

fn px_to_cell(x: f32, y: f32, level_height_px: f32) -> Cell {
    let cx = (x / 16.0) as i32;
    let cy = ((level_height_px - y) / 16.0) as i32 - 1;
    Cell::new(cx, cy)
}

fn ldtk_cell_to_wanderrust(cell: LdtkCell, level_height_cells: i32) -> Cell {
    Cell::new(cell.x, level_height_cells - 1 - cell.y)
}

pub fn generate_ldtk_tilemap(
    mut commands: Commands,
    res: Option<Res<LdtkProject>>,
    // HACKHACK for testing
    mut ns: ResMut<NextState<GameState>>,
) {
    let Some(project) = res else {
        return;
    };
    let project = project.as_ref();
    let lookup = TileIdx::reverse_lookup();

    let mut distinct_tiles = HashSet::<TileIdx>::new();
    let mut new_tiles: Vec<TileCell> = Vec::new();
    let mut distinct_entities = HashSet::<(String, Cell)>::new();

    let level = project.levels.get(0).unwrap();
    let mut spawn: Option<Cell> = None;

    // HACKHACK for testing
    // for level in &project.levels {
    //     info!("loading level {}", level.identifier);

    let mut c_wid = 1;
    let mut c_hei = 1;

    for layer in &level.layer_instances {
        c_wid = c_wid.max(layer.c_width);
        c_hei = c_hei.max(layer.c_height);

        if layer.layer_type.to_ascii_lowercase().eq("tiles") {
            for tile in &layer.grid_tiles {
                let tile_idx = lookup.get(&tile.atlas_idx).copied().unwrap_or_default();
                distinct_tiles.insert(tile_idx);
                let cell = px_to_cell(tile.px.x, tile.px.y, level.px_height);
                println!(
                    "px {} {} grid {} {} tile {}",
                    tile.px.x, tile.px.y, cell.x, cell.y, tile_idx,
                );
                new_tiles.push((tile_idx, cell));
            }
        }

        if layer.layer_type.to_ascii_lowercase().eq("entities") {
            for actor in &layer.entities {
                let cell = ldtk_cell_to_wanderrust(actor.cell, layer.c_height);
                distinct_entities.insert((actor.identifier.clone(), cell));

                if actor.identifier.eq_ignore_ascii_case("Worldspawn") {
                    spawn = Some(cell);
                }
            }
        }
    }
    println!("new tiles: {:?}", &new_tiles);

    // HACKHACK for testing
    let mut spec = TilemapSpec::default();
    spec.all_tiles.insert(StratumId(0), new_tiles);
    spec.light_level = LightLevel::Bright;
    spec.size = Dimensions {
        width: c_wid as u32,
        height: c_hei as u32,
        tile_size: tiles::TILE_SIZE_PX as u32,
    };

    spec.spawn_point = if let Some(spawn) = spawn {
        spawn
    } else {
        warn!("did not find spawn point");
        Cell::default()
    };

    info!("spawning at {}", spec.spawn_point);

    commands.insert_resource(spec);

    ns.set(GameState::Loading);
}

macro_rules! enum_with_str {
    ( $enum_name:ident, [ $( $variant:ident ),* ] ) => {
        #[derive(Default, Debug, Eq, PartialEq, Copy, Clone, Hash, Reflect)]
        pub enum $enum_name {
            #[default]
            Unset,
            $( $variant, )*
        }

        impl $enum_name {
            pub fn all() -> &'static [$enum_name] {
                &[ $( $enum_name::$variant, )* ]
            }

            pub fn pairs() -> &'static [(&'static str, $enum_name)] {
                &[ $( (stringify!($variant), $enum_name::$variant), )* ]
            }

            pub fn reverse_lookup() -> HashMap<&'static str, $enum_name> {
                $enum_name::pairs().iter().copied().collect()
            }

            pub fn from_str(value: &str) -> Option<$enum_name> {
                Self::pairs().iter().find(|(s, _)| &value == s).copied().map(|(_, v)| v)
            }
        }
    };
}

enum_with_str!(
    LdtkActor,
    [Combatant, Speaker, Door, Chest, Emitter, Portal, Spawn]
);
