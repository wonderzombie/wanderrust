use bevy::prelude::*;
use bevy_northstar::{
    grid::Grid,
    prelude::{AgentOfGrid, AgentPos, Blocking, CardinalNeighborhood},
};

use crate::{
    actors::{Actor, DisplayName, PieceBundle, Player},
    atlas::SpriteAtlas,
    cell::Cell,
    combat::{Belligerent, CombatStats},
    fov::Vision,
    gamestate,
    interactions::{self, Interactable},
    inventory::{Inventory, Item},
    light::{Emitter, LightLevel},
    tilemap::Portal,
    tiles::TileIdx,
};

pub(crate) fn add_test_mobs(
    mut commands: Commands,
    atlas: Res<SpriteAtlas>,
    grid_entity: Query<Entity, With<Grid<CardinalNeighborhood>>>,
    player_stratum: Query<&ChildOf, With<Player>>,
) {
    // Test/Demo only code.
    let active_stratum = player_stratum.single().unwrap().parent();
    let grid_entity = grid_entity.single().unwrap();

    commands.spawn((
        Actor,
        TileIdx::Skeleton,
        Blocking,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 53, y: 53 },
            ..default()
        },
        DisplayName("Mr. Boney".into()),
        interactions::Interactable::Speaker,
        interactions::Dialogue::phrases(vec!["hello".into(), "hi".into(), "how are you".into()]),
        ChildOf(active_stratum),
    ));

    commands.spawn((
        Actor,
        TileIdx::Skeleton,
        Blocking,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 49, y: 49 },
            ..default()
        },
        DisplayName("Mr. Sandbag".into()),
        interactions::Interactable::Combatant,
        CombatStats {
            max_hp: 12,
            ..default()
        },
        Vision(5),
        ChildOf(active_stratum),
    ));

    let cell = Cell { x: 40, y: 40 };
    commands.spawn((
        Actor,
        TileIdx::Bat,
        Blocking,
        PieceBundle {
            sprite: atlas.sprite(),
            cell,
            ..default()
        },
        DisplayName("Bat".into()),
        interactions::Interactable::Combatant,
        CombatStats {
            max_hp: 4,
            ..default()
        },
        Vision(3),
        AgentPos(cell.into()),
        AgentOfGrid(grid_entity),
        ChildOf(active_stratum),
        gamestate::Turn::Idling,
    ));
}

pub(crate) fn add_test_emitters(
    mut commands: Commands,
    atlas: Res<SpriteAtlas>,
    player_stratum: Query<&ChildOf, With<Player>>,
) {
    // Test/Demo only code.
    let active_stratum = player_stratum.single().unwrap().parent();
    commands.spawn((
        Actor,
        TileIdx::Torch,
        Blocking,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 50, y: 48 },
            ..default()
        },
        Emitter::new((LightLevel::Light, 1), (LightLevel::Dim, 1)),
        ChildOf(active_stratum),
    ));
}

pub(crate) fn add_test_portals(
    mut commands: Commands,
    atlas: Res<SpriteAtlas>,
    player_stratum: Query<&ChildOf, With<Player>>,
) {
    // Test/Demo only code.
    let active_stratum = player_stratum.single().unwrap().parent();

    commands.spawn((
        Actor,
        TileIdx::DoorwayBrownThick,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 50, y: 45 },
            ..default()
        },
        Portal {
            id: "door_exit".into(),
            arrive_at: "door_entry".into(),
        },
        ChildOf(active_stratum),
    ));

    commands.spawn((
        Actor,
        TileIdx::DoorwayBrownThick,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 48, y: 48 },
            ..default()
        },
        Portal {
            id: "door_entry".into(),
            arrive_at: "door_exit".into(),
        },
        ChildOf(active_stratum),
    ));
}

pub(crate) fn add_test_chests(
    mut commands: Commands,
    atlas: Res<SpriteAtlas>,
    player_stratum: Query<&ChildOf, With<Player>>,
) {
    // Test/Demo only code.
    let active_stratum = player_stratum.single().unwrap().parent();

    commands.spawn((
        Actor,
        TileIdx::ChestBrownClosed,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 51, y: 48 },
            ..default()
        },
        Interactable::Chest {
            is_open: false,
            contents: Inventory::with_item(Item::from("gold"), 15),
        },
        ChildOf(active_stratum),
    ));
}
