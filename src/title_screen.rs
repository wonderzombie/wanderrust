use bevy::{prelude::*, text::FontSourceTemplate};
use std::time::Duration;

use crate::{
    gamestate::{GameState, Screen},
    typewriter::Typewriter,
};

pub struct TitleScreenPlugin;

impl Plugin for TitleScreenPlugin {
    fn build(&self, app: &mut App) {
        app.insert_state(Screen::Title)
            .add_systems(OnEnter(Screen::Title), setup)
            .add_systems(OnExit(Screen::Title), discard)
            .add_systems(Update, interaction_system.run_if(in_state(Screen::Title)));
    }
}

#[derive(Component, Clone, Default, Debug)]
pub struct TitleScreen;

/// Set up and show the title screen using Bevy's UI APIs.
pub fn setup(mut commands: Commands) {
    commands.spawn_scene(screen_bundle());
}

fn pcsr_font(font_size: i32) -> impl Scene {
    let font = FontSourceTemplate::Handle("fonts/pcsenior.ttf".into());
    bsn! {
        TextFont { font, font_size: px(font_size), font_smoothing: FontSmoothing::None }
    }
}

pub fn screen_bundle() -> impl Scene {
    bsn! {
        TitleScreen
        BackgroundColor(Color::BLACK)
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        Children [
            pcsr_font(54)
            Text("")
            TextLayout::justify(Justify::Center)
            Typewriter {
                tint: Color::WHITE,
                per_char: Duration::from_millis(100),
                txt: String::from("ADVENTUREGAME"),
            }
            ,
            Node {
                min_height: px(32)
            },

            Button
            Text("[ START ]")
            pcsr_font(33)
        ]
    }
}

pub fn interaction_system(
    mut commands: Commands,
    interactions: Query<&Interaction, Changed<Interaction>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    let mut go_play = false;

    for interaction in interactions.iter() {
        match interaction {
            Interaction::Pressed => go_play = true,
            _ => (),
        }
    }

    go_play |= input.is_changed() && input.any_just_released([KeyCode::Space, KeyCode::Enter]);

    if go_play {
        commands.set_state_if_neq(Screen::Playing);
        commands.set_state_if_neq(GameState::AwaitingInput);
    }
}

/// Despawn the title screen.
pub fn discard(entity: Single<Entity, With<TitleScreen>>, mut commands: Commands) {
    commands.entity(*entity).despawn();
}
