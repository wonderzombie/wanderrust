use std::path::PathBuf;

use bevy::{platform::collections::HashSet, prelude::*};
use serde::Deserialize;
use serde_json::{Value, from_value};

use crate::{
    cell::Cell,
    gamestate::GameState,
    interactions::{self, Interactable},
    light::{self, Emitter, LightLevel},
    tilemap::{
        self, Dimensions, EmitterCell, InterxCell, Portal, PortalCell, StratumId, StratumTileSpec,
        TileCell, WorldSpec,
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
    #[serde(rename = "worldGridWidth")]
    pub grid_width: u32,
    #[serde(rename = "worldGridHeight")]
    pub grid_height: u32,
}

#[derive(Debug, Deserialize)]
pub struct LdtkLevel {
    #[serde(rename = "fieldInstances", default)]
    pub field_instances: Vec<LdtkField>,
    #[serde(rename = "layerInstances", default)]
    pub layer_instances: Vec<LdtkLayer>,
    #[serde(rename = "pxHei")]
    pub px_height: f32,
    #[serde(rename = "worldDepth")]
    pub world_depth: i32,
}

impl LdtkLevel {
    pub fn light_level_or(&self, default: LightLevel) -> LightLevel {
        self.field_instances
            .iter()
            .find(|fi| fi.identifier.eq_ignore_ascii_case("light_level"))
            .and_then(|fi| match ParsedValue::from(fi) {
                ParsedValue::LightLevelEnum(l) => LightLevel::from_str(l),
                _ => None,
            })
            .unwrap_or(default)
    }
}

#[derive(Debug, Deserialize)]
pub struct LdtkLayer {
    #[serde(rename = "__identifier")]
    pub identifier: String,
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

pub fn generate_ldtk_world(
    mut commands: Commands,
    project: Option<Res<LdtkProject>>,
    mut ns: ResMut<NextState<GameState>>,
) {
    let Some(project) = project else {
        return;
    };
    let project = project.as_ref();
    let mut world = WorldSpec::default();

    for level in &project.levels {
        let stratum_id = StratumId(level.world_depth);
        let spec = world.maps.entry(stratum_id).or_default();
        for layer in &level.layer_instances {
            spec.size = Dimensions {
                width: layer.c_width as u32,
                height: layer.c_height as u32,
                ..default()
            };

            spec.tiles
                .extend(get_grid_tiles(&layer.grid_tiles, level.px_height));

            for actor in &layer.entities {
                let cell = actor.ldtk_cell.to_wandrs(layer.c_height);

                let t: TileIdx = actor.get_tile();
                if t == TileIdx::default() {
                    warn!("actor has default tile: {:?}", actor);
                }

                match ParsedActor::from_ldtk(actor) {
                    Some(ParsedActor::Interactable(i)) => spec.interxs.push((i, cell)),
                    Some(ParsedActor::Emitter(e)) => spec.emitters.push((e, cell)),
                    Some(ParsedActor::Portal(p)) => spec.portals.push((p, cell)),
                    Some(ParsedActor::Spawn) => spec.spawn_point = Some((stratum_id, cell)),
                    None => warn!("ignoring unparsable actor: {:?}", actor),
                }
            }
        }
    }

    commands.insert_resource(world);
    ns.set(GameState::Loading);
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

    let mut distinct_entities = HashSet::<(String, Cell)>::new();

    let mut new_tiles: Vec<TileCell> = Vec::new();
    let mut new_portals: Vec<PortalCell> = Vec::new();
    let mut new_interx: Vec<InterxCell> = Vec::new();
    let mut new_emitters: Vec<EmitterCell> = Vec::new();

    let level = project.levels.first().unwrap();
    let mut spawn: Option<Cell> = None;
    let light_level = level.light_level_or(LightLevel::Dark);

    let mut c_wid = 1;
    let mut c_hei = 1;

    for layer in &level.layer_instances {
        c_wid = c_wid.max(layer.c_width);
        c_hei = c_hei.max(layer.c_height);

        if layer.layer_type.eq_ignore_ascii_case("tiles") {
            let grid_tiles = get_grid_tiles(&layer.grid_tiles, level.px_height);
            new_tiles.extend(grid_tiles);
        }

        if layer.layer_type.eq_ignore_ascii_case("entities") {
            for actor in &layer.entities {
                let wandrs_cell = actor.ldtk_cell.to_wandrs(layer.c_height);
                distinct_entities.insert((actor.identifier.clone(), wandrs_cell));

                let t: TileIdx = actor.get_tile();
                if t == TileIdx::default() {
                    warn!("actor has default tile: {:?}", actor);
                }

                info!("entity: {:?}", actor.identifier);

                match ParsedActor::from_ldtk(actor) {
                    Some(ParsedActor::Interactable(i)) => new_interx.push((i, t, wandrs_cell)),
                    Some(ParsedActor::Emitter(e)) => new_emitters.push((e, t, wandrs_cell)),
                    Some(ParsedActor::Portal(p)) => new_portals.push((p, t, wandrs_cell)),
                    Some(ParsedActor::Spawn) => spawn = Some(wandrs_cell),
                    None => error!(
                        "skipping unknown actor type: {:?} (ty {:?}",
                        actor,
                        actor.ty()
                    ),
                }
            }
        }
    }
    info!("🧰 new tiles: {}", new_tiles.len());
    info!("🧰 new emitters: {}", new_emitters.len());
    info!("🧰 new interactables: {}", new_interx.len());
    info!("🧰 new portals: {}", new_portals.len());

    // HACKHACK for testing
    let mut spec = StratumTileSpec::default();
    spec.all_tiles.insert(StratumId(0), new_tiles);
    spec.all_portals.insert(StratumId(0), new_portals);
    spec.all_interxs.insert(StratumId(0), new_interx);
    spec.all_emitters.insert(StratumId(0), new_emitters);
    spec.light_level = light_level;
    spec.size = Dimensions {
        width: c_wid as u32,
        height: c_hei as u32,
        tile_size: tiles::TILE_SIZE_PX as u32,
    };

    spec.spawn_point = if let Some(spawn) = spawn {
        (StratumId(0), spawn)
    } else {
        error!("did not find spawn point");
        (StratumId(0), Cell::default())
    };

    info!("🧰 setting spawn to {:?}", spec.spawn_point);

    commands.insert_resource(spec);

    ns.set(GameState::Loading);
}

fn get_grid_tiles(grid_tiles: &Vec<LdtkGridTile>, level_px_height: f32) -> Vec<TileCell> {
    let mut new_tiles: Vec<TileCell> = vec![];
    for grid_tile in grid_tiles {
        let tile_idx = TileIdx::from_idx(grid_tile.atlas_idx).unwrap_or(TileIdx::GridSquare);
        let cell = grid_tile.into_cell(level_px_height);
        new_tiles.push((tile_idx, cell));
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
