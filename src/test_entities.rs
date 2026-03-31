use bevy::prelude::*;
use bevy_northstar::{
    grid::Grid,
    prelude::{AgentOfGrid, AgentPos, Blocking, CardinalNeighborhood},
};

use crate::{
    actors::{Actor, PieceBundle},
    atlas::SpriteAtlas,
    cell::Cell,
    combat::CombatStats,
    fov::Vision,
    gamestate, interactions,
    light::{Emitter, LightLevel},
    tilemap::Portal,
    tiles::TileIdx,
};

pub(crate) fn add_test_npc(
    mut commands: Commands,
    atlas: Res<SpriteAtlas>,
    grid_entity: Single<Entity, With<Grid<CardinalNeighborhood>>>,
) {
    commands.spawn((
        Actor,
        TileIdx::Skeleton,
        Blocking,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 53, y: 53 },
            ..default()
        },
        interactions::Interactable::Speaker {
            nameplate: "Mr. Boney".into(),
        },
        interactions::Dialogue::phrases(vec!["hello".into(), "hi".into(), "how are you".into()]),
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
        interactions::Interactable::Combatant,
        CombatStats {
            nameplate: "Mr. Sandbag".into(),
            max_hp: 12,
            ..default()
        },
        Vision(5),
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
        interactions::Interactable::Combatant,
        CombatStats {
            nameplate: "Bat".into(),
            max_hp: 4,
            ..default()
        },
        Vision(3),
        AgentPos(cell.into()),
        AgentOfGrid(*grid_entity),
        gamestate::Turn::Idling,
    ));
}

pub(crate) fn add_test_emitters(mut commands: Commands, atlas: Res<SpriteAtlas>) {
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
    ));
}

pub(crate) fn add_test_portals(mut commands: Commands, atlas: Res<SpriteAtlas>) {
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
    ));
}
