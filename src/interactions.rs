use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    actors::{Actor, PieceBundle},
    atlas::SpriteAtlas,
    colors, combat,
    event_log::MessageLog,
    inventory::{self, *},
    ldtk_loader::{LdtkActor, LdtkEntity, LdtkEntityExt},
    tilemap::{Stratum, WorldSpec},
    tiles::TileIdx,
};

/// A component representing an interactable object in the world, such as a door or chest, that can be interacted with by actors.
#[derive(Component, Debug, Default, Clone, Reflect, Serialize, Deserialize, Eq, PartialEq)]
#[reflect(Component)]
pub enum Interactable {
    Door {
        is_open: bool,
        requires: Option<Item>,
        tile_idx: TileIdx,
    },
    Chest {
        is_open: bool,
        contents: Option<Inventory>,
        tile_idx: TileIdx,
    },
    #[default]
    Speaker,
    Combatant,
}

impl Interactable {
    pub fn default_tile(&self) -> Option<TileIdx> {
        use Interactable::*;

        match self {
            Chest {
                tile_idx: tile_idx_default,
                ..
            }
            | Door {
                tile_idx: tile_idx_default,
                ..
            } => Some(*tile_idx_default),
            _ => None,
        }
    }

    pub fn set_tile(&self, tile_idx: TileIdx) -> Self {
        use Interactable::*;

        match self {
            Chest {
                is_open,
                contents,
                tile_idx: _,
            } => Chest {
                is_open: *is_open,
                contents: contents.clone(),
                tile_idx,
            },
            Door {
                is_open,
                requires,
                tile_idx: _,
            } => Door {
                is_open: *is_open,
                requires: requires.clone(),
                tile_idx,
            },
            _ => self.clone(),
        }
    }
}

impl LdtkEntityExt<Interactable> for Interactable {
    fn from_ldtk(entity: &LdtkEntity) -> Option<Interactable> {
        let Some(ty) = entity.ty() else {
            warn!(
                "📦 unknown interactable type: {:?} on LdtkEntity {:?}",
                entity.ty(),
                entity
            );
            return None;
        };

        let tile_idx = entity.get_tile();

        use Interactable::*;
        match ty {
            LdtkActor::Combatant => Some(Combatant),
            LdtkActor::Speaker => Some(Speaker),
            LdtkActor::Door => {
                let requires = entity.get_string("requires").map(Item);
                let is_open = entity.get_bool("is_open");
                Some(Door {
                    is_open,
                    requires,
                    tile_idx,
                })
            }
            LdtkActor::Chest => {
                let contents = entity
                    .get_str_array("contents")
                    .and_then(Inventory::from_str_array);
                let is_open = entity.get_bool("is_open");
                Some(Chest {
                    is_open,
                    contents,
                    tile_idx,
                })
            }
            _ => None,
        }
    }
}

/// Examine is a general word for interactions.
#[derive(Message, Debug, Copy, Clone)]
pub struct Examine {
    pub interactor: Entity,
    pub target: Entity,
}

/// The player listens to the NPC.
#[derive(Message, Debug, Copy, Clone)]
pub struct Listen {
    pub entity: Entity,
}

