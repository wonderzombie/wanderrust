use std::time::Duration;

use bevy::prelude::*;

use crate::{gamestate::Screen, typewriter::Typewriter};

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

#[derive(Component, Debug)]
struct TitleScreen;

#[derive(Component, Reflect, Debug)]
struct TitleText;

/// Set up and show the title screen using Bevy's UI APIs.
pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(screen_bundle(asset_server));
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
enum Buttons {
    Start,
    Intro,
}

fn screen_bundle(asset_server: Res<AssetServer>) -> impl Bundle {
    let font: Handle<Font> = asset_server.load("fonts/pcsenior.ttf");
    // TODO: convert this to bsn!
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
                TitleText,
                Text::new(""),
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(54.0),
                    font_smoothing: FontSmoothing::None,
                    ..default()
                },
                TextLayout::justify(Justify::Center),
                // TODO: we start with this because it's the title screen.
                // However, `show()` should do it for us.
                Typewriter {
                    tint: Color::WHITE,
                    per_char: Duration::from_millis(100),
                    txt: String::from("ADVENTUREGAME"),
                }
            ),
            Node {
                min_height: Val::Px(32.),
                ..default()
            },
            (
                Button,
                Buttons::Start,
                Text::new("START"),
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(33.),
                    ..default()
                },
                TextLayout::justify(Justify::Center),
            ),
            Node {
                min_height: Val::Px(32.),
                ..default()
            },
            (
                Button,
                Buttons::Intro,
                Text::new("INTRO"),
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(33.),
                    ..default()
                },
                TextLayout::justify(Justify::Center),
            )
        ],
    )
}

fn interaction_system(
    mut commands: Commands,
    interactions: Query<(&Interaction, &Buttons), Changed<Interaction>>,
) {
    for (interaction, button_ty) in interactions.iter() {
        if interaction != &Interaction::Pressed {
            continue;
        }

        match button_ty {
            Buttons::Start => {
                commands.set_state_if_neq(Screen::Playing);
            }
            Buttons::Intro => {
                commands.set_state_if_neq(Screen::Intro);
            }
        }
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
        .despawn_children()
        .remove::<Typewriter>();
}
