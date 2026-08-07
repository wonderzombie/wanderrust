use std::fmt::Display;

use bevy::prelude::*;

use crate::{
    atlas::SpriteAtlas,
    cell::{Cell, PreviousCell},
    combat::CombatantBundle,
    equipment::ToggleEquip,
    equipment_menu,
    gamestate::Modal,
    inventory::{CarriedBy, Inventory, InventoryChange},
    inventory_menu,
    items::{ItemId, Quantity},
    light::{Emitter, LightLevel},
    tilemap::{self, ActiveLevel, TileStorage, WorldSpawn},
    tiles::{self, MapTile, Occupied, Revealed, TileIdx},
};

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct Dead;

/// A marker component for entities that perform actions in the world, such as
/// the player or NPCs.
#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Actor;

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct Player;

/// A bundle for map pieces that includes a sprite, cell position, transform,
/// and pickable. Pickable is specific to Bevy's sprite picking system.
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

#[derive(EntityEvent, Debug)]
pub struct Moved(pub Entity);

#[derive(EntityEvent, Debug)]
pub struct Bonk(pub Entity);

const STARTING_EQUIPMENT: &[&ItemId] = &[&ItemId::Rags, &ItemId::Stick];

pub fn starting_items() -> Inventory {
    Inventory::from_str_array(["gold:2", "strange key", "glowing tome", "red salve:3"])
        .unwrap_or_else(Inventory::empty)
}

/// Spawns the player entity at the start position of the tilemap on the
/// player's layer.
pub fn setup_player(
    mut commands: Commands,
    spawn: Single<&WorldSpawn>,
    atlas: Res<SpriteAtlas>,
    player: Option<Single<Entity, With<Player>>>,
    active: Single<Entity, With<ActiveLevel>>,
) {
    let WorldSpawn { level_entity, cell } = *spawn;
    if let Some(entity) = player {
        info!("🕹️ respawning player");
        commands
            .entity(*entity)
            .insert(ChildOf(*level_entity))
            .insert(*cell);
    } else {
        info!("🕹️ spawning player at {cell} {level_entity:?}");

        commands.spawn((
            ChildOf(*active),
            // from ecs
            Name::new("Player"),
            Actor,
            Player,
            TileIdx::Player,
            // from crate::light
            Emitter::new(
                TileIdx::Blank,
                (LightLevel::Bright, 2),
                (LightLevel::Light, 1),
            ),
            Flasks(3),
            // from crate::combat, crate::fov
            CombatantBundle::default(),
            PieceBundle {
                sprite: atlas.sprite(),
                cell: *cell,
                transform: Transform::from_xyz(0., 0., *tilemap::PLAYER_LAYER),
                ..default()
            },
        ));
    }
}

pub fn on_player_added(
    mut commands: Commands,
    player: Single<Entity, Added<Player>>,
    mut inv_changes: MessageWriter<InventoryChange>,
    mut equip_changes: MessageWriter<ToggleEquip>,
) {
    let parent = *player;
    for itam in STARTING_EQUIPMENT.iter() {
        let id = commands
            .spawn((CarriedBy(parent), **itam, Quantity(1)))
            .id();
        equip_changes.write(ToggleEquip {
            target: parent,
            equipment: id,
        });
    }

    // add starting items as well
    inv_changes.write_batch(InventoryChange::acquire(parent, starting_items()));
}

/// Updates the [Transform] of pieces based on their [Cell] coordinates when the
/// cell changes.
pub fn update_transforms(
    mut pieces: Query<(&Cell, &mut Transform, Has<Player>), (Without<MapTile>, Changed<Cell>)>,
) {
    for (piece_cell, mut transform, is_player) in pieces.iter_mut() {
        transform.translation.x = piece_cell.x as f32 * tiles::TILE_SIZE_PX;
        transform.translation.y = piece_cell.y as f32 * tiles::TILE_SIZE_PX;
        transform.translation.z = if is_player {
            *tilemap::PLAYER_LAYER
        } else {
            *tilemap::ACTOR_LAYER
        };
    }
}

/// A message representing an attempt by an actor to interact with a cell in the
/// world, such as moving into it or interacting with an object on it.
#[derive(Resource, Debug, Reflect)]
#[reflect(Resource)]
pub struct Action {
    pub entity: Entity,
    pub origin_cell: Cell,
    pub act: Act,
}

impl Action {
    pub fn adjusted_cell(&self) -> Cell {
        if let Act::Move(dir) = self.act {
            return self.origin_cell + dir;
        }
        self.origin_cell
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Action(entity={:?}, origin_cell={}, act={:?})",
            self.entity, self.origin_cell, self.act
        )
    }
}

#[derive(Resource, Debug, Reflect)]
#[reflect(Component)]
pub enum Act {
    Move(IVec2),
    Pass,
    Flask,
    Attack((Entity, Entity)),
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct Flasks(pub i32);

/// Handles player input and sends an [ActionAttempt] message derived from player input.
pub fn handle_player_input(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    player_query: Single<(Entity, &Cell), With<Player>>,
    modal: Res<State<Modal>>,
) {
    if !input.is_changed() {
        return;
    }

    let (entity, &origin_cell) = *player_query;

    match get_menu_input(&input) {
        Some(menu_act) => match menu_act {
            MenuAct::Inventory => {
                info!("inventory");
                commands.trigger(inventory_menu::ToggleUi);
                return;
            }
            MenuAct::Equipment => {
                info!("equipment");
                commands.trigger(equipment_menu::ToggleUi);
                return;
            }
        },
        None => (),
    }

    if matches!(**modal, Modal::None)
        && let Some(act) = get_action(&input)
    {
        commands.insert_resource(Action {
            entity,
            origin_cell,
            act,
        });
    }
}

pub enum MenuAct {
    Inventory,
    Equipment,
}

fn get_menu_input(input: &ButtonInput<KeyCode>) -> Option<MenuAct> {
    if (input.just_released(KeyCode::KeyI) && input.pressed(KeyCode::ShiftLeft))
        || input.just_released(KeyCode::Tab)
    {
        info!("handle_player_input: toggle inventory");
        return Some(MenuAct::Inventory);
    } else if (input.just_released(KeyCode::KeyO) && input.pressed(KeyCode::ShiftLeft))
        || input.just_released(KeyCode::Backslash)
    {
        return Some(MenuAct::Equipment);
    }

    None
}

fn get_action(input: &ButtonInput<KeyCode>) -> Option<Act> {
    if let Some(dir) = get_direction(input) {
        return Some(Act::Move(dir));
    } else if input.any_just_pressed([KeyCode::KeyP, KeyCode::Space]) {
        return Some(Act::Pass);
    } else if input.any_just_released([KeyCode::KeyF])
        && input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
    {
        return Some(Act::Flask);
    }
    None
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

/// Syncs the [Occupied] component on tiles based on actor positions, adding or
/// removing as needed. An Occupied tile is not visible even under partially
/// transparent sprites.
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
