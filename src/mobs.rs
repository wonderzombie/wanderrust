use bevy::prelude::*;
use bevy_northstar::prelude::*;

use crate::{
    actors::{Dead, Player},
    atlas::{self, SpriteAtlas},
    cell::Cell,
    colors,
    combat::{Awareness, Parameters},
    fov::Fov,
    gamestate::{GameState, Turn},
    interactions::Interactable,
    inventory,
    loot::{FixedLoot, LootTable},
    tilemap::{ActiveLevel, Zone},
    tiles::TileIdx,
};

/// Checks each mob's status and alerts mobs when the player enters their FOV.
pub fn check_fov(
    mut commands: Commands,
    active_zone: Single<(&Fov, &Zone), With<ActiveLevel>>,
    active_mobs: Populated<(&Awareness, &Cell, &Parameters), (With<AgentOfGrid>, Without<Dead>)>,
    player_cell: Single<&Cell, With<Player>>,
) {
    let player_cell: (i32, i32) = (*player_cell).into();

    let (fov, entities) = active_zone.into_inner();

    for entity in entities.iter() {
        let Ok((awareness, cell, params)) = active_mobs.get(entity) else {
            continue;
        };

        let view = fov.from(cell.into(), params.vision.range());

        if view.has(player_cell) && awareness < &Awareness::Alerted {
            commands
                .entity(entity)
                .insert(Awareness::Alerted)
                .insert(Turn::Waiting);
        }
    }
}

#[derive(Component, Debug, Default)]
pub(crate) struct Indicator;

pub fn init_indicators(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    atlas: Res<SpriteAtlas>,
    query: Populated<(Entity, &Interactable), Added<Interactable>>,
) {
    let image: Handle<Image> = asset_server.load(atlas::TRANSPARENT_SHEET);
    let sprite = Sprite::from_atlas_image(
        image,
        TextureAtlas {
            layout: atlas.layout.clone(),
            index: TileIdx::Corners.into(),
        },
    );

    let xform = Transform::from_xyz(0., 0., 1.);
    for (nty, interx) in query {
        match interx {
            Interactable::Belligerent { .. } | Interactable::Speaker { .. } => {
                info!("initialized indicator for {nty:?}");
                commands.spawn((
                    Indicator,
                    xform,
                    ChildOf(nty),
                    TileIdx::Corners,
                    sprite.clone(),
                    Visibility::Inherited,
                ));
            }
            _ => continue,
        }
    }
}

pub fn update_mob_indicators(
    mut commands: Commands,
    zone: Single<&Zone, With<ActiveLevel>>,
    mobs: Populated<(&Awareness, Has<Dead>)>,
    indicators: Query<(Entity, &ChildOf, &mut Sprite), With<Indicator>>,
) {
    let active_mob_ntys = zone.collection();
    for (indic_nty, ChildOf(parent), mut sprite) in indicators {
        // TODO: verify that we don't need to hide the indicator explicitly
        // since the parent of the indicator should be hidden along with its
        // parent, the mob entity.
        if let Ok((awareness, is_dead)) = mobs.get(*parent)
            && active_mob_ntys.contains(parent)
        {
            if is_dead {
                commands.entity(indic_nty).despawn();
            } else {
                match awareness {
                    Awareness::Idling => sprite.color = colors::KENNEY_OFF_WHITE,
                    Awareness::Alerted => sprite.color = colors::KENNEY_RED,
                }
            }
        }
    }
}

pub fn handle_dead(
    query: Populated<(Option<&FixedLoot>, Option<&LootTable>), (With<Dead>, With<Turn>)>,
    mut acquisitions: MessageWriter<inventory::Acquisition>,
) {
    for (fixed_loot_opt, loot_opt) in &query {
        let mut acquired = inventory::Inventory::default();

        if let Some(loot) = loot_opt {
            acquired.extend(loot.roll());
        }

        if let Some(FixedLoot(fixed)) = fixed_loot_opt {
            acquired.extend(fixed.clone());
        }

        if !acquired.is_empty() {
            acquisitions.write(inventory::Acquisition { items: acquired });
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PreUpdate, init_indicators)
        .add_systems(OnEnter(GameState::AwaitingInput), update_mob_indicators)
        .add_systems(Last, handle_dead);
}