/// Processes [`Examine`] messages, executing the interaction between the player
/// and an [`Interactable`] entity. Interaction fails if the target cell is
/// merely solid. Otherwise interaction depends on the type of [`Interactable`].
pub fn process_interactions(
    mut attempts: MessageReader<Examine>,
    mut interactables: Query<(Entity, &mut TileIdx, &mut Interactable, Option<&Name>)>,
    mut acquisitions: MessageWriter<Acquisition>,
    mut attacks: MessageWriter<combat::Attack>,
    mut speech: MessageWriter<Listen>,
    player_inventory: Res<Inventory>,
    mut log: ResMut<MessageLog>,
) {
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

        trace!(
            "process_interactions: matched interactable: {:#?}",
            interactable
        );

        match interactable.as_mut() {
            Interactable::Door {
                is_open,
                requires,
                tile_idx: _,
            } => {
                trace!("process_interactions: door");
                if !*is_open {
                    if let Some(required_item) = requires
                        && !required_item.0.is_empty()
                    {
                        if !player_inventory.has_item(required_item) {
                            info!("Player lacks required item: {}", required_item.0);
                            log.add("Locked.", colors::KENNEY_BLUE);
                            continue;
                        } else {
                            info!("Player opens the door with {:?}.", required_item);
                            log.add(
                                format!("Opened door with {}.", required_item),
                                colors::KENNEY_BLUE,
                            );
                        }
                    } else {
                        info!("Player opens the door.");
                        log.add("Opened door.", colors::KENNEY_BLUE);
                    }
                    *is_open = true;
                    trace!(
                        "changing tile_idx from {:?} to {:?}",
                        tile_idx,
                        tile_idx.opened_version()
                    );
                    tile_idx.set_if_neq(tile_idx.opened_version().unwrap_or(*tile_idx));
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
                    tile_idx.set_if_neq(tile_idx.opened_version().unwrap_or(*tile_idx));
                    info!("Player opens chest: {:?}", contents);
                    log.add("Opened chest.", colors::KENNEY_BLUE);
                    if let Some(contents) = contents {
                        log.add_all(contents.summary("got").as_ref(), colors::KENNEY_GREEN);
                        acquisitions.write(inventory::Acquisition {
                            items: contents.clone(),
                        });
                    }
                }
            }
            Interactable::Speaker => {
                info!(
                    "Player talks to {}.",
                    name_opt.map_or("someone", |n| n.as_str())
                );
                speech.write(Listen { entity });
            }
            Interactable::Combatant => {
                attacks.write(combat::Attack {
                    attacker: attempt.interactor,
                    target: entity,
                });
            }
        }
    }
}

/// A component representing the dialogue of an NPC.
///
/// This component is used to store and manage the dialogue of an NPC, including the current phrase and the list of phrases.
#[derive(Component, Debug, Default, Serialize, Deserialize)]
pub struct Dialogue {
    idx: usize,
    phrases: Vec<String>,
}

impl Dialogue {
    pub fn advance(&mut self) -> &str {
        let phrase = &self.phrases[self.idx];
        self.idx = (self.idx + 1) % self.phrases.len();
        phrase
    }

    pub fn phrases(phrases: Vec<String>) -> Self {
        Self { idx: 0, phrases }
    }
}

/// Processes the dialogue of an NPC when the player listens to it.
pub fn process_dialogue(
    mut speech: MessageReader<Listen>,
    mut log: ResMut<MessageLog>,
    mut dialogues: Query<&mut Dialogue>,
) {
    for attempt in speech.read() {
        let Ok(mut dialogue) = dialogues.get_mut(attempt.entity) else {
            continue;
        };

        log.add(dialogue.advance(), colors::KENNEY_BLUE);
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
    strata: Query<&Stratum>,
) {
    for Stratum(strat_entity, strat_id) in strata.iter() {
        let Some(spec) = world_spec.maps.get(strat_id) else {
            continue;
        };

        info!("📦 {:?}: spawning interactables", strat_id);

        let mut count = 0;
        spec.interxs
            .iter()
            .map(|(interx, cell)| {
                (
                    InterxBundle {
                        interx: interx.clone(),
                        tile_idx: interx.default_tile().unwrap_or(TileIdx::GridSquare),
                        piece: PieceBundle {
                            cell: *cell,
                            sprite: atlas.sprite(),
                            ..default()
                        },
                        ..default()
                    },
                    ChildOf(*strat_entity),
                )
            })
            .for_each(|b| {
                trace!("spawning {:?}", &b);
                count += 1;
                commands.spawn(b);
            });

        info!("📦 {:?} spawned {} interactables", strat_id, count);
    }
}

pub fn plugin(app: &mut App) {
    app.add_message::<Listen>().add_message::<Examine>();
}
