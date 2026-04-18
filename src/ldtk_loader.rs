use std::path::PathBuf;

use bevy::{platform::collections::HashMap, prelude::*};
use serde::Deserialize;
use serde_json::Value;

use crate::cell::Cell;

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
    pub cell: Cell,
    #[serde(rename = "__tile", default)]
    pub tile: LdtkPxTile,
    #[serde(rename = "fieldInstances", default)]
    pub field_instances: Vec<LdtkField>,
    #[serde(rename = "__tags")]
    pub tags: Vec<String>,
}

impl LdtkEntity {
    pub fn field_map(&self) -> impl FieldMapExt {
        self.field_instances
            .clone()
            .into_iter()
            .map(|it| (it.identifier, it.val))
            .collect::<HashMap<String, Value>>()
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

pub fn load_and_import(fname: PathBuf) -> Result<LdtkProject, BevyError> {
    let serialized = std::fs::read_to_string(fname)?;
    let project = serde_json::from_str::<LdtkProject>(&serialized)?;
    info!("project levels count: {}", project.levels.len());
    Ok(project)
}
