use std::ops::Add;

use bevy::prelude::*;

use crate::{
    atlas::SpriteAtlas,
    cell::Cell,
    combat::CombatStats,
    light::{Emitter, LightLevel},
    tilemap::{TileStorage, TilemapLayer, TilemapSpec},
    tiles::{self, MapTile, Occupied, TileIdx},
};

#[derive(Component, Debug)]
/// A marker component for entities that perform actions in the world, such as the player or NPCs.
pub struct Actor;

#[derive(Component, Debug)]
pub struct Player;

#[derive(Component, Debug, Deref)]
pub struct PreviousCell(pub Cell);

#[derive(Bundle, Default, Clone, Debug)]
/// A bundle for map pieces that includes a sprite, cell position, and transform.
pub struct PieceBundle {
    pub sprite: Sprite,
    pub cell: Cell,
    pub transform: Transform,
    pub visibility: Visibility,
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

const ACTOR_LAYER: TilemapLayer = TilemapLayer(-2.0);
const PLAYER_LAYER: TilemapLayer = TilemapLayer(-1.0);

#[derive(EntityEvent, Debug)]
pub struct Moved(pub Entity);

pub fn setup_player(mut commands: Commands, spec: Res<TilemapSpec>, atlas: Res<SpriteAtlas>) {
    commands.spawn((
        Player,
        Actor,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: spec.start,
            transform: Transform::from_xyz(0., 0., *PLAYER_LAYER),
            ..Default::default()
        },
        TileIdx::Player,
        Emitter::new((LightLevel::Bright, 2), (LightLevel::Light, 1)),
        CombatStats {
            nameplate: "Player".into(),
            max_hp: 10,
            attack: 2,
            defense: 1,
            hp: 10,
            ..Default::default()
        },
    ));
}

/// Syncs changed actor [TileIdx] for [Sprite]s `Without<MapTile>`.
pub fn sync_actor_sprites(
    mut pieces: Query<(&mut Sprite, &TileIdx), (Without<MapTile>, Changed<TileIdx>)>,
) {
    for (mut sprite, tile_idx) in pieces.iter_mut() {
        if let Some(texture_atlas) = &mut sprite.texture_atlas {
            texture_atlas.index = tile_idx.into();
        }
    }
}

/// Updates the [Transform] of pieces based on their [Cell] coordinates when the cell changes.
pub fn update_actor_transforms(
    mut pieces: Query<(&Cell, &mut Transform), (With<Actor>, Changed<Cell>)>,
) {
    for (piece_cell, mut transform) in pieces.iter_mut() {
        transform.translation.x = piece_cell.x as f32 * tiles::TILE_SIZE_PX;
        transform.translation.y = piece_cell.y as f32 * tiles::TILE_SIZE_PX;
        transform.translation.z = *ACTOR_LAYER;
    }
}

#[derive(Message, Debug)]
/// A message representing an attempt by an actor to interact with a cell in the world, such as moving into it or interacting with an object on it.
pub struct ActionAttempt {
    pub entity: Entity,
    pub origin_cell: Cell,
    pub target_cell: Cell,
}

/// Handles player input and sends an [ActionAttempt] message derived from player input.
pub fn handle_player_input(
    mut events: MessageWriter<ActionAttempt>,
    input: Res<ButtonInput<KeyCode>>,
    player_query: Query<(Entity, &Cell), With<Player>>,
) {
    let Some(direction) = get_direction(&input) else {
        return;
    };

    let Ok((player_entity, player_cell)) = player_query.single() else {
        warn!("No player entity found in the world.");
        return;
    };

    events.write(ActionAttempt {
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

pub fn sync_occupied_tiles(
    mut commands: Commands,
    actors: Query<(&Cell, Option<&PreviousCell>), (With<Actor>, Changed<Cell>)>,
    storage: Single<&TileStorage>,
) {
    for (curr_cell, prev_cell_opt) in actors.iter() {
        if let Some(tile) = storage.get(curr_cell) {
            commands.entity(tile).insert(Occupied);
        }

        if let Some(prev_cell) = prev_cell_opt
            && let Some(prev_tile) = storage.get(prev_cell)
        {
            commands.entity(prev_tile).remove::<Occupied>();
        }
    }
}
