use bevy::prelude::*;
use bevy_northstar::{
    grid::Grid,
    prelude::{AgentOfGrid, AgentPos, Blocking, CardinalNeighborhood},
};

use crate::{
    actors::{Actor, ActorBundle, Player},
    atlas::SpriteAtlas,
    cell::Cell,
    combat::{Belligerent, Health, Parameters},
    fov::Vision,
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
        TileIdx::Skeleton,
        Blocking,
        ActorBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 53, y: 53 },
            ..default()
        },
        Name::new("Mr. Boney"),
        interactions::Interactable::Speaker,
        interactions::Dialogue::phrases(vec!["hello".into(), "hi".into(), "how are you".into()]),
        ChildOf(active_stratum),
    ));

    commands.spawn((
        TileIdx::Skeleton,
        Blocking,
        ActorBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 49, y: 49 },
            ..default()
        },
        Name::new("Mr.Sandbag"),
        Belligerent::new(Parameters {
            health: Health {
                max: 12,
                ..default()
            },
            attack: 0,
            defense: 0,
            vision: Vision(0),
        }),
        ChildOf(active_stratum),
    ));

    let cell = Cell { x: 40, y: 40 };
    commands.spawn((
        Belligerent::new(Parameters {
            health: Health {
                max: 7,
                ..Default::default()
            },
            attack: 1,
            defense: 1,
            vision: Vision::default(),
        }),
        TileIdx::Bat,
        ActorBundle {
            sprite: atlas.sprite(),
            cell,
            ..default()
        },
        Name::new("Bat"),
        AgentPos(cell.into()),
        AgentOfGrid(grid_entity),
        ChildOf(active_stratum),
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
        TileIdx::Torch,
        Blocking,
        ActorBundle {
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
        TileIdx::DoorwayBrownThick,
        ActorBundle {
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
        TileIdx::DoorwayBrownThick,
        ActorBundle {
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
        TileIdx::ChestBrownClosed,
        ActorBundle {
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
