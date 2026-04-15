use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    colors, combat,
    event_log::MessageLog,
    inventory::{self, *},
    tiles::TileIdx,
};

/// A component representing an interactable object in the world, such as a door or chest, that can be interacted with by actors.
#[derive(Component, Debug, Default, Reflect, Serialize, Deserialize)]
pub enum Interactable {
    #[default]
    Unset,
    Door {
        is_open: bool,
        requires: Option<Item>,
    },
    Chest {
        is_open: bool,
        contents: Inventory,
    },
    Speaker,
    Combatant,
}

impl Interactable {
    pub fn for_tile(tile_idx: TileIdx) -> Option<Interactable> {
        match tile_idx {
            TileIdx::ChestBrownClosed | TileIdx::ChestWhiteClosed => Some(Interactable::Chest {
                is_open: false,
                contents: Inventory::with_item(Item("gold".to_string()), 10),
            }),
            TileIdx::DoorBrownThickClosed1
            | TileIdx::DoorBrownThickClosed2
            | TileIdx::DoorBrownThickClosed3 => Some(Interactable::Door {
                is_open: false,
                requires: None,
            }),
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
                "Interaction attempted with entity {:?}, but it's not interactable.",
                attempt.target
            );
            continue;
        };

        match interactable.as_mut() {
            Interactable::Unset => {
                warn!("interactable found with Unset");
                continue;
            }
            Interactable::Door { is_open, requires } => {
                if !*is_open {
                    if let Some(required_item) = requires {
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
                    tile_idx.set_if_neq(tile_idx.opened_version().unwrap_or(*tile_idx));
                }
            }
            Interactable::Chest { is_open, contents } => {
                if !*is_open {
                    *is_open = true;
                    tile_idx.set_if_neq(tile_idx.opened_version().unwrap_or(*tile_idx));
                    info!("Player opens chest: {:?}", contents);
                    log.add("Opened chest.", colors::KENNEY_BLUE);
                    log.add_all(contents.summary("got").as_ref(), colors::KENNEY_GREEN);
                    acquisitions.write(inventory::Acquisition {
                        items: contents.clone(),
                    });
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

/// Sets up interactable objects in the world, such as doors and chests, based on the tile indices.
///
/// Mostly this means interactables that have such as an open/closed sprite.
pub fn setup(mut commands: Commands, tiles: Query<(Entity, &TileIdx), Added<TileIdx>>) {
    for (entity, &tile_idx) in tiles.iter() {
        if tile_idx.is_interactable() {
            if let Some(bundle) = Interactable::for_tile(tile_idx) {
                info!("{:?} {:?} gets {:?}", entity, tile_idx, bundle);
                commands.entity(entity).insert(bundle);
            } else {
                warn!(
                    "found interactable tile without Interactable: {:?}",
                    tile_idx
                );
            }
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_message::<Listen>().add_message::<Examine>();
}
