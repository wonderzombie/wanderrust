//! Loads level and game object definitions from the LDtk JSON format so they can spawn.
//!
//! LDtk has *negative* Y as up and Bevy has *positive* Y as up.
//!
//! The LDtk documentation is the authority on these data structures; I've tried to mirror them as
//! much as makes sense for my needs. In truth, deserializing the JSON via serde is probably the
//! least complicated part.
//!
//! https://ldtk.io/docs/game-dev/json-overview/
use std::path::PathBuf;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value};

use crate::{
    cell::Cell,
    enum_with_str,
    interactions::{self, Interactable},
    light::{self, Emitter, LightLevel},
    route::Route,
    tilemap::{self, Dimensions, LevelId, LevelSpec, Portal, TileCell, WorldSpec},
    tiles::{SHEET_SIZE_G, TileIdx},
};

/// Converts from an LdtkEntity, a thin wrapper around [`serde_json::Value`] for (typically) one of
/// a handful of possible entity types. The fields vary from entity to entity, although some like
/// `name` or `tile` are common.
pub trait LdtkEntityExt<T> {
    fn from_ldtk(entity: &LdtkEntity) -> Option<T>;
}

#[derive(Debug, Deserialize, Resource)]
pub struct LdtkProject {
    /// Each [`LdtkLevel`] contains multiple layers, such as entities or map tiles.
    pub levels: Vec<LdtkLevel>,
    /// Width of the level measured in grid cells.
    #[serde(rename = "worldGridWidth")]
    pub grid_width: u32,
    /// Height of the level measured in grid cells.
    #[serde(rename = "worldGridHeight")]
    pub grid_height: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LdtkLevel {
    /// Unique ID as given by LDtk itself.
    pub uid: i32,
    /// Name of the level.
    pub identifier: String,
    /// Layer instances belonging to this level, if any.
    pub layer_instances: Vec<LdtkLayer>,
    /// The pixel height to convert from LDtk coordinates to Bevy coordinates.
    pub px_hei: f32,
    /// The depth of this level as assigned or configured in LDtk.
    pub world_depth: i32,
    /// A list of fields belonging to the level.
    pub field_instances: Vec<LdtkField>,
    /// Pixel x coordinate in the LDtk world.
    pub world_x: i32,
    /// Pixel y coordinate in the LDtk world.
    pub world_y: i32,
}

impl LdtkLevel {
    /// Get this level's setting for [`LightLevel`] via the `light_level` field, if any.
    fn light_level(&self) -> Option<LightLevel> {
        self.field_instances
            .iter()
            .find(|field| field.identifier == "light_level")
            .map(ParsedValue::from)
            .and_then(|it| match it {
                ParsedValue::LightLevelEnum(level) => LightLevel::from_text(level),
                _ => None,
            })
    }
}

/// Contains data about an LDtk `layerInstance`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LdtkLayer {
    /// Can be IntGrid, Entities, Tiles or AutoLayer.
    #[serde(rename = "__type")]
    pub layer_type: String,
    /// Contains [`LdtkGridTile`] when this is a layer with tiles.
    pub grid_tiles: Vec<LdtkGridTile>,
    /// Contains [`LdtkGridTile`] when this layer is an AutoLayer.
    pub auto_layer_tiles: Vec<LdtkGridTile>,
    /// Contains [`LdtkEntity`] when this an Entities layer.
    #[serde(rename = "entityInstances", default)]
    pub entities: Vec<LdtkEntity>,
    /// The width of this level in cells.
    #[serde(rename = "__cWid")]
    pub c_width: i32,
    /// The height of this level in cells.
    #[serde(rename = "__cHei")]
    pub c_height: i32,
}

/// A single field of some type belonging to a Level or Entity.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct LdtkField {
    #[serde(rename = "__identifier")]
    pub identifier: String,
    #[serde(rename = "__type")]
    pub field_type: String,
    #[serde(rename = "__value")]
    pub val: Value,
}

