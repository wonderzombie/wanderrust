use bevy::{prelude::*, text::FontSourceTemplate};
use std::time::Duration;

use crate::{colors, debug::DebugState, gamestate::Screen, typewriter::Typewriter};

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

#[derive(Component, Clone, Reflect, Debug, Default)]
struct TitleText;

/// Set up and show the title screen using Bevy's UI APIs.
fn pcsr_font(font_size: i32) -> impl Scene {
    let font = FontSourceTemplate::Handle("fonts/pcsenior.ttf".into());
    bsn! {
        TextFont { font, font_size: px(font_size), font_smoothing: FontSmoothing::None }
    }
}

pub fn setup(mut commands: Commands) {
    commands.spawn_scene(screen_bundle());
}

fn discard(mut commands: Commands, screen: Single<Entity, With<TitleScreen>>) {
    info!("discard");
    commands.entity(*screen).despawn();
}

#[derive(Component, Reflect, Debug, Clone, FromTemplate)]
#[reflect(Component)]
enum Buttons {
    #[default]
    Start,
    Intro,
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
            TitleText
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

            // TODO: justify center
            Button
            Buttons::Start
            Text("[ START ]")
            pcsr_font(33),

            // TODO: justify center
            Button
            Buttons::Intro
            Text("[ INTRO ]")
            pcsr_font(33),

            ColorTest
            Text("")
            pcsr_font(33),

        ]
    }
}

#[derive(Component, Debug, Clone, Default)]
struct ColorTest;

#[derive(Component, Debug, Clone, Default)]
struct TextTest;

fn interaction_system(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    interactions: Query<(&Interaction, &Buttons), Changed<Interaction>>,
    ct: Single<Entity, With<ColorTest>>,
    debug_mode: Res<State<DebugState>>,
) {
    for (_, button_ty) in interactions.iter() {
        let quick_skip =
            input.is_changed() && input.any_just_released([KeyCode::Space, KeyCode::Enter]);

        if quick_skip || matches!(button_ty, Buttons::Start) {
            commands.set_state_if_neq(Screen::Playing);
            return;
        } else if matches!(button_ty, Buttons::Intro) {
            commands.set_state_if_neq(Screen::Intro);
            return;
        };
    }

    if matches!(**debug_mode, DebugState::Disabled) {
        return;
    }

    if input.all_just_pressed([KeyCode::KeyC, KeyCode::AltRight]) {
        info!("spawning color test");
        let scenes = colors::Ramp::kenney_test()
            .iter()
            .map(|&c| {
                bsn! {
                        TextTest
                        Text("[COLOR]")
                        pcsr_font(33)
                        TextColor(c)
                        ChildOf({ *ct })
                }
            })
            .collect::<Vec<_>>();

        commands.spawn_scene_list(scenes);
    } else if input.all_just_pressed([KeyCode::KeyR, KeyCode::AltRight]) {
        info!("spawning color test");
        let bundles = colors::Ramp::fade_out()
            .iter()
            .map(|&c| {
                bsn! {
                        TextTest
                        Text("[COLOR]")
                        pcsr_font(33)
                        TextColor(c)
                        ChildOf({ *ct })
                }
            })
            .collect::<Vec<_>>();

        commands.spawn_scene_list(bundles);
    }
}
