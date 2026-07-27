use std::time::Duration;

use bevy::{prelude::*, text::FontSourceTemplate};

use crate::{
    gamestate::{GameState, Screen},
    typewriter::{FinishNow, Finished, Typewriter},
};

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Intro), setup)
        .add_systems(OnExit(Screen::Intro), discard)
        .add_systems(Update, interaction_system.run_if(in_state(Screen::Intro)));
}

#[derive(Component, Clone, Copy, Debug, Default)]
struct IntroScreen;

#[derive(Component, Clone, Default)]
struct IntroText;

fn setup(mut commands: Commands) {
    info!("setup");
    commands.spawn_scene(screen_bundle());
}

fn discard(mut commands: Commands, screen: Single<Entity, With<IntroScreen>>) {
    info!("discard");
    commands.entity(*screen).despawn();
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
            Typewriter {
                tint: Color::WHITE,
                per_char: Duration::from_millis(100),
                txt: intro_text(),
            }
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
    typewriter: Query<(Entity, Has<Finished>), With<Typewriter>>,
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
