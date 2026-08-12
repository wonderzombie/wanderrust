use bevy::{platform::collections::HashMap, prelude::*};

use crate::{
    actors::{Flasks, Player},
    cell::Cell,
    colors,
    equipment::{EquippedBy, HasEquipped, unwrap_collection},
    gamestate::{Screen, WorldClock},
    inventory::Inventory,
    items::{ItemId, Slot},
    parameters::{Health, Parameters},
    ui::theme,
};

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (update_labels, update_health_color).run_if(in_state(Screen::Playing)),
    );
}

#[derive(Component, Copy, Clone, Debug, Default)]
pub struct StatusPanel;

#[derive(Component, Copy, Clone, Debug, FromTemplate, PartialEq)]
pub enum Label {
    #[default]
    Hp,
    Flasks,
    Ticks,
    Cell,
    Move,
    Attack,
    Defense,
    MainHand,
    OffHand,
    Armor,
    Trinket,
    Gold,
}

fn update_labels(
    status: Single<(&Cell, &Health, &Flasks, &Parameters, Option<&HasEquipped>), With<Player>>,
    equipment: Query<(&ItemId, &EquippedBy)>,
    inv: Res<Inventory>,
    mut labels: Query<(&mut Text, &Label)>,
    clock: Res<WorldClock>,
) {
    let (cell, health, flasks, params, equipment_opt) = *status;

    let equip_nt: Vec<Entity> = unwrap_collection(equipment_opt);
    let equipped = equipment
        .iter_many(equip_nt)
        .map(|(it, eq_by)| (eq_by.slot, it.def().label))
        .collect::<HashMap<Slot, &str>>();
    let eq_or_none = |s: Slot| equipped.get(&s).unwrap_or_else(|| &"--").to_uppercase();

    let g = inv
        .item_quantity(&ItemId::Gold)
        .map(|it| it.0)
        .unwrap_or_default();

    for (mut text, label) in labels.iter_mut() {
        let new_text = match label {
            Label::Hp => format!("HP: {}", health.hp),
            Label::Flasks => format!("FR: {}", flasks.0),
            Label::Ticks => format!("T:  {}", *clock),
            Label::Cell => format!("C:  {}", *cell),
            Label::Move => format!("M: S{}", params.move_speed),
            Label::Attack => format!("A: {} S{}", params.attack, params.attack_speed),
            Label::Defense => format!("D: {}", params.defense),
            Label::MainHand => format!("+ {}", eq_or_none(Slot::MainHand)),
            Label::OffHand => format!("+ {}", eq_or_none(Slot::OffHand)),
            Label::Armor => format!("+ {}", eq_or_none(Slot::Armor)),
            Label::Trinket => format!("+ {}", eq_or_none(Slot::Trinket)),
            Label::Gold => format!("{} GP", g),
        };
        text.set_if_neq(Text::new(new_text));
    }
}

fn update_health_color(
    mut commands: Commands,
    health: Single<(&Parameters, Ref<Health>), With<Player>>,
    labels: Query<(Entity, &Label)>,
) {
    let Some((entity, _)) = labels.iter().find(|(_, label)| **label == Label::Hp) else {
        warn!("unable to find health label");
        return;
    };

    let (params, health) = *health;

    let pct = health.hp as f32 / params.max_hp as f32;

    let color = if pct < 0.5 {
        colors::KENNEY_RED
    } else {
        colors::KENNEY_OFF_WHITE
    };

    commands.entity(entity).insert(TextColor(color));
}

pub(crate) fn scene() -> impl Scene {
    bsn! {
        #StatusPanel
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
                Node { height: px(16) }
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
                Node { height: px(16) }
            ,
                theme::label_row("")
                Label::Move
            ,
                theme::label_row("")
                Label::Attack
            ,
                theme::label_row("")
                Label::Defense
            ,
                Node { height: px(16) }
            ,
                theme::label_row("")
                Label::MainHand
            ,
                theme::label_row("")
                Label::OffHand
            ,
                theme::label_row("")
                Label::Armor
            ,
                theme::label_row("")
                Label::Trinket
            ,
                Node { height: px(16) }
            ,
                theme::label_row("?? GP")
                Label::Gold
        ]
    }
}
