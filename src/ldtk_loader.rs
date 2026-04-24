use std::path::PathBuf;

use bevy::prelude::*;
use serde::Deserialize;
use serde_json::{Value, from_value};

use crate::{
    cell::Cell,
    interactions::{self, Interactable},
    light::{self, Emitter, LightLevel},
    tilemap::{
        self, Dimensions, EmitterCell, InterxCell, Portal, PortalCell, StratumId, StratumSpec,
        StratumTileSpec, TileCell, WorldSpec,
    },
    tiles::{self, SHEET_SIZE_G, TileIdx},
};

macro_rules! enum_with_str {
    ( $enum_name:ident, [ $( $variant:ident ),* ] ) => {
        #[derive(Default, Debug, Eq, PartialEq, Copy, Clone, Hash, Reflect)]
        pub enum $enum_name {
            #[default]
            Unset,
            $( $variant, )*
        }

        #[allow(dead_code)]
        impl $enum_name {
            pub fn all() -> &'static [$enum_name] {
                &[ $( $enum_name::$variant, )* ]
            }

            pub fn pairs() -> &'static [(&'static str, $enum_name)] {
                &[ $( (stringify!($variant), $enum_name::$variant), )* ]
            }

            pub fn from_str(value: &str) -> Option<$enum_name> {
                Self::pairs().iter().find(|(s, _)| &value == s).copied().map(|(_, v)| v)
            }
        }
    };
}

pub trait LdtkEntityExt<T> {
    fn from_ldtk(entity: &LdtkEntity) -> Option<T>;
}

#[derive(Debug, Deserialize, Resource)]
pub struct LdtkProject {
    pub levels: Vec<LdtkLevel>,
    // #[serde(rename = "worldGridWidth")]
    // pub grid_width: u32,
    // #[serde(rename = "worldGridHeight")]
    // pub grid_height: u32,
}

#[derive(Debug, Deserialize)]
pub struct LdtkLevel {
    #[serde(rename = "layerInstances", default)]
    pub layer_instances: Vec<LdtkLayer>,
    #[serde(rename = "pxHei")]
    pub px_height: f32,
    #[serde(rename = "worldDepth")]
    pub world_depth: i32,
}

#[derive(Debug, Deserialize)]
pub struct LdtkLayer {
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
    #[serde(rename = "__grid")]
    pub ldtk_cell: LdtkCell,
    /// This is the primary tile field for most entities.
    #[serde(rename = "__tile", default)]
    pub tile: LdtkPxTile,
    #[serde(rename = "fieldInstances", default)]
    pub field_instances: Vec<LdtkField>,
}

const LDTK_ENTITES_ENUM: &str = "Actor";

#[allow(dead_code)]
impl LdtkEntity {
    pub fn ty(&self) -> Option<LdtkActor> {
        self.field_instances
            .iter()
            .find(|f| f.identifier.eq_ignore_ascii_case(LDTK_ENTITES_ENUM))
            .and_then(|v| v.val.as_str())
            .and_then(LdtkActor::from_str)
    }

    fn field_val(&self, key: &str) -> Option<ParsedValue> {
        self.field_instances
            .iter()
            .find(|it| it.identifier.eq_ignore_ascii_case(key))
            .map(ParsedValue::from)
    }

    pub fn get_bool(&self, key: &str) -> bool {
        match self.field_val(key) {
            Some(ParsedValue::Bool(v)) => v,
            _ => false,
        }
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        match self.field_val(key) {
            Some(ParsedValue::Ztring(s)) => Some(s.clone()),
            _ => None,
        }
    }

    pub fn get_str_array(&self, key: &str) -> Option<Vec<String>> {
        match self.field_val(key) {
            Some(ParsedValue::ArrayString(vec)) => Some(vec.clone()),
            _ => None,
        }
    }

    pub fn get_actor_enum(&self, key: &str) -> Option<String> {
        match self.field_val(key) {
            Some(ParsedValue::ActorEnum(s)) => Some(s),
            _ => None,
        }
    }

    pub fn get_tile_field(&self, key: &str) -> Option<TileIdx> {
        match self.field_val(key) {
            Some(ParsedValue::PxTile(t)) => Some(t),
            _ => None,
        }
    }

