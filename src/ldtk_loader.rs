use std::path::PathBuf;

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use serde::Deserialize;
use serde_json::{Value, from_value};

use crate::{
    cell::Cell,
    gamestate::GameState,
    interactions::{self, Interactable},
    light::{self, Emitter, LightLevel},
    tilemap::{
        self, Dimensions, EmitterCell, InterxCell, Portal, PortalCell, StratumId, TileCell,
        TilemapSpec,
    },
    tiles::{self, SHEET_SIZE_G, TileIdx},
};

pub trait LdtkEntityExt<T> {
    fn from_ldtk(entity: &LdtkEntity) -> Option<T>;
}

#[derive(Debug, Deserialize, Resource)]
pub struct LdtkProject {
    pub levels: Vec<LdtkLevel>,
    #[serde(rename = "worldGridWidth")]
    pub grid_width: u32,
    #[serde(rename = "worldGridHeight")]
    pub grid_height: u32,
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

    pub fn get_light_enum(&self, key: &str) -> Option<String> {
        match self.field_val(key) {
            Some(ParsedValue::LightLevelEnum(l)) => Some(l),
            _ => None,
        }
    }

    pub fn get_tile_field(&self, key: &str) -> Option<TileIdx> {
        match self.field_val(key) {
            Some(ParsedValue::PxTile(t)) => Some(t),
            _ => None,
        }
    }

    pub fn get_default_tile(&self) -> TileIdx {
        self.tile.into()
    }
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
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
        let tile_idx = cell.to_idx(SHEET_SIZE_G.x);
        TileIdx::pairs()
            .iter()
            .find(|(idx, _)| *idx == tile_idx)
            .map(|(_, tile)| *tile)
            .unwrap_or_default()
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

/// Converts from LDtk (+y is down) to bevy (+y is up).
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
    let mut distinct_entities = HashSet::<(String, Cell)>::new();

    let mut new_tiles: Vec<TileCell> = Vec::new();
    let mut new_portals: Vec<PortalCell> = Vec::new();
    let mut new_interx: Vec<InterxCell> = Vec::new();
    let mut new_emitters: Vec<EmitterCell> = Vec::new();

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

        if layer.layer_type.eq_ignore_ascii_case("tiles") {
            for tile in &layer.grid_tiles {
                let tile_idx = lookup.get(&tile.atlas_idx).copied().unwrap_or_default();
                distinct_tiles.insert(tile_idx);
                let cell = px_to_cell(tile.px.x, tile.px.y, level.px_height);
                new_tiles.push((tile_idx, cell));
            }
        }

        if layer.layer_type.eq_ignore_ascii_case("entities") {
            for actor in &layer.entities {
                let cell = ldtk_cell_to_wanderrust(actor.cell, layer.c_height);
                distinct_entities.insert((actor.identifier.clone(), cell));

                let t: TileIdx = actor.get_default_tile();
                if t == TileIdx::Blank {
                    warn!("actor has blank tile: {:?}", actor);
                }

                match ParsedActor::from_ldtk(actor) {
                    Some(ParsedActor::Interactable(i)) => new_interx.push((i, t, cell)),
                    Some(ParsedActor::Emitter(e)) => new_emitters.push((e, t, cell)),
                    Some(ParsedActor::Portal(p)) => new_portals.push((p, t, cell)),
                    Some(ParsedActor::Spawn) => spawn = Some(cell),
                    None => warn!("ignoring unparsable actor: {:?}", actor),
                }
            }
        }
    }
    info!("new tiles: {}", new_tiles.len());
    info!("new emitters: {}", new_emitters.len());
    info!("new interactables: {}", new_interx.len());
    info!("new portals: {}", new_portals.len());

    // HACKHACK for testing
    let mut spec = TilemapSpec::default();
    spec.all_tiles.insert(StratumId(0), new_tiles);
    spec.all_portals.insert(StratumId(0), new_portals);
    spec.all_interxs.insert(StratumId(0), new_interx);
    spec.light_level = LightLevel::Bright;
    spec.size = Dimensions {
        width: c_wid as u32,
        height: c_hei as u32,
        tile_size: tiles::TILE_SIZE_PX as u32,
    };

    spec.spawn_point = if let Some(spawn) = spawn {
        spawn
    } else {
        error!("did not find spawn point");
        Cell::default()
    };

    info!("setting spawn to {}", spec.spawn_point);

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

#[derive(Debug, Clone, Default)]
pub enum ParsedValue {
    #[default]
    Unset,
    ActorEnum(String),
    Ztring(String),
    PxTile(TileIdx),
    Bool(bool),
    EntityRef(HashMap<String, String>),
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
            "EntityRef" => match from_value::<HashMap<String, String>>(val) {
                Ok(map) => EntityRef(map),
                Err(_) => Unset,
            },
            _ => Unset,
        }
    }
}

enum_with_str!(
    LdtkActor,
    [Combatant, Speaker, Door, Chest, Emitter, Portal, Spawn]
);

pub enum ParsedActor {
    Interactable(interactions::Interactable),
    Portal(tilemap::Portal),
    Emitter(light::Emitter),
    Spawn,
}

impl LdtkEntityExt<ParsedActor> for ParsedActor {
    fn from_ldtk(entity: &LdtkEntity) -> Option<ParsedActor> {
        match entity.ty()? {
            LdtkActor::Chest | LdtkActor::Door | LdtkActor::Combatant | LdtkActor::Speaker => {
                Interactable::from_ldtk(entity).map(Self::Interactable)
            }
            LdtkActor::Portal => Portal::from_ldtk(entity).map(Self::Portal),
            LdtkActor::Spawn => Some(Self::Spawn),
            LdtkActor::Emitter => Emitter::from_ldtk(entity).map(Self::Emitter),
            LdtkActor::Unset => None,
        }
    }
}
