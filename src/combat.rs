use bevy::{prelude::*, sprite::Text2dShadow};
use bevy_northstar::prelude::{AgentOfGrid, Blocking};

use crate::{
    actors::{Actor, Dead, DisplayName},
    colors,
    event_log::MessageLog,
    fov::Vision,
    gamestate::Turn,
    interactions::Interactable,
};

#[derive(Component, Debug, Default)]
pub struct CombatStats {
    pub hp: i32,
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    // This is set by mutable access as the change will be immediate to the
    // system at the moment it occurs. Using just [`Dead`] wouldn't apply it
    // until the system that inserted it had finished running b/c [`Commands`].
    pub is_dead: bool,
}

#[derive(Bundle)]
pub struct Belligerent {
    pub stats: CombatStats,
    pub awareness: Awareness,
    pub vision: Vision,
    pub turn: Turn,
    pub interactable: Interactable,
    pub blocking: Blocking,
    pub actor: Actor,
}

impl Belligerent {
    pub fn new(stats: CombatStats, vision: u32) -> Self {
        Self {
            stats,
            awareness: Awareness::default(),
            vision: Vision(vision),
            turn: Turn::default(),
            interactable: Interactable::Combatant,
            blocking: Blocking::default(),
            actor: Actor::default(),
        }
    }
}

/// Add Awareness if the Actor needs complex behavior related to the Player.
#[derive(Component, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
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

pub fn init_combatants(mut combatants: Query<&mut CombatStats, Added<CombatStats>>) {
    for mut combatant in combatants.iter_mut() {
        combatant.hp = combatant.max_hp;
    }
}

pub fn process_attacks(
    mut commands: Commands,
    mut combatants: Query<(Entity, &DisplayName, &mut CombatStats)>,
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

        let (defender_entity, defender_name, mut defender) = defender;
        let (_, attacker_name, attacker) = attacker;

        if defender.is_dead {
            log.add(
                format!("{} is already dead", defender_name),
                colors::KENNEY_GOLD,
            );
            continue;
        }

        let damage = attacker.attack - defender.defense;
        if damage >= 0 {
            defender.hp = defender.hp.saturating_sub(damage);
            log.add(
                format!("{} hits {}!", attacker_name, defender_name),
                colors::KENNEY_GOLD,
            );

            if defender.hp <= 0 {
                defender.is_dead = true;
                log.add(format!("{} is dead", defender_name), colors::KENNEY_RED);
                spawn_floating_text(
                    &mut commands,
                    colors::KENNEY_RED,
                    &font,
                    defender_entity,
                    "dead",
                );
                commands
                    .entity(defender_entity)
                    .insert(Dead)
                    .remove::<AgentOfGrid>()
                    .remove::<Turn>();
            } else {
                spawn_floating_text(&mut commands, Color::WHITE, &font, defender_entity, damage);
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
