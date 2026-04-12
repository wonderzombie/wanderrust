use bevy::{ecs::query::QueryData, prelude::*, sprite::Text2dShadow};
use bevy_northstar::prelude::AgentOfGrid;

use crate::{
    actors::Dead, colors, event_log::MessageLog, fov::Vision, gamestate::Turn,
    interactions::Interactable,
};

#[derive(Debug, Default, Copy, Clone)]
pub struct Health {
    pub hp: i32,
    pub max: i32,
    pub is_dead: bool,
}

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Parameters {
    pub attack: i32,
    pub defense: i32,
    pub health: Health,
    pub vision: Vision,
}

pub fn init_combatants(mut combatants: Query<&mut Parameters, Added<Parameters>>) {
    for mut it in combatants.iter_mut() {
        it.health.hp = it.health.max;
    }
}

#[derive(Bundle, Default)]
pub struct Belligerent {
    pub params: Parameters,
    pub awareness: Awareness,
    pub turn: Turn,
    pub interactable: Interactable,
}

impl Belligerent {
    pub fn new(params: Parameters) -> Self {
        Self {
            params,
            interactable: Interactable::Combatant,
            ..default()
        }
    }
}

/// Add Awareness if the Actor needs complex behavior related to the Player.
#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum Awareness {
    // Oblivious,
    #[default]
    Idling,
    // Returning,
    Alerted,
    // Hunting,
}

#[derive(Message, Debug, Copy, Clone)]
pub struct Attack {
    pub attacker: Entity,
    pub target: Entity,
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct CombatantStats {
    pub entity: Entity,
    pub name: &'static Name,
    pub params: &'static mut Parameters,
}

pub fn process_attacks(
    mut commands: Commands,
    mut combatants: Query<(Entity, &Name, &mut Parameters)>,
    mut attacks: MessageReader<Attack>,
    mut log: ResMut<MessageLog>,
    asset_server: Res<AssetServer>,
) {
    let font: Handle<Font> = asset_server.load("fonts/Kenney Mini.ttf");

    for attack in attacks.read() {
        let Ok([attacker, defender]) = combatants.get_many_mut([attack.attacker, attack.target])
        else {
            continue;
        };

        let (defender_id, defender_name, mut defender) = defender;
        let (_, attacker_name, attacker) = attacker;

        if defender.health.is_dead {
            log.add(
                format!("{} is already dead", defender_name),
                colors::KENNEY_GOLD,
            );
            continue;
        }

        let damage = attacker.attack - defender.defense;
        if damage >= 0 {
            defender.health.hp = defender.health.hp.saturating_sub(damage);
            log.add(
                format!("{} hits {}!", attacker_name, defender_name),
                colors::KENNEY_GOLD,
            );

            if defender.health.hp <= 0 {
                defender.health.is_dead = true;
                log.add(format!("{} is dead", defender_name), colors::KENNEY_RED);
                spawn_floating_text(
                    &mut commands,
                    colors::KENNEY_RED,
                    &font,
                    defender_id,
                    "dead",
                );
                commands
                    .entity(defender_id)
                    .insert(Dead)
                    .remove::<AgentOfGrid>()
                    .remove::<Turn>();
            } else {
                spawn_floating_text(&mut commands, Color::WHITE, &font, defender_id, damage);
            }
        } else {
            log.add(
                format!("{} does no damage", attacker_name),
                colors::KENNEY_GOLD,
            )
        }
    }
}

#[derive(Component)]
pub struct FloatingText {
    timer: Timer,
    rise_speed: f32,
}

pub fn spawn_floating_text(
    commands: &mut Commands,
    color: Color,
    font: &Handle<Font>,
    target_entity: Entity,
    amount: impl std::fmt::Display,
) {
    commands.spawn((
        Text2d::new(format!("{amount}")),
        ChildOf(target_entity),
        Transform::from_xyz(8., 8., 0.),
        TextColor(color),
        Text2dShadow {
            offset: Vec2::new(1., -1.),
            ..Default::default()
        },
        FloatingText {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            rise_speed: 32.,
        },
        TextFont {
            font: font.clone(),
            font_size: 12.,
            ..Default::default()
        },
    ));
}

pub fn animate_floating_text(
    mut commands: Commands,
    delta: Res<Time>,
    mut floating_numbers: Query<(
        Entity,
        &mut Transform,
        &mut TextColor,
        &mut Text2dShadow,
        &mut FloatingText,
    )>,
) {
    for (entity, mut transform, mut color, mut shadow, mut text) in floating_numbers.iter_mut() {
        text.timer.tick(delta.delta());
        transform.translation.y += text.rise_speed * delta.delta_secs();

        color.set_alpha(1. - text.timer.fraction());
        shadow.color.set_alpha(1. - text.timer.fraction());

        if text.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
