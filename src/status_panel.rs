use bevy::prelude::*;

use crate::{
    actors::{Flasks, Player},
    cell::Cell,
    colors,
    gamestate::{Screen, WorldClock},
    parameters::Health,
    ui::theme,
};

pub struct StatusPanelPlugin;

impl Plugin for StatusPanelPlugin {
    fn build(&self, app: &mut App) {
        app
            // .add_systems(OnEnter(Screen::Playing), setup)
            // .add_systems(OnExit(Screen::Playing), discard)
            .add_systems(Update, update_labels.run_if(in_state(Screen::Playing)));
    }
}

#[derive(Component, Copy, Clone, Debug, Default)]
pub struct StatusPanel;

#[derive(Component, Copy, Clone, Debug, FromTemplate)]
pub enum Label {
    #[default]
    Hp,
    Flasks,
    Ticks,
    Cell,
}

fn update_labels(
    status: Single<(&Cell, &Health, &Flasks), With<Player>>,
    mut labels: Query<(&mut Text, &Label)>,
    clock: Res<WorldClock>,
) {
    let (cell, health, flasks) = *status;

    for (mut text, label) in labels.iter_mut() {
        let new_text = match label {
            Label::Hp => format!("HP: {}", health.hp),
            Label::Flasks => format!("FR: {}", flasks.0),
            Label::Ticks => format!("T:  {}", *clock),
            Label::Cell => format!("C:  {}", *cell),
        };
        text.set_if_neq(Text::new(new_text));
    }
}

pub(crate) fn scene() -> impl Scene {
    bsn! {
        StatusPanel
        Visibility::Inherited
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(8)),
        }
        BackgroundColor(colors::KENNEY_BG)
        BorderColor::all(colors::KENNEY_OFF_WHITE)
        Children [
                Node
                Text::new("WANDERRUST")
                theme::pcsr_font(16)
                TextColor(colors::KENNEY_OFF_WHITE)
            ,
                theme::label_row("HP: ??")
                Label::Hp
            ,
                theme::label_row("FR: ??")
                Label::Flasks
            ,
                theme::label_row("T: ??")
                Label::Ticks
            ,
                theme::label_row("C: ??")
                Label::Cell
            ,
        ]
    }
}
