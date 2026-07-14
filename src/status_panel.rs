use bevy::{prelude::*, text::FontSourceTemplate};

use crate::{
    actors::{Flasks, Player},
    cell::Cell,
    colors,
    gamestate::{Screen, WorldClock},
    parameters::Health,
};

pub struct StatusPanelPlugin;

impl Plugin for StatusPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Screen::Playing), setup.run_if(run_once))
            .add_systems(PostUpdate, update_labels.run_if(in_state(Screen::Playing)));
    }
}

#[derive(Component, Copy, Clone, Debug, Default)]
pub struct StatusPanel;

#[derive(Component, Copy, Clone, Debug, FromTemplate)]
pub enum Label {
    #[default]
    None,
    Hp,
    Flasks,
    Ticks,
    Cell,
}

fn setup(mut commands: Commands) {
    commands.spawn_scene(panel_bundle());
}

fn update_labels(
    status: Single<(&Health, &Flasks), With<Player>>,
    mut labels: Query<(&mut Text, &Label)>,
    clock: Res<WorldClock>,
    cell: Single<&Cell, With<Player>>,
) {
    let (health, flasks) = *status;

    for (mut text, label) in labels.iter_mut() {
        match label {
            Label::Hp => text.0 = format!("HP: {}", health.hp),
            Label::Flasks => text.0 = format!("FR: {}", flasks.0),
            Label::Ticks => text.0 = format!("T:  {}", *clock),
            Label::Cell => text.0 = format!("C:  {}", *cell),
            _ => error!("found invalid label: {:?} {:?}", text, label),
        }
    }
}

fn pcsr_font(font_size: i32) -> impl Scene {
    let font = FontSourceTemplate::Handle("fonts/pcsenior.ttf".into());
    bsn! {
        TextFont { font, font_size: px(font_size) }
    }
}

fn panel_bundle() -> impl Scene {
    bsn! {
        StatusPanel
        Visibility::Inherited
        Node {
            width: px(196),
            height: px(600),
            flex_direction: FlexDirection::Column,
            position_type: PositionType::Absolute,
            right: px(800),
            left: px(800 - 196),

        }
        Children [
            (
                Node
                Text::new("WANDERRUST")
                pcsr_font(16)
                TextColor(colors::KENNEY_OFF_WHITE)
            ),
            (
                Node
                Text::new("HP: ??")
                Label::Hp
                pcsr_font(12)
                TextColor(colors::KENNEY_OFF_WHITE)
            ),
            (
                Node
                Label::Flasks
                Text::new("FR: ??")
                pcsr_font(12)
                TextColor(colors::KENNEY_OFF_WHITE)
            ),
            (
                Node
                Label::Ticks
                Text::new("T: ??")
                pcsr_font(12)
                TextColor(colors::KENNEY_OFF_WHITE)
            ),
            (
                Node
                Label::Cell
                Text::new("C: ??")
                pcsr_font(12)
                TextColor(colors::KENNEY_OFF_WHITE)
            ),
        ]
    }
}
