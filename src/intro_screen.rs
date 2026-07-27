use std::time::Duration;

use bevy::{prelude::*, text::FontSourceTemplate};

use crate::{
    gamestate::{GameState, Screen},
    tiles::Revealed,
    typewriter::{FinishNow, Revealing, Typewriter},
};

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(OnEnter(Screen::Intro), show)
        .add_systems(OnExit(Screen::Intro), hide)
        .add_systems(Update, interaction_system.run_if(in_state(Screen::Intro)));
}

#[derive(Component, Clone, Copy, Debug, Default)]
struct IntroScreen;

#[derive(Component, Clone, Default)]
struct IntroText;

pub fn setup(mut commands: Commands) {
    commands.spawn_scene(screen_bundle());
}

fn show(
    mut commands: Commands,
    screen: Single<Entity, With<IntroScreen>>,
    txt: Single<Entity, With<IntroText>>,
) {
    debug!("showing intro");
    commands.entity(*screen).insert(Visibility::Inherited);
    debug!("added typewriter to {:?}", *txt);
    commands.entity(*txt).insert(Typewriter {
        tint: Color::WHITE,
        per_char: Duration::from_millis(100),
        txt: intro_text(),
    });
}

fn hide(
    mut commands: Commands,
    screen: Single<Entity, With<IntroScreen>>,
    intro: Single<Entity, With<IntroText>>,
) {
    debug!("hiding intro {:?} {:?}", *screen, *intro);
    commands.entity(*screen).insert(Visibility::Hidden);
    commands
        .entity(*intro)
        .remove::<(Typewriter, Revealing)>()
        .despawn_children();
}

fn pcsr_font(font_size: i32) -> impl Scene {
    let font = FontSourceTemplate::Handle("fonts/pcsenior.ttf".into());
    bsn! {
        TextFont {
            font,
            font_size: px(font_size),
            font_smoothing: FontSmoothing::None
        }
    }
}

fn screen_bundle() -> impl Scene {
    bsn! {
        IntroScreen
        BackgroundColor(Color::BLACK)
        Visibility::Hidden
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Row,
        }
        Children [
            Node {
                margin: UiRect::all(px(100))
            }
            IntroText
            pcsr_font(20)
            Text("")
            TextLayout::justify(Justify::Start)
        ]
    }
}

fn intro_text() -> String {
    // The whitespace here is deliberate, creating a natural pause.
    String::from("you don't belong here     \n\n\nand you can't leave").to_ascii_uppercase()
}

fn interaction_system(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    typewriter: Query<(Entity, Has<Revealed>), With<Typewriter>>,
) {
    if !input.any_just_pressed([KeyCode::Escape]) {
        return;
    }

    let Ok((nt, revealed)) = typewriter.single() else {
        return;
    };

    if revealed {
        commands.set_state_if_neq(GameState::AwaitingInput);
        commands.set_state_if_neq(Screen::Title);
    } else {
        commands.trigger(FinishNow(nt));
    }
}
