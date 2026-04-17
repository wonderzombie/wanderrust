use std::path::PathBuf;

use bevy::prelude::*;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::cell::Cell;

#[derive(Debug, Deserialize)]
pub struct LdtkProject {
    levels: Vec<LdtkLevel>,
}

#[derive(Debug, Deserialize)]
pub struct LdtkLevel {
    pub identifier: String,
    pub iid: String,
    #[serde(rename = "fieldInstances", default)]
    pub field_instances: Vec<LdtkField>,
    #[serde(rename = "layerInstances", default)]
    pub layer_instances: Vec<LdtkLayer>,
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
}

#[derive(Debug, Deserialize, Default)]
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
    pub cell: Cell,
    #[serde(rename = "__tile", default)]
    pub tile: LdtkPxTile,
    #[serde(rename = "fieldInstances", default)]
    pub field_instances: Vec<LdtkField>,
}

// { "px": [32,32], "src": [0,208], "f": 0, "t": 637, "d": [34], "a": 1 },

#[derive(Debug, Deserialize, Default)]
pub struct LdtkGridTile {
    #[serde(rename = "t")]
    pub atlas_idx: usize,
    #[serde(rename = "a")]
    pub alpha: f64,
    #[serde(rename = "f")]
    pub flip_bits: i32,
    #[serde(rename = "px")]
    pub px: Cell,
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

pub fn load_and_import(fname: PathBuf) -> Result<(), BevyError> {
    let serialized = std::fs::read_to_string(fname)?;

    let project = serde_json::from_str::<LdtkProject>(&serialized)?;

    println!("project levels count: {}", project.levels.len());

    println!("=== BEGIN DUMP ===");
    println!("{:#?}", project);
    println!("=== END DUMP ===");

    Ok(())
}
