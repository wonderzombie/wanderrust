use bevy::{platform::collections::HashSet, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    actors::{Actor, PieceBundle},
    atlas::SpriteAtlas,
    interxables::*,
    inventory::*,
    items::ItemId,
    ldtk_loader::{LdtkActor, LdtkEntity, LdtkEntityExt},
    mobs::{self},
    tilemap::{Level, WorldSpec},
    tiles::TileIdx,
};

/// A component representing an interactable object in the world, such as a door
/// or chest, that can be interacted with by actors.
#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize, Eq, PartialEq)]
#[reflect(Component)]
#[require(Actor)]
pub enum Interactable {
    Door {
        is_open: bool,
        requires: Option<ItemId>,
        tile_idx: TileIdx,
    },
    Chest {
        is_open: bool,
        contents: Option<Inventory>,
        tile_idx: TileIdx,
    },
    Speaker {
        name: String,
        tile_idx: TileIdx,
        lines: Vec<String>,
    },
    Mob {
        name: String,
        tile_idx: TileIdx,
    },
    Shrine {
        id: String,
        tile_idx: TileIdx,
    },
}

impl Interactable {
    /// Uses the tile label for inanimate-adjacent types. Animate types have names.
    pub fn display_name(&self) -> Option<String> {
        match self {
            Interactable::Door { tile_idx, .. } | Interactable::Chest { tile_idx, .. } => {
                tile_idx.label().map(String::from)
            }
            Interactable::Speaker { name, .. } | Interactable::Mob { name, .. } => {
                Some(name.clone())
            }
            Interactable::Shrine { id, .. } => Some(id.clone()),
        }
    }

    pub fn tile(&self) -> TileIdx {
        match self {
            Self::Chest { tile_idx, .. }
            | Self::Door { tile_idx, .. }
            | Self::Mob { tile_idx, .. }
            | Self::Speaker { tile_idx, .. }
            | Self::Shrine { tile_idx, .. } => *tile_idx,
        }
    }
}

impl LdtkEntityExt<Interactable> for Interactable {
    fn from_ldtk(entity: &LdtkEntity) -> Option<Interactable> {
        let Some(ty) = entity.ty() else {
            error!(
                "📦 unknown interactable type: {:?} on LdtkEntity {entity:?}",
                entity.ty(),
            );
            return None;
        };

        let tile_idx = entity.get_tile();
        let name = entity.deduce_display_name();

        match ty {
            LdtkActor::Combatant => Some(Self::Mob { name, tile_idx }),
            LdtkActor::Speaker => {
                let lines = entity.get_str_array("lines").unwrap_or_default();
                if lines.is_empty() {
                    error!("found zero lines for speaker: {name} {tile_idx}");
                }
                Some(Self::Speaker {
                    name,
                    tile_idx,
                    lines,
                })
            }
            LdtkActor::Door => {
                let requires = entity.get_string("requires").and_then(ItemId::from_label);
                let is_open = entity.get_bool("is_open");
                Some(Self::Door {
                    is_open,
                    requires,
                    tile_idx,
                })
            }
            LdtkActor::Chest => {
                let contents = entity
                    .get_str_array("contents")
                    .and_then(Inventory::from_str_array);
                if contents.is_none() {
                    error!("empty chest found: {name} {tile_idx}\n{entity:#?}")
                }
                let is_open = entity.get_bool("is_open");
                Some(Self::Chest {
                    is_open,
                    contents,
                    tile_idx,
                })
            }
            LdtkActor::Shrine => {
                let id = entity.get_string("id").unwrap_or_default();
                Some(Self::Shrine { tile_idx, id })
            }
            _ => None,
        }
    }
}

/// Given an [`Interactable`], insert the [`crate::interxables`] version.
struct InsertInto(pub Interactable);

impl EntityCommand for InsertInto {
    type Out = ();

