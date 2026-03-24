use bevy::prelude::*;

use crate::{
    colors, combat,
    event_log::MessageLog,
    inventory::*,
    tiles::{MapTile, TileIdx},
};

#[derive(Component, Debug)]
/// A component representing an interactable object in the world, such as a door or chest, that can be interacted with by actors.
pub enum Interactable {
    Door {
        is_open: bool,
        requires: Option<Item>,
    },
    Chest {
        is_open: bool,
        contents: Inventory,
    },
    Speaker {
        nameplate: String,
    },
    Combatant,
}

#[derive(Message, Debug, Copy, Clone)]
pub struct InteractionAttempt {
    pub interactor: Entity,
    pub target: Entity,
}

#[derive(Message, Debug, Copy, Clone)]
pub struct DialogueAttempt {
    pub entity: Entity,
}

/// Processes [InteractionAttempt] messages, executing the interaction between the player and an [Interactable] entity.
pub fn process_interactions(
    mut attempts: MessageReader<InteractionAttempt>,
    mut interactables: Query<(Entity, &mut TileIdx, &mut Interactable)>,
    mut acquisitions: MessageWriter<Acquisition>,
    mut attacks: MessageWriter<combat::AttackAttempt>,
    mut speech: MessageWriter<DialogueAttempt>,
    player_inventory: Res<Inventory>,
    mut log: ResMut<MessageLog>,
) {
    for attempt in attempts.read() {
        let Ok((entity, mut tile_idx, mut interactable)) = interactables.get_mut(attempt.target)
        else {
            info!(
                "Interaction attempted with entity {:?}, but it's not interactable.",
                attempt.target
            );
            continue;
        };

        match interactable.as_mut() {
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
                    acquisitions.write(Acquisition {
                        items: contents.clone(),
                    });
                }
            }
            Interactable::Speaker { nameplate, .. } => {
                info!("Player talks to {}.", nameplate);
                speech.write(DialogueAttempt { entity });
            }
            Interactable::Combatant => {
                attacks.write(combat::AttackAttempt {
                    attacker: attempt.interactor,
                    target: entity,
                });
            }
        }
    }
}

#[derive(Component, Debug, Default)]
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

pub fn process_dialogue(
    mut speech: MessageReader<DialogueAttempt>,
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

pub fn setup_interactables(
    mut commands: Commands,
    tiles: Query<(Entity, &TileIdx), With<MapTile>>,
) {
    for (entity, tile_idx) in tiles.iter() {
        if !tile_idx.is_interactable() {
            continue;
        }

        let bundle = match tile_idx {
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
        };

        if let Some(bundle) = bundle {
            commands.entity(entity).insert(bundle);
        }
    }
}