/// A game object defined in LDtk.
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LdtkEntity {
    /// Editor display name.
    #[serde(rename = "__identifier")]
    pub identifier: String,
    /// Cell in LDtk coordinates.
    #[serde(rename = "__grid")]
    pub ldtk_cell: LdtkCell,
    /// This is the primary tile field for most entities.
    #[serde(rename = "__tile", default)]
    pub tile: Option<LdtkPxTile>,
    /// Any remaining fields not explicitly mentioned here.
    pub field_instances: Vec<LdtkField>,
}

const LDTK_ENTITIES_ENUM: &str = "Actor";

impl LdtkEntity {
    /// Attempts to guess the type of [`LdtkActor`] this [`LdtkEntity`] represents.
    pub fn ty(&self) -> Option<LdtkActor> {
        self.field_instances
            .iter()
            .find(|f| f.identifier.eq_ignore_ascii_case(LDTK_ENTITIES_ENUM))
            .and_then(|v| v.val.as_str())
            .and_then(LdtkActor::from_str)
    }

    /// Returns a [`ParsedValue`] for the field named [`key`].
    fn field_val(&self, key: &str) -> Option<ParsedValue> {
        self.field_instances
            .iter()
            .find(|it| it.identifier.eq_ignore_ascii_case(key))
            .map(ParsedValue::from)
    }

    /// Reads a `boolean` field's value, returning false when there isn't one.
    pub fn get_bool(&self, key: &str) -> bool {
        match self.field_val(key) {
            Some(ParsedValue::Bool(v)) => v,
            _ => false,
        }
    }

    /// Reads a `String` field's value.
    pub fn get_string(&self, key: &str) -> Option<String> {
        match self.field_val(key) {
            Some(ParsedValue::Ztring(s)) => Some(s.clone()),
            _ => None,
        }
    }

    /// Reads an `Array<String>` field's value as`Vec<String>`.
    pub fn get_str_array(&self, key: &str) -> Option<Vec<String>> {
        match self.field_val(key) {
            Some(ParsedValue::ArrayString(vec)) => Some(vec.clone()),
            _ => None,
        }
    }

    /// Reads an `Array<Point>` field's value as `Vec<LdtkGridPoint>`.
    /// These are from LDtk space and must be converted.
    pub fn get_point_array(&self, key: &str) -> Option<Vec<LdtkGridPoint>> {
        match self.field_val(key) {
            Some(ParsedValue::ArrayPoint(points)) => Some(points),
            _ => None,
        }
    }

    // pub fn get_actor_enum(&self, key: &str) -> Option<String> {
    //     match self.field_val(key) {
    //         Some(ParsedValue::ActorEnum(s)) => Some(s),
    //         _ => None,
    //     }
    // }

    /// Reads a `Tile` field from an entity, if any.
    pub fn get_tile_field(&self, key: &str) -> Option<TileIdx> {
        match self.field_val(key) {
            Some(ParsedValue::PxTile(t)) => Some(t),
            _ => None,
        }
    }

    /// Gets the primary tile from this entity. The LDtk editor UI
    /// and wanderrust use this tile first and most often.
    pub fn get_tile(&self) -> TileIdx {
        self.tile.unwrap_or_default().into()
    }

    /// Reads the `name` field from the entity. This isn't the same
    /// as the identifier. See also [`LdtkEntity::deduce_display_name()`].
    pub fn get_name_field(&self) -> Option<String> {
        self.get_string("name")
    }

    /// Given an LDtk entity which may or may not warrant a name, find the most appropriate name
    /// among what is set.
    ///
    /// It tries:
    /// 1. `name` field unless `None`.
    /// 1. a [`TileIdx::label()`] unless `None`.
    /// 1. The entity's identifier unless `None` or empty.
    /// 1. The debug name for the tile.
    pub fn deduce_display_name(&self) -> String {
        self.get_name_field()
            .or_else(|| self.get_tile().label().map(Into::into))
            .or_else(|| Some(self.identifier.clone()).filter(|s| !s.is_empty()))
            .unwrap_or_else(|| self.get_tile().to_string())
    }
}

