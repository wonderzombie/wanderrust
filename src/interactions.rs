use bevy::{platform::collections::HashSet, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    actors::{Actor, PieceBundle, Player},
    atlas::SpriteAtlas,
    cell::Cell,
    colors,
    combat::{self, SpawnPoint},
    dialogue_modal::DialogueStart,
    gamestate::PlayerRested,
    inventory::*,
    items::ItemId,
    ldtk_loader::{LdtkActor, LdtkEntity, LdtkEntityExt},
    message_log::LogEvent,
    sounds,
    tilemap::{ActiveLevel, Level, WorldSpec},
    tiles::TileIdx,
};

/// A component representing an interactable object in the world, such as a door
/// or chest, that can be interacted with by actors.
#[derive(Component, Debug, Default, Clone, Reflect, Serialize, Deserialize, Eq, PartialEq)]
#[reflect(Component)]
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

        let name = entity
            .display_name()
            .unwrap_or_else(|| String::from("MISSINGNAME"));
        let tile_idx = entity.get_tile();

        match ty {
            LdtkActor::Combatant => Some(Self::Mob { name, tile_idx }),
            LdtkActor::Speaker => {
                let lines = entity.get_str_array("lines").unwrap_or_default();
                if lines.is_empty() {
                    warn!("found zero lines for speaker: {name} {tile_idx}");
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
                    warn!("empty chest found: {name} {tile_idx}\n{entity:#?}")
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

/// ShrinesVisited tracks all the shrine entities which the player has visited.
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct ShrinesVisited(pub HashSet<Entity>);

/// Examine is a general word for interactions.
#[derive(Message, Debug, Copy, Clone)]
pub struct Examine {
    pub interactor: Entity,
    pub target: Entity,
}

/// Processes [`Examine`] messages, executing the interaction between the player
/// and an [`Interactable`] entity. Interaction fails if the target cell is
/// merely solid. Otherwise interaction depends on the type of [`Interactable`].
pub fn process_interactions(
    mut commands: Commands,
    active_level: Single<Entity, With<ActiveLevel>>,
    mut attempts: MessageReader<Examine>,
    mut interactables: Query<(Entity, &mut TileIdx, &mut Interactable, Option<&Name>)>,
    mut inv_changes: MessageWriter<InventoryChange>,
    mut attacks: MessageWriter<combat::Attack>,
    player_inv: Res<Inventory>,
    player: Single<(Entity, &Cell), With<Player>>,
    mut log: MessageWriter<LogEvent>,
    mut shrines_visited: ResMut<ShrinesVisited>,
) {
    let (player_nt, player_cell) = *player;
    for attempt in attempts.read() {
        let Ok((entity, mut tile_idx, mut interactable, name_opt)) =
            interactables.get_mut(attempt.target)
        else {
            info!(
                "📦 Interaction attempted with entity {:?}, but it's not interactable.",
                attempt.target
            );
            continue;
        };

        trace!("process_interactions: matched interactable: {interactable:#?}",);

        match interactable.as_mut() {
            Interactable::Invalid => {
                error!("invalid interactable; skipping: {attempt:?}");
                continue;
            }
            Interactable::Door {
                is_open,
                requires,
                tile_idx: _,
            } => {
                trace!("process_interactions: door");
                if !*is_open {
                    if let Some(required_item) = requires {
                        let reqd_itam = required_item.def();
                        if !player_inv.has_item(required_item) {
                            info!("Player lacks required item: {reqd_itam}");
                            log.write(("Locked.", colors::KENNEY_BLUE).into());
                            continue;
                        } else {
                            info!("Player opens the door with {reqd_itam}.");
                            log.write(
                                (
                                    format!("Opened door with {reqd_itam}.").as_str(),
                                    colors::KENNEY_BLUE,
                                )
                                    .into(),
                            );
                        }
                    } else {
                        info!("Player opens the door.");
                        log.write(("Opened door.", colors::KENNEY_BLUE).into());
                    }
                    *is_open = true;
                    trace!(
                        "changing tile_idx from {tile_idx:?} to {:?}",
                        tile_idx.engaged_version()
                    );
                    commands.trigger(sounds::Opened);
                    tile_idx.set_if_neq(tile_idx.engaged_version().unwrap_or(*tile_idx));
                } else {
                    info!("Player can't open an open door.");
                }
            }
            Interactable::Chest {
                is_open,
                contents,
                tile_idx: _,
            } => {
                if !*is_open {
                    *is_open = true;
                    tile_idx.set_if_neq(tile_idx.engaged_version().unwrap_or(*tile_idx));
                    info!("Player opens chest: {contents:?}");
                    log.write(("Opened chest.", colors::KENNEY_BLUE).into());
                    commands.trigger(sounds::Opened);
                    if let Some(contents) = contents {
                        inv_changes
                            .write_batch(InventoryChange::acquire(player_nt, contents.clone()));
                        contents.summarized("got").iter().for_each(|it| {
                            log.write((it.as_str(), colors::KENNEY_GREEN).into());
                        });
                    }
                }
            }
            Interactable::Speaker { name, .. } => {
                info!(
                    "Player talks to {}.",
                    name_opt.map_or(name.as_str(), |n| n.as_str())
                );
                commands.trigger(DialogueStart(attempt.target));
            }
            Interactable::Mob { name, .. } => {
                info!("Player attacks {name}.");
                attacks.write(combat::Attack {
                    attacker: attempt.interactor,
                    target: entity,
                });
            }
            Interactable::Shrine { id, .. } => {
                info!("Player interacts with {id}.");
                if shrines_visited.0.contains(&entity) {
                    log.write(LogEvent {
                        txt: format!("rested at shrine {id}"),
                        color: Some(colors::KENNEY_GOLD),
                    });
                    commands.entity(player_nt).insert(SpawnPoint {
                        respawn_cell: *player_cell,
                        level_nt: *active_level,
                    });
                    commands.trigger(PlayerRested);
                    commands.trigger(sounds::Rest);
                } else {
                    shrines_visited.0.insert(entity);
                    log.write(LogEvent {
                        txt: format!("lit shrine {id}"),
                        color: Some(colors::KENNEY_BLUE),
                    });
                    commands.trigger(sounds::LitShrine);
                }
            }
        }
    }
}

/// A component representing the dialogue of an NPC.
///
/// This component is used to store and manage the dialogue of an NPC, including
/// the current phrase and the list of phrases.
#[derive(Component, Debug, Default, Serialize, Deserialize, Reflect)]
#[reflect(Component)]
pub struct Dialogue {
    idx: usize,
    phrases: Vec<String>,
}

impl Dialogue {
    pub fn advance(&mut self) -> Option<&str> {
        match &self.phrases.get(self.idx) {
            Some(phrase) => {
                self.idx = (self.idx + 1) % self.phrases.len();
                Some(phrase)
            }
            _ => None,
        }
    }
}

#[derive(Resource)]
pub struct DialogueEntity(pub Entity);

pub fn detect_speakers(
    mut commands: Commands,
    interactables: Query<(Entity, &Interactable), Added<Interactable>>,
) {
    let mut count = 0;
    for (nt, interx) in interactables {
        match interx {
            Interactable::Speaker { lines, .. } => {
                count += 1;
                commands.entity(nt).insert(Dialogue {
                    idx: 0,
                    phrases: lines.clone(),
                });
            }
            _ => continue,
        };
    }

    if count > 0 {
        info!("detected speakers: {count}");
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
                (
                    Name::new(format!("{} {}", interx.tile(), *cell)),
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
                info!("spawning {}", b.0);
                trace!("spawning {b:?}");
                count += 1;
                commands.spawn(b);
            });

        info!("📦 {level_id:?}: spawned {count} interactables");
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(PreUpdate, detect_speakers)
        .add_message::<Examine>()
        .init_resource::<ShrinesVisited>();
}
