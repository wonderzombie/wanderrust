use std::ops::Add;

use bevy::{prelude::*, text::FontSourceTemplate};

use crate::{colors, gamestate::Screen};

pub struct MessageLogPlugin;

impl Plugin for MessageLogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Screen::Playing), setup.run_if(run_once))
            .add_systems(PostUpdate, update_log.run_if(in_state(Screen::Playing)));
    }
}

#[derive(Component, Copy, Clone, Debug, Default)]
pub struct MessageLog;

fn setup(mut commands: Commands) {
    commands.spawn_scene(log_bundle());
}

fn update_log(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    scrolling_log: Single<(Entity, &Node, &Children, &mut ScrollPosition), With<MessageLog>>,
) {
    if input.just_pressed(KeyCode::F7) {
        let (log_nt, _, _, mut scroll_pos) = scrolling_log.into_inner();

        info!("adding a message");

        let nt = commands
            .spawn_scene(bsn! {
                Node
                pcsr_font(12)
                Text::new("and away we go")
                TextColor(colors::KENNEY_OFF_WHITE)
            })
            .id();

        commands.entity(log_nt).add_child(nt);

        let scroll_amt = Vec2 { x: 0., y: 12. };
        *scroll_pos = scroll_pos.add(scroll_amt).into();
    } else if input.just_pressed(KeyCode::F8) {
        let (log_nt, node, children, mut scroll_pos) = scrolling_log.into_inner();

        info!("adding a message");

        let options = textwrap::Options::new(26).initial_indent("• ");

        let out = if input.pressed(KeyCode::SuperLeft) {
            "ABCDEFGHIJLKMNOPQRSTUVWXYZ01234567890"
        } else if input.pressed(KeyCode::ShiftLeft) {
            "ABCDEFGHIJLKMNOPQRSTUVWXYZ"
        } else {
            "Jubilant griffons vexed the wizard king’s phlegmatic quest."
        };

        let wrapped = textwrap::fill(out, options);
        let n = (1 + wrapped.bytes().filter(|&s| s == b'\n').count()) * 16;

        let nt = commands
            .spawn_scene(bsn! {
                Node
                pcsr_font(12)
                Text::new(wrapped)
                TextColor(colors::KENNEY_OFF_WHITE)
            })
            .id();

        commands.entity(log_nt).add_child(nt);
        let scroll_amt = Vec2 { x: 0., y: n as f32 };
        *scroll_pos = scroll_pos.add(scroll_amt).into();
    }
}

fn pcsr_font(font_size: i32) -> impl Scene {
    let font = FontSourceTemplate::Handle("fonts/pcsenior.ttf".into());
    bsn! {
        TextFont { font, font_size: px(font_size) }
    }
}

fn log_bundle() -> impl Scene {
    bsn! {
        MessageLog
        Visibility::Inherited
        Node {
            width: px(384),
            height: px(160),
            top: percent(70),
            right: percent(100),
            left: percent(60),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::scroll_y(),
        }
        Children [
                (
                    Node
                    Text::new("welcome to wanderrust.") pcsr_font(12)
                ),

                (
                    Node
                    Text::new("stay a while.") pcsr_font(12)
                ),

                (
                    Node
                    Text::new("stay forever!") pcsr_font(12)
                ),
        ]


    }
}
