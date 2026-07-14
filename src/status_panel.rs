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

#[derive(Component, Copy, Clone, Debug, Default)]
pub struct HpLabel;

#[derive(Component, Copy, Clone, Debug, Default)]
pub struct FlasksLabel;

#[derive(Component, Copy, Clone, Debug, Default)]
pub struct TicksLabel;

#[derive(Component, Copy, Clone, Debug, Default)]
pub struct CellLabel;

fn setup(mut commands: Commands) {
    commands.spawn_scene(panel_bundle());
}

fn update_labels(
    status: Single<(&Health, &Flasks), With<Player>>,
    mut labels: ParamSet<(
        Single<&mut Text, With<HpLabel>>,
        Single<&mut Text, With<FlasksLabel>>,
        Single<&mut Text, With<TicksLabel>>,
        Single<&mut Text, With<CellLabel>>,
    )>,
    clock: Res<WorldClock>,
    cell: Single<&Cell, With<Player>>,
) {
    let (health, flasks) = *status;

    // TODO: consider replacing FooLabel markers with newtypes over Text.
    labels.p0().0 = format!("HP: {}", health.hp);
    labels.p1().0 = format!("FR: {}", flasks.0);
    labels.p2().0 = format!("T: {}", *clock);
    labels.p3().0 = format!("C: {}", *cell);
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
                HpLabel
                pcsr_font(12)
                TextColor(colors::KENNEY_OFF_WHITE)
            ),
            (
                Node
                FlasksLabel
                Text::new("FR: ??")
                pcsr_font(12)
                TextColor(colors::KENNEY_OFF_WHITE)
            ),
            (
                Node
                TicksLabel
                Text::new("T: ??")
                pcsr_font(12)
                TextColor(colors::KENNEY_OFF_WHITE)
            ),
            (
                Node
                CellLabel
                Text::new("C: ??")
                pcsr_font(12)
                TextColor(colors::KENNEY_OFF_WHITE)
            ),
        ]
    }
}
