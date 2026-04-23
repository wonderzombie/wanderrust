use std::fmt::Display;
use std::ops::Add;

use bevy::prelude::*;
use bevy_northstar::prelude::Blocking;

use crate::{
    atlas::SpriteAtlas,
    cell::{Cell, PreviousCell},
    combat::{Belligerent, Health, Parameters},
    fov::Vision,
    light::{Emitter, LightLevel},
    tilemap::{self, Stratum, TileStorage, WorldSpawn},
    tiles::{self, MapTile, Occupied, Revealed, TileIdx},
};

#[derive(Component, Debug)]
pub struct Dead;

/// A marker component for entities that perform actions in the world, such as the player or NPCs.
#[derive(Component, Debug, Default)]
pub struct Actor;

#[derive(Component, Debug, Reflect)]
pub struct Player;

/// A bundle for map pieces that includes a sprite, cell position, transform, and pickable.
/// Pickable is specific to Bevy's sprite picking system.
#[derive(Bundle, Default, Clone, Debug)]
pub struct PieceBundle {
    pub sprite: Sprite,
    pub cell: Cell,
    pub prev_cell: PreviousCell,
    pub transform: Transform,
    pub visibility: Visibility,
    pub pickable: Pickable,
    pub revealed: Revealed,
}

#[derive(Resource, Debug)]
pub struct PlayerStats {
    pub vision_range: u32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self { vision_range: 5 }
    }
}

impl PlayerStats {
    const DEFAULT_VISION: u32 = 5;

    pub fn set_vision_range(&mut self, vision_range: u32) {
        self.vision_range = vision_range;
    }

    pub fn reset_vision_range(&mut self) {
        self.vision_range = PlayerStats::DEFAULT_VISION;
    }

    pub fn is_default(&self) -> bool {
        self.vision_range == PlayerStats::DEFAULT_VISION
    }
}

#[derive(EntityEvent, Debug)]
pub struct Moved(pub Entity);

/// Spawns the player entity at the start position of the tilemap on the player's layer.
pub fn setup_player(
    mut commands: Commands,
    spawn: Single<&WorldSpawn>,
    atlas: Res<SpriteAtlas>,
    player: Option<Single<Entity, With<Player>>>,
    strata: Query<Entity, With<Stratum>>,
) {
    let WorldSpawn { strat_entity, cell } = *spawn;
    if let Some(entity) = player {
        info!("🕹️ respawning player");
        commands
            .entity(*entity)
            .insert(ChildOf(*strat_entity))
            .insert(*cell);
    } else {
        info!("🕹️ spawning player at {:?} {:?}", cell, strat_entity);
        commands.spawn((
            // TODO: figure out the real active stratum.
            ChildOf(strata.iter().next().unwrap()),
            Name::new("Player"),
            Actor,
            Player,
            TileIdx::Player,
            Blocking,
            Emitter::new(
                TileIdx::Blank,
                (LightLevel::Bright, 2),
                (LightLevel::Light, 1),
            ),
            Belligerent {
                params: Parameters {
                    attack: 2,
                    defense: 1,
                    health: Health {
                        hp: 10,
                        max: 10,
                        is_dead: false,
                    },
                    vision: Vision(5),
                },
                ..default()
            },
            PieceBundle {
                sprite: atlas.sprite(),
                cell: *cell,
                transform: Transform::from_xyz(0., 0., *tilemap::PLAYER_LAYER),
                ..default()
            },
        ));
    }
}

/// Updates the [Transform] of pieces based on their [Cell] coordinates when the cell changes.
pub fn update_transforms(
    mut pieces: Query<(&Cell, &mut Transform), (Without<MapTile>, Changed<Cell>)>,
) {
    for (piece_cell, mut transform) in pieces.iter_mut() {
        transform.translation.x = piece_cell.x as f32 * tiles::TILE_SIZE_PX;
        transform.translation.y = piece_cell.y as f32 * tiles::TILE_SIZE_PX;
        transform.translation.z = *tilemap::ACTOR_LAYER;
    }
}

/// A message representing an attempt by an actor to interact with a cell in the world, such as moving into it or interacting with an object on it.
#[derive(Message, Debug)]
pub struct Action {
    pub entity: Entity,
    pub origin_cell: Cell,
    pub target_cell: Cell,
}

impl Action {
    pub fn adjusted_cell(&self) -> Cell {
        self.origin_cell + (self.target_cell - self.origin_cell)
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Action(entity={:?}, origin_cell={}, target_cell={})",
            self.entity, self.origin_cell, self.target_cell
        )
    }
}

/// Handles player input and sends an [ActionAttempt] message derived from player input.
pub fn handle_player_input(
    mut events: MessageWriter<Action>,
    input: Res<ButtonInput<KeyCode>>,
    player_query: Single<(Entity, &Cell), With<Player>>,
) {
    if !input.is_changed() {
        return;
    }
    let Some(direction) = get_direction(&input) else {
        return;
    };

    let (player_entity, player_cell) = *player_query;

    events.write(Action {
        entity: player_entity,
        origin_cell: *player_cell,
        target_cell: player_cell.add(direction),
    });
}

/// Returns the [IVec2] direction implied by [KeyCode], if any.
fn get_direction(input: &ButtonInput<KeyCode>) -> Option<IVec2> {
    let mut direction = IVec2::ZERO;

    if input.just_pressed(KeyCode::KeyW) {
        direction += IVec2::Y;
    }
    if input.just_pressed(KeyCode::KeyS) {
        direction += IVec2::NEG_Y;
    }
    if input.just_pressed(KeyCode::KeyA) {
        direction += IVec2::NEG_X;
    }
    if input.just_pressed(KeyCode::KeyD) {
        direction += IVec2::X;
    }

    if direction != IVec2::ZERO {
        Some(direction)
    } else {
        None
    }
}

/// Syncs the [Occupied] component on tiles based on actor positions, adding or removing as needed.
/// An Occupied tile is not visible even under partially transparent sprites.
pub fn sync_occupied_tiles(
    mut commands: Commands,
    actors: Query<(&Cell, &PreviousCell, &ChildOf), (Without<MapTile>, Changed<Cell>)>,
    storages: Query<&TileStorage>,
) {
    for (curr_cell, prev_cell, child_of) in actors.iter() {
        if let Ok(storage) = storages.get(child_of.parent()) {
            if let Some(tile) = storage.get(curr_cell) {
                commands.entity(tile).insert(Occupied);
            }

            if let Some(prev_tile) = storage.get(prev_cell) {
                commands.entity(prev_tile).remove::<Occupied>();
            }
        }
    }
}
