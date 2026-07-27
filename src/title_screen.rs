use bevy::{prelude::*, text::FontSourceTemplate};
use std::time::Duration;

use crate::{
    gamestate::Screen,
    typewriter::{Revealing, Typewriter},
};

pub struct TitleScreenPlugin;

impl Plugin for TitleScreenPlugin {
    fn build(&self, app: &mut App) {
        app.insert_state(Screen::Title)
            .add_systems(Startup, setup)
            .add_systems(OnEnter(Screen::Title), show)
            .add_systems(OnExit(Screen::Title), hide)
            .add_systems(Update, interaction_system.run_if(in_state(Screen::Title)));
    }
}

#[derive(Component, Clone, Default, Debug)]
pub struct TitleScreen;

#[derive(Component, Reflect, Debug)]
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

        ]
    }
}

fn interaction_system(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    interactions: Query<(&Interaction, &Buttons), Changed<Interaction>>,
) {
    for (interaction, button_ty) in interactions.iter() {
        if interaction != &Interaction::Pressed {
            continue;
        }

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
}

fn show(
    screen: Single<Entity, With<TitleScreen>>,
    title: Single<Entity, With<TitleText>>,
    mut commands: Commands,
) {
    info!("showing title screen {:?} {:?}", *screen, *title);
    commands.entity(*screen).insert(Visibility::Inherited);
    commands.entity(*title).insert(Typewriter {
        tint: Color::WHITE,
        per_char: Duration::from_millis(100),
        txt: String::from("ADVENTUREGAME"),
    });
}

fn hide(
    screen: Single<Entity, With<TitleScreen>>,
    title: Single<Entity, With<TitleText>>,
    mut commands: Commands,
) {
    info!("hiding title screen {:?} {:?}", *screen, *title);
    commands.entity(*screen).insert(Visibility::Hidden);

    commands
        .entity(*title)
        .remove::<(Typewriter, Revealing)>()
        .despawn_children();
}
