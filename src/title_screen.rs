use bevy::prelude::*;

use crate::gamestate::{GameState, Screen};

/// Set up and show the title screen using Bevy's UI APIs.
pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(title_screen(asset_server));
}

#[derive(Component, Debug)]
pub struct TitleScreen;

pub fn title_screen(asset_server: Res<AssetServer>) -> impl Bundle {
    let font: Handle<Font> = asset_server.load("fonts/pcsenior.ttf");
    (
        TitleScreen,
        BackgroundColor(Color::BLACK),
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
                Text::new("ADVENTUREGAME"),
                TextFont {
                    font: font.clone(),
                    font_size: 54.0,
                    ..default()
                },
                TextLayout::new_with_justify(Justify::Center),
            ),
            (
                Button,
                Text::new("START"),
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

pub fn system(
    mut commands: Commands,
    interactions: Query<(Entity, &Interaction), Changed<Interaction>>,
) {
    for (_, interaction) in interactions.iter() {
        match interaction {
            Interaction::Pressed => {
                commands.set_state_if_neq(Screen::Playing);
                commands.set_state_if_neq(GameState::AwaitingInput);
            }
            _ => {}
        }
    }
}

/// Despawn the title screen.
pub fn discard(entity: Single<Entity, With<TitleScreen>>, mut commands: Commands) {
    commands.entity(*entity).despawn();
}
