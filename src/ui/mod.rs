use bevy::prelude::*;

use crate::{gamestate::Screen, message_log, status_panel};

pub mod theme;

pub const SIDEBAR_W: f32 = 228.0;

#[derive(Component, Clone, Default)]
pub struct HudRoot;

#[derive(Component, Clone, Default)]
pub struct PlayField;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Playing), setup)
        .add_systems(OnExit(Screen::Playing), discard);
}

fn setup(mut commands: Commands) {
    commands.spawn_scene(hud());
}

fn discard(mut commands: Commands, hud_nt: Single<Entity, With<HudRoot>>) {
    commands.entity(*hud_nt).despawn();
}

fn hud() -> impl Scene {
    bsn! {
        #HudRoot
        HudRoot
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Row,
        }
        Pickable::IGNORE
        Children [
            (PlayField Node { flex_grow: 1.0 } Pickable::IGNORE ),
            (
                Node {
                    width: px(SIDEBAR_W),
                    flex_direction: FlexDirection::Column,
                }
                Children [
                    status_panel::scene(),
                    message_log::scene(),
                ]
            ),
        ]
    }
}
