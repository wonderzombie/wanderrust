use bevy::{ecs::query::QueryData, prelude::*, sprite::Text2dShadow};
use bevy_northstar::prelude::AgentOfGrid;

use crate::{
    actors::Dead, bestiary::Bestiary, colors, event_log::MessageLog, gamestate::Turn,
    interactions::Interactable, parameters::*, tiles::TileIdx,
};

#[derive(EntityEvent, Debug)]
pub(crate) struct Attacked(pub Entity);

#[derive(EntityEvent, Debug)]
pub(crate) struct Hit(pub Entity);

#[derive(EntityEvent, Debug)]
pub(crate) struct Died(pub Entity);

pub fn detect_belligerents(
    mut commands: Commands,
    interxs: Populated<(Entity, &Interactable), Added<Interactable>>,
) {
    for (entity, interx) in interxs {
        match interx {
            Interactable::Belligerent { .. } => {
                commands.entity(entity).insert(Combatant);
            }
            _ => continue,
        }
    }
}

pub fn init_combatants(
    mut commands: Commands,
    combatants: Populated<
        (Entity, Option<&Name>, &TileIdx),
        (Added<Combatant>, Without<Parameters>),
    >,
) {
    for (entity, name_opt, tile_idx) in combatants.into_iter() {
        let name = name_opt.map(|it| it.as_str()).unwrap_or("default");
        let params_opt = Bestiary::from_tile(tile_idx).or(Bestiary::from_name(name));

        // Do not skip adding parameters; instead, add a default and log an error.
        // This will keep this function from running repeatedly and doing nothing.
        if params_opt.is_none() || params_opt.is_some_and(|it| it.is_default()) {
            error!(
                "no parameters exist for entity {:?} named {} with tile {:?}; using {:#?}",
                entity, name, tile_idx, params_opt
            );
        }

        let params = params_opt.unwrap_or_default();
        let health = Health {
            hp: params.max_hp.cast_signed(),
            is_dead: false,
        };

        commands
            .entity(entity)
            .insert(CombatantBundle {
                params,
                ..default()
            })
            .insert(health);
    }
}

#[derive(Component, Default, Reflect)]
pub struct Combatant;

#[derive(Bundle, Default)]
pub struct CombatantBundle {
    pub combatant: Combatant,
    pub params: Parameters,
    pub awareness: Awareness,
    pub turn: Turn,
}

#[derive(Message, Debug, Copy, Clone, Reflect)]
pub struct Attack {
    pub attacker: Entity,
    pub target: Entity,
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct CombatantStats {
    pub entity: Entity,
    pub name: &'static Name,
    pub params: &'static Parameters,
    pub health: &'static mut Health,
}

pub fn process_attacks(
    mut commands: Commands,
    mut combatants: Query<(Entity, Option<&Name>, &Parameters, &mut Health)>,
    mut attacks: MessageReader<Attack>,
    mut log: ResMut<MessageLog>,
    asset_server: Res<AssetServer>,
) {
    let font: Handle<Font> = asset_server.load("fonts/Kenney Mini.ttf");

    if !attacks.is_empty() {
        info!("process_attacks: {}", attacks.len());
    }

    for attack in attacks.read() {
        let Ok([attacker, defender]) = combatants.get_many_mut([attack.attacker, attack.target])
        else {
            warn!(
                "either attacker {:?} or target {:?} was not found among combatants: {} vs {}",
                attack.attacker,
                attack.target,
                combatants.contains(attack.attacker),
                combatants.contains(attack.target)
            );
            continue;
        };

        let (defender_id, defender_name, def_params, mut defender) = defender;
        let (_, attacker_name, atk_params, _) = attacker;

        let defender_name = defender_name.map_or("some defender", |n| n.as_str());
        let attacker_name = attacker_name.map_or("some attacker", |n| n.as_str());

        if defender.is_dead {
            log.add(
                format!("{} is already dead", defender_name,),
                colors::KENNEY_GOLD,
            );
            continue;
        }

        let damage = atk_params.attack - def_params.defense;
        if damage >= 0 {
            commands.entity(defender_id).trigger(Hit);
            defender.hp = defender.hp.saturating_sub(damage);
            log.add(
                format!("{} hits {}!", attacker_name, defender_name),
                colors::KENNEY_GOLD,
            );

            if defender.hp <= 0 {
                commands.entity(defender_id).trigger(Died);
                defender.is_dead = true;
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
                    .trigger(Died)
                    .remove::<AgentOfGrid>()
                    .remove::<Turn>();
            } else {
                spawn_floating_text(&mut commands, Color::WHITE, &font, defender_id, damage);
                commands.trigger(Attacked(defender_id))
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