/// Defines a tile's location in the LDtk grid in pixels.
#[derive(Debug, Deserialize, Default, Clone, Copy)]
pub struct LdtkGridTile {
    /// The index of this tile's sprite in the sprite atlas.
    #[serde(rename = "t")]
    pub atlas_idx: usize,
    /// The pixel coordinates for this tile.
    #[serde(rename = "px")]
    px: Vec2,
}

impl LdtkGridTile {
    /// Converts [`LdtkGridTile`] into [`Cell`] using pixel height of level to flip the Y axis.
    fn into_cell(self, level_height_px: f32, depth: i32) -> Cell {
        let x = self.px.x;
        let y = self.px.y;
        let cx = (x / 16.0) as i32;
        let cy = ((level_height_px - y) / 16.0) as i32 - 1;
        Cell::at_depth(cx, cy, depth)
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

/// Describes a tile's coordinates in the LDtk atlas.
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

/// Defines a grid coordinate obtained from LDtk, so the y axis is inverted.
#[derive(Deref, PartialEq, Debug, Deserialize, Default, Clone, Copy)]
pub struct LdtkCell(pub Cell);

impl LdtkCell {
    /// Converts [`LdtkCell`] into [`Cell`] using pixel height of level to flip the Y axis.
    pub fn to_wandrs(self, level_height_cells: i32, depth: i32) -> Cell {
        Cell::at_depth(self.x, level_height_cells - 1 - self.y, depth)
    }
}

impl From<&LdtkGridPoint> for LdtkCell {
    fn from(value: &LdtkGridPoint) -> Self {
        LdtkCell(Cell::new(value.x, value.y))
    }
}

/// LdtkGridPoint is from LDtk's `Point` type, identifying a cell in LDtk space.
#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
pub struct LdtkGridPoint {
    #[serde(rename = "cx")]
    pub x: i32,
    #[serde(rename = "cy")]
    pub y: i32,
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
        grid_width: project.grid_width,
        grid_height: project.grid_height,
        ..Default::default()
    };

    for level in &project.levels {
        let level_id = LevelId(level.uid);
        let spec: &mut LevelSpec = world.maps.entry(level_id).or_default();
        spec.identifier = level.identifier.clone();

        assert!(
            level.world_depth >= 0,
            "negative world depths (i32) are not yet supported; `bevy_northstar` needs u32"
        );
        spec.depth = level.world_depth;
        world.depths.insert(tilemap::Depth(spec.depth));
        world.max_depth = world.max_depth.max(tilemap::Depth(spec.depth));
        spec.light_level = level.light_level().unwrap_or(world.light_level);
        spec.world_pos = Vec2 {
            x: level.world_x as f32,
            y: level.world_y as f32,
        };
        for layer in &level.layer_instances {
            spec.dimensions = Dimensions {
                width: layer.c_width as u32,
                height: layer.c_height as u32,
                depth: spec.depth,
                ..default()
            };

            let tiles = match layer.layer_type.to_lowercase().as_ref() {
                "autolayer" => &layer.auto_layer_tiles,
                "tiles" => &layer.grid_tiles,
                _ => &vec![],
            };

            spec.tiles
                .extend(get_grid_tiles(tiles, level.px_hei, level.world_depth));

            if layer.layer_type.eq_ignore_ascii_case("entities") {
                info!("🧰 {level_id}: loading {} entities", layer.entities.len());
                for actor in &layer.entities {
                    let cell = actor.ldtk_cell.to_wandrs(layer.c_height, spec.depth);

                    if actor.get_tile() == TileIdx::default() {
                        warn!("LDtk entity has default or empty tile: {:?}", actor);
                    }

                    match ParsedActor::from_ldtk(actor) {
                        Some(ParsedActor::Interactable(i)) => spec.interxs.push((i, cell)),
                        Some(ParsedActor::Emitter(e)) => spec.emitters.push((e, cell)),
                        Some(ParsedActor::Portal(p)) => spec.portals.push((p, cell)),
                        Some(ParsedActor::Spawn) => {
                            world.spawn_point = (level_id, cell);
                            if level_id == LevelId::default() && cell == Cell::default() {
                                warn!(
                                    "world spawn: both level ID and cell are defaults; zero values?"
                                );
                            }
                        }
                        Some(ParsedActor::Route) => {
                            let w = Route::new(&spec.dimensions, actor);
                            spec.routes.push(w);
                        }
                        None => warn!("ignoring unparsable actor: {actor:?}"),
                    }
                }
            }
        }
    }

    commands.insert_resource(world);
}

fn get_grid_tiles(
    grid_tiles: &Vec<LdtkGridTile>,
    level_px_height: f32,
    world_depth: i32,
) -> Vec<TileCell> {
    let mut new_tiles: Vec<TileCell> = vec![];
    let mut blank = 0;
    let mut nzero = 0;
    for grid_tile in grid_tiles {
        let tile_idx = TileIdx::from_idx(grid_tile.atlas_idx).unwrap_or(TileIdx::GridSquare);
        let cell = grid_tile.into_cell(level_px_height, world_depth);
        if cell == Cell::ZERO {
            nzero += 1;
        }
        if tile_idx == TileIdx::Blank {
            blank += 1;
        }
        new_tiles.push((tile_idx, cell));
    }
    if !grid_tiles.is_empty() {
        if blank == grid_tiles.len() {
            error!("🧰 {blank} out of {} tiles were blank", grid_tiles.len());
        }
        if nzero == grid_tiles.len() {
            error!(
                "🧰 {nzero} out of {} tiles were at (0, 0)",
                grid_tiles.len()
            );
        }
    }
    new_tiles
}

#[derive(Debug, Clone, Default)]
pub enum ParsedValue {
    #[default]
    Unset,
    // ActorEnum(String),
    Ztring(String),
    PxTile(TileIdx),
    Bool(bool),
    // EntityRef(HashMap<String, String>),
    ArrayString(Vec<String>),
    ArrayPoint(Vec<LdtkGridPoint>),
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
            // "LocalEnum.Actor" => match val.as_str() {
            //     Some(s) => ActorEnum(s.to_string()),
            //     None => Unset,
            // },
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
            "Array<Point>" => match from_value::<Vec<LdtkGridPoint>>(val) {
                Ok(vec) => ArrayPoint(vec),
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
    [
        Combatant, Speaker, Door, Chest, Emitter, Portal, Spawn, Shrine, Route
    ]
);

/// ParsedActor is the intermediate representation between LDtk types and
/// wanderrust types. NB that these must match the **Actor enum in LDtk**.
pub enum ParsedActor {
    Interactable(interactions::Interactable),
    Portal(tilemap::Portal),
    Emitter(light::Emitter),
    Spawn,
    Route,
}

impl LdtkEntityExt<ParsedActor> for ParsedActor {
    fn from_ldtk(entity: &LdtkEntity) -> Option<ParsedActor> {
        let Some(ty) = entity.ty() else {
            warn!("unknown LdtkEntity type: {entity:#?}");
            return None;
        };

        match ty {
            LdtkActor::Chest
            | LdtkActor::Door
            | LdtkActor::Combatant
            | LdtkActor::Speaker
            | LdtkActor::Shrine => Interactable::from_ldtk(entity).map(Self::Interactable),
            LdtkActor::Portal => Portal::from_ldtk(entity).map(Self::Portal),
            LdtkActor::Spawn => Some(Self::Spawn),
            LdtkActor::Route => Some(Self::Route),
            LdtkActor::Emitter => Emitter::from_ldtk(entity).map(Self::Emitter),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_point() -> Result<()> {
        let ldtk_grid_point = r#"{ "cx": 8, "cy": 6 }"#;

        let grid_point = serde_json::from_str::<LdtkGridPoint>(ldtk_grid_point)?;
        assert_eq!(grid_point, LdtkGridPoint { x: 8, y: 6 });

        let ldtk_array_grid_point = r#"[ { "cx": 9, "cy": 5 } ]"#;
        let array_grid_point = serde_json::from_str::<Vec<LdtkGridPoint>>(ldtk_array_grid_point)?;
        assert_eq!(array_grid_point, vec![LdtkGridPoint { x: 9, y: 5 }]);

        Ok(())
    }

    #[test]
    fn test_parsed_value_array_point() -> Result<()> {
        let grid_point = LdtkGridPoint { x: 77, y: 99 };
        let field = LdtkField {
            identifier: "waypoints".to_string(),
            field_type: "Array<Point>".to_string(),
            val: serde_json::to_value(vec![grid_point])?,
        };

        let parsed: ParsedValue = field.into();

        let ParsedValue::ArrayPoint(actual) = parsed else {
            panic!("expected ParsedValue to be ArrayPoint; was {parsed:?}");
        };

        assert_eq!(vec![grid_point], actual);
        Ok(())
    }

    #[test]
    fn test_parsed_value_ztring() -> Result<(), String> {
        let field = LdtkField {
            identifier: "name".to_string(),
            field_type: "String".to_string(),
            val: "Poppy".into(),
        };

        let parsed: ParsedValue = field.into();
        let name = "Poppy".to_string();

        let ParsedValue::Ztring(parsed_name) = parsed else {
            panic!("expected ParsedValue to be Ztring; was {parsed:?}");
        };

        assert_eq!(parsed_name, name);

        Ok(())
    }

    #[test]
    fn test_parsed_value_tile() -> Result<(), String> {
        let px_tile =
            serde_json::from_str(r#"{ "tilesetUid": 2, "x": 416, "y": 112, "w": 16, "h": 16 }"#)
                .map_err(|e| e.to_string())?;

        let parsed: ParsedValue = LdtkField {
            identifier: "tile".to_string(),
            field_type: "Tile".to_string(),
            val: px_tile,
        }
        .into();

        let ParsedValue::PxTile(parsed_tile) = parsed else {
            panic!("expected ParsedValue to be Tile; was {parsed:?}");
        };

        assert_eq!(parsed_tile, TileIdx::Chicken);

        Ok(())
    }

    #[test]
    fn test_deduce_display_name_uses_name() {
        let entity = LdtkEntity {
            identifier: "hello".into(),
            ldtk_cell: super::LdtkCell(Cell::new(5, 5)),
            // Chicken tile
            tile: Some(super::LdtkPxTile {
                atlas_x_px: 416,
                atlas_y_px: 112,
            }),
            field_instances: vec![LdtkField {
                identifier: "name".into(),
                field_type: "String".into(),
                val: "Poppy".into(),
            }],
        };
        let display_name = entity.deduce_display_name();
        assert_eq!(display_name, "Poppy".to_string());
    }

    #[test]
    fn test_deduce_display_name_uses_identifier() {
        let entity = LdtkEntity {
            identifier: "Identifier".into(),
            ldtk_cell: super::LdtkCell(Cell::new(5, 5)),
            // Chicken tile
            tile: Some(super::LdtkPxTile {
                atlas_x_px: 416,
                atlas_y_px: 112,
            }),
            field_instances: vec![],
        };
        let display_name = entity.deduce_display_name();
        assert_eq!(display_name, "Identifier".to_string());
    }

    #[test]
    fn test_deduce_display_name_uses_tile_label() {
        let entity = LdtkEntity {
            identifier: "Some Kind Of Pot".into(),
            ldtk_cell: super::LdtkCell(Cell::new(5, 5)),
            // Chest tile has a label.
            tile: Some(super::LdtkPxTile {
                atlas_x_px: 80,
                atlas_y_px: 224,
            }),
            field_instances: vec![
                LdtkField {
                    identifier: "contents".into(),
                    field_type: "String".into(),
                    val: Value::from("gold:2"),
                },
                LdtkField {
                    identifier: "is_open".into(),
                    field_type: "Bool".into(),
                    val: Value::from(false),
                },
            ],
        };
        let display_name = entity.deduce_display_name();
        assert_eq!(display_name, "pot".to_string());
    }
}
