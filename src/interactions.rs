use bevy::{platform::collections::HashSet, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    actors::{Actor, PieceBundle, Player},
    atlas::SpriteAtlas,
    cell::Cell,
    colors,
    combat::{self, SpawnPoint},
    gamestate::PlayerRested,
    interacticator::InteractCommand,
    interxables::*,
    inventory::*,
    items::ItemId,
    ldtk_loader::{LdtkActor, LdtkEntity, LdtkEntityExt},
    message_log::LogEvent,
    mobs::{self},
    tilemap::{ActiveLevel, Level, WorldSpec},
    tiles::TileIdx,
};

/// A component representing an interactable object in the world, such as a door
/// or chest, that can be interacted with by actors.
#[derive(Component, Debug, Default, Clone, Reflect, Serialize, Deserialize, Eq, PartialEq)]
#[reflect(Component)]
#[require(Actor)]
pub enum Interactable {
    #[default]
    Invalid,
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
            _ => None,
        }
    }

    pub fn tile(&self) -> TileIdx {
        match self {
            Self::Chest { tile_idx, .. }
            | Self::Door { tile_idx, .. }
            | Self::Mob { tile_idx, .. }
            | Self::Speaker { tile_idx, .. }
            | Self::Shrine { tile_idx, .. } => *tile_idx,
            _ => {
                warn!("no tile for interactable: {self:?}");
                TileIdx::GridSquare
            }
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
        match value {
            Interactable::Invalid => (),
            Interactable::Door {
                is_open,
                requires,
                tile_idx,
            } => {
                entity.insert(door::Door {
                    is_open,
                    requires,
                    tile_idx,
                });
            }
            Interactable::Chest {
                is_open,
                contents,
                tile_idx: _,
            } => {
                entity.insert(chest::Chest { is_open, contents });
            }
            Interactable::Speaker {
                name,
                tile_idx: _,
                lines,
            } => {
                entity.insert(speaker::Speaker { name, lines });
            }
            Interactable::Shrine { id, tile_idx: _ } => {
                entity.insert(shrine::Shrine { id });
            }
            Interactable::Mob { name, tile_idx: _ } => {
                entity.insert(mobs::Mob { name: name.clone() });
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

/// Processes [`Examine`] messages, executing the interaction between the player
/// and an [`Interactable`] entity. Interaction fails if the target cell is
/// merely solid. Otherwise interaction depends on the type of [`Interactable`].
pub fn process_interactions(
    mut commands: Commands,
    active_level: Single<Entity, With<ActiveLevel>>,
    mut interactions: MessageReader<Examine>,
    mut interactables: Query<(Entity, &mut Interactable)>,
    mut attacks: MessageWriter<combat::Attack>,
    player: Single<(Entity, &Cell), With<Player>>,
    mut log: MessageWriter<LogEvent>,
    mut shrines_visited: ResMut<ShrinesVisited>,
) {
    let (player_nt, player_cell) = *player;
    for interaction in interactions.read() {
        let Ok((entity, mut interactable)) = interactables.get_mut(interaction.target) else {
            info!(
                "📦 Interaction attempted with entity {}, but it's not interactable.",
                interaction.target
            );
            continue;
        };

        info!(
            "process_interactions: matched interactable: {entity} {:?}",
            interactable.display_name(),
        );

        match interactable.as_mut() {
            Interactable::Invalid => {
                error!("invalid interactable; skipping: {interaction:?}");
                continue;
            }
            Interactable::Door { .. } => {
                commands.interact::<door::Door>(interaction.into());
            }
            Interactable::Chest { .. } => {
                commands.interact::<chest::Chest>(interaction.into());
            }
            Interactable::Speaker { .. } => {
                commands.interact::<speaker::Speaker>(interaction.into());
            }
            Interactable::Mob { name, .. } => {
                info!("Player attacks {name}.");
                attacks.write(combat::Attack {
                    attacker: interaction.actor,
                    target: entity,
                });
            }
            Interactable::Shrine { id, .. } => {
                info!("Player interacts with {id}.");
                if shrines_visited.0.contains(&entity) {
                    log.write(LogEvent {
                        txt: format!("rest at {id}"),
                        color: Some(colors::KENNEY_GOLD),
                    });
                    commands.entity(player_nt).insert(SpawnPoint {
                        respawn_cell: *player_cell,
                        level_nt: *active_level,
                    });
                    commands.trigger(PlayerRested);
                    // commands.trigger(sounds::Rest);
                } else {
                    shrines_visited.0.insert(entity);
                    log.write(LogEvent {
                        txt: format!("lit shrine {id}"),
                        color: Some(colors::KENNEY_BLUE),
                    });
                    // commands.trigger(sounds::LitShrine);
                }
            }
        }
    }
}

#[derive(Bundle, Default, Debug)]
struct InterxBundle {
    act: Actor,
    tile_idx: TileIdx,
    interx: Interactable,
    piece: PieceBundle,
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

        info!("📦 {level_id:?}: spawning interactables");

        let mut count = 0;
        spec.interxs
            .iter()
            .map(|(interx, cell)| {
                let name = interx.display_name().unwrap_or_else(|| format!("{cell}"));
                (
                    Name::new(name),
                    InterxBundle {
                        interx: interx.clone(),
                        tile_idx: interx.tile(),
                        piece: PieceBundle {
                            cell: *cell,
                            sprite: atlas.sprite(),
                            ..default()
                        },
                        ..default()
                    },
                    ChildOf(*level_entity),
                )
            })
            .for_each(|b| {
                let insert_interxable = InsertInto(b.1.interx.clone());
                info!("spawning {}", b.0);
                trace!("spawning {b:?}");
                count += 1;
                commands.spawn(b).queue(insert_interxable);
            });

        info!("📦 {level_id:?}: spawned {count} interactables");
    }
}

pub fn plugin(app: &mut App) {
    app.add_message::<Examine>()
        .init_resource::<ShrinesVisited>();
}