    fn apply(self, mut entity: EntityWorldMut) -> Self::Out {
        let InsertInto(value) = self;
        let name_component = Name::new(
            value
                .display_name()
                .unwrap_or_else(|| format!("MISSINGNAME")),
        );

        match value {
            Interactable::Door {
                is_open,
                requires,
                tile_idx,
            } => {
                entity
                    .insert(door::Door {
                        is_open,
                        requires,
                        tile_idx,
                    })
                    .insert((name_component, tile_idx));
            }
            Interactable::Chest {
                is_open,
                contents,
                tile_idx,
            } => {
                entity
                    .insert(chest::Chest { is_open, contents })
                    .insert((name_component, tile_idx));
            }
            Interactable::Speaker {
                name,
                tile_idx,
                lines,
            } => {
                entity
                    .insert(speaker::Speaker { name, lines })
                    .insert((name_component, tile_idx));
            }
            Interactable::Shrine { id, tile_idx } => {
                entity
                    .insert(shrine::Shrine { id })
                    .insert((name_component, tile_idx));
            }
            Interactable::Mob { name, tile_idx } => {
                entity
                    .insert(mobs::Mob {
                        name: name.clone(),
                        tile_idx,
                    })
                    .insert((name_component, tile_idx));
            }
        }
    }
}

/// ShrinesVisited tracks all the shrine entities which the player has visited.
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct ShrinesVisited(pub HashSet<Entity>);

/// Examine is a general word for interactions.
#[derive(Message, Debug, Copy, Clone)]
pub struct Examine {
    pub actor: Entity,
    pub target: Entity,
}

pub fn spawn_interxs(
    mut commands: Commands,
    world_spec: Res<WorldSpec>,
    atlas: Res<SpriteAtlas>,
    levels: Query<&Level>,
) {
    for Level(level_entity, level_id) in levels.iter() {
        let Some(spec) = world_spec.maps.get(level_id) else {
            continue;
        };

        info!("📦 {level_id} {level_entity}: spawning interactables");

        let mut count = 0;
        for (interx, cell) in spec.interxs.iter() {
            commands
                .spawn((
                    ChildOf(*level_entity), // for display purposes
                    PieceBundle {
                        cell: *cell,
                        sprite: atlas.sprite(),
                        ..default()
                    },
                ))
                .queue(InsertInto(interx.clone()));
            info!(
                "spawning {} ({})",
                interx.display_name().unwrap_or_else(|| cell.to_string()),
                interx.tile()
            );
            count += 1;
        }

        info!("📦 {level_id} {level_entity}: spawned {count} interactables");
    }
}

pub fn plugin(app: &mut App) {
    app.add_message::<Examine>()
        .init_resource::<ShrinesVisited>();
}

#[cfg(test)]
mod tests {

    use crate::items::Quantity;

    use super::*;

    fn _init_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app
    }

    #[test]
    fn test_insert_into_basic() {
        let mut app = _init_app();

        let id = app.world_mut().spawn_empty().id();

        app.update();

        let op = InsertInto(Interactable::Mob {
            name: "hello".into(),
            tile_idx: TileIdx::Chicken,
        });

        let mut c = app.world_mut().commands();
        let mut ec = c.entity(id);
        ec.queue(op);

        app.update();

        assert!(
            app.world()
                .get::<TileIdx>(id)
                .is_some_and(|it| it == &TileIdx::Chicken)
        );
        assert!(
            app.world()
                .get::<Name>(id)
                .is_some_and(|it| it.as_str().eq_ignore_ascii_case("hello"))
        );
    }

    #[test]
    fn test_insert_into_with_chest() {
        let mut app = _init_app();

        let id = app.world_mut().spawn_empty().id();

        let op = InsertInto(Interactable::Chest {
            is_open: false,
            contents: Some(Inventory::with_item(ItemId::Gold, Quantity(2))),
            tile_idx: TileIdx::ChestBrownClosed,
        });

        let w = app.world_mut();
        let mut c = w.commands();
        c.entity(id).queue(op);

        app.update();

        assert!(
            app.world_mut()
                .get::<TileIdx>(id)
                .is_some_and(|it| it == &TileIdx::ChestBrownClosed)
        );

        let chest = app
            .world_mut()
            .get::<chest::Chest>(id)
            .expect("expected chest to be present");

        let contents = chest
            .contents
            .clone()
            .expect("expected chest to have contents");

        let q = contents
            .item_quantity(&ItemId::Gold)
            .map(|q| q.0)
            .unwrap_or_default();

        assert_eq!(false, chest.is_open, "expected chest not to be open");
        assert_eq!(q, 2, "expected two gold pieces to be present in chest");
    }
}
