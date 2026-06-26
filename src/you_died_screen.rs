use bevy::prelude::*;

use crate::gamestate::{GameState, Screen};

pub struct YouDiedScreenPlugin;

impl Plugin for YouDiedScreenPlugin {
    fn build(&self, app: &mut App) {
        app.insert_state(Screen::YouDied)
            .add_systems(OnEnter(Screen::YouDied), setup)
            .add_systems(OnExit(Screen::YouDied), discard)
            .add_systems(Update, interaction_system.run_if(in_state(Screen::YouDied)));
    }
}

#[derive(Component, Debug)]
pub struct YouDiedScreen;

pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(screen_bundle(asset_server));
}

pub fn discard(entity: Single<Entity, With<YouDiedScreen>>, mut commands: Commands) {
    commands.entity(*entity).despawn();
}

pub fn screen_bundle(asset_server: Res<AssetServer>) -> impl Bundle {
    let font: Handle<Font> = asset_server.load("fonts/pcsenior.ttf");
    (
        YouDiedScreen,
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        children![
            (
                Text::new("YOU DIED"),
                TextFont {
                    font: font.clone(),
                    font_size: 54.0,
                    ..default()
                },
                TextLayout::new_with_justify(Justify::Center),
            ),
            Node {
                min_height: Val::Px(32.),
                ..default()
            },
            (
                Button,
                Text::new("RESPAWN"),
                TextFont {
                    font: font.clone(),
                    font_size: 33.0,
                    ..default()
                },
                TextLayout::new_with_justify(Justify::Center),
            )
        ],
    )
}

pub fn interaction_system(
    mut commands: Commands,
    interactions: Query<(Entity, &Interaction), Changed<Interaction>>,
) {
    for (_, interaction) in interactions.iter() {
        if interaction == &Interaction::Pressed {
            commands.set_state_if_neq(Screen::Playing);
            commands.set_state_if_neq(GameState::AwaitingInput);
        }
    }
}