    pub fn get_tile(&self) -> TileIdx {
        self.tile.into()
    }
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
pub struct LdtkGridTile {
    #[serde(rename = "t")]
    pub atlas_idx: usize,
    #[serde(rename = "px")]
    px: Vec2,
}

impl LdtkGridTile {
    fn into_cell(self, level_height_px: f32) -> Cell {
        let x = self.px.x;
        let y = self.px.y;
        let cx = (x / 16.0) as i32;
        let cy = ((level_height_px - y) / 16.0) as i32 - 1;
        Cell::new(cx, cy)
    }
}

impl From<TileIdx> for LdtkGridTile {
    fn from(value: TileIdx) -> Self {
        Self {
            atlas_idx: value.into(),
            ..default()
        }
    }
}

#[derive(Debug, Deserialize, Default, Copy, Clone)]
pub struct LdtkPxTile {
    #[serde(rename = "x")]
    atlas_x_px: i32,
    #[serde(rename = "y")]
    atlas_y_px: i32,
}

impl From<LdtkPxTile> for TileIdx {
    fn from(value: LdtkPxTile) -> TileIdx {
        let cell = Cell::new(value.atlas_x_px / 16, value.atlas_y_px / 16);
        let idx = cell.to_idx(SHEET_SIZE_G.x);
        TileIdx::from_idx(idx).unwrap_or(TileIdx::GridSquare)
    }
}

#[derive(Deref, Debug, Deserialize, Default, Clone, Copy)]
pub struct LdtkCell(Cell);

impl LdtkCell {
    fn to_wandrs(self, level_height_cells: i32) -> Cell {
        Cell::new(self.x, level_height_cells - 1 - self.y)
    }
}

pub fn load_and_import(fname: PathBuf) -> Result<LdtkProject, BevyError> {
    let serialized = std::fs::read_to_string(&fname)?;
    let project = serde_json::from_str::<LdtkProject>(&serialized)?;
    info!(
        "🧰 {:?}: project levels count: {}",
        fname.file_name(),
        project.levels.len()
    );
    Ok(project)
}

pub fn generate_ldtk_world(mut commands: Commands, project: Option<Res<LdtkProject>>) {
    let Some(project) = project else {
        return;
    };
    let project = project.as_ref();
    let mut world = WorldSpec {
        light_level: LightLevel::Night,
        ..Default::default()
    };

    for level in &project.levels {
        let stratum_id = StratumId(level.world_depth);
        let spec: &mut StratumSpec = world.maps.entry(stratum_id).or_default();
        spec.light_level = level.light_level_or(world.light_level);
        for layer in &level.layer_instances {
            spec.size = Dimensions {
                width: layer.c_width as u32,
                height: layer.c_height as u32,
                ..default()
            };

            if layer.layer_type.eq_ignore_ascii_case("tiles") {
                info!("🧰 loading {} grid tiles", layer.grid_tiles.len());
                spec.tiles
                    .extend(get_grid_tiles(&layer.grid_tiles, level.px_height));
            }

            if layer.layer_type.eq_ignore_ascii_case("entities") {
                info!("🧰 loading {} entities", layer.entities.len());
                for actor in &layer.entities {
                    let cell = actor.ldtk_cell.to_wandrs(layer.c_height);

                    if actor.get_tile() == TileIdx::default() {
                        warn!("actor has default tile: {:?}", actor);
                    }

                    match ParsedActor::from_ldtk(actor) {
                        Some(ParsedActor::Interactable(i)) => spec.interxs.push((i, cell)),
                        Some(ParsedActor::Emitter(e)) => spec.emitters.push((e, cell)),
                        Some(ParsedActor::Portal(p)) => spec.portals.push((p, cell)),
                        Some(ParsedActor::Spawn) => {
                            world.spawn_point = (stratum_id, cell);
                            if stratum_id == StratumId::default() && cell == Cell::default() {
                                warn!(
                                    "world spawn: both stratum ID and cell are defaults; zero values?"
                                );
                            }
                        }
                        None => warn!("ignoring unparsable actor: {:?}", actor),
                    }
                }
            }
        }
    }

    commands.insert_resource(world);
}

fn get_grid_tiles(grid_tiles: &Vec<LdtkGridTile>, level_px_height: f32) -> Vec<TileCell> {
    let mut new_tiles: Vec<TileCell> = vec![];
    let mut blank = 0;
    let mut zero = 0;
    for grid_tile in grid_tiles {
        let tile_idx = TileIdx::from_idx(grid_tile.atlas_idx).unwrap_or(TileIdx::GridSquare);
        let cell = grid_tile.into_cell(level_px_height);
        if cell == Cell::ZERO {
            zero += 1;
        }
        if tile_idx == TileIdx::Blank {
            blank += 1;
        }
        new_tiles.push((tile_idx, cell));
    }
    // info!("🧰 blank tiles: {}; zero tiles: {}", blank, zero);
    if blank == grid_tiles.len() {
        error!("🧰 {} out of {} tiles were blank", blank, grid_tiles.len());
    }
    if zero == grid_tiles.len() {
        error!(
            "🧰 {} out of {} tiles were at (0, 0)",
            zero,
            grid_tiles.len()
        );
    }
    new_tiles
}

#[derive(Debug, Clone, Default)]
pub enum ParsedValue {
    #[default]
    Unset,
    ActorEnum(String),
    Ztring(String),
    PxTile(TileIdx),
    Bool(bool),
    // EntityRef(HashMap<String, String>),
    ArrayString(Vec<String>),
    LightLevelEnum(String),
}

impl From<LdtkField> for ParsedValue {
    #[inline]
    fn from(field: LdtkField) -> ParsedValue {
        Self::from(&field)
    }
}

impl From<&LdtkField> for ParsedValue {
    fn from(field: &LdtkField) -> ParsedValue {
        use ParsedValue::*;
        let val = field.val.clone();
        match field.field_type.as_str() {
            "LocalEnum.Actor" => match val.as_str() {
                Some(s) => ActorEnum(s.to_string()),
                None => Unset,
            },
            "LocalEnum.LightLevel" => match val.as_str() {
                Some(l) => LightLevelEnum(l.to_string()),
                None => Unset,
            },
            "String" => match val.as_str() {
                Some(s) => Ztring(s.to_string()),
                None => Unset,
            },
            "Tile" => match from_value::<LdtkPxTile>(val) {
                Ok(px_tile) => PxTile(px_tile.into()),
                Err(_) => Unset,
            },
            "Bool" => match field.val.as_bool() {
                Some(v) => Bool(v),
                None => Unset,
            },
            "Array<String>" => match from_value::<Vec<String>>(val) {
                Ok(vec) => ArrayString(vec.clone()),
                Err(_) => Unset,
            },
            // "EntityRef" => match from_value::<HashMap<String, String>>(val) {
            //     Ok(map) => EntityRef(map),
            //     Err(_) => Unset,
            // },
            _ => Unset,
        }
    }
}

enum_with_str!(
    LdtkActor,
    [Combatant, Speaker, Door, Chest, Emitter, Portal, Spawn]
);

/// ParsedActor is the intermediate representation between LDtk types and wanderrust types.
/// NB that these must match the **Actor enum in LDtk**.
pub enum ParsedActor {
    Interactable(interactions::Interactable),
    Portal(tilemap::Portal),
    Emitter(light::Emitter),
    Spawn,
}

impl LdtkEntityExt<ParsedActor> for ParsedActor {
    fn from_ldtk(entity: &LdtkEntity) -> Option<ParsedActor> {
        let Some(e) = entity.ty() else {
            warn!("unknown LdtkEntity type: {:#?}", entity);
            return None;
        };

        match e {
            LdtkActor::Chest | LdtkActor::Door | LdtkActor::Combatant | LdtkActor::Speaker => {
                Interactable::from_ldtk(entity).map(Self::Interactable)
            }
            LdtkActor::Portal => Portal::from_ldtk(entity).map(Self::Portal),
            LdtkActor::Spawn => Some(Self::Spawn),
            LdtkActor::Emitter => Emitter::from_ldtk(entity).map(Self::Emitter),
            LdtkActor::Unset => {
                warn!("unknown LdtkEntity type: {:#?}", entity);
                None
            }
        }
    }
}
