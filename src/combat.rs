use bevy::{prelude::*, sprite::Text2dShadow};
use bevy_northstar::prelude::AgentOfGrid;

use crate::{
    actors::Dead, atlas::SpriteAtlas, bestiary::Bestiary, colors, event_log::MessageLog,
    gamestate::Turn, interactions::Interactable, parameters::*, tiles::TileIdx,
};

#[derive(EntityEvent, Debug)]
pub(crate) struct Attacked(pub Entity);

#[derive(EntityEvent, Debug)]
pub(crate) struct Hit(pub Entity);

#[derive(EntityEvent, Debug)]
pub(crate) struct Died(pub Entity);

/// Detects entities with Interactable that may be Belligerents.
/// Adds Combatant and Name components.
pub fn detect_belligerents(
    mut commands: Commands,
    interxs: Populated<(Entity, &Interactable), Added<Interactable>>,
) {
    for (entity, interx) in interxs {
        if let Interactable::Belligerent { name, .. } = interx {
            commands
                .entity(entity)
                .insert((CombatantBundle::default(), Name::new(name.clone())));
        }
    }
}

#[derive(Component)]
pub(crate) struct AttackIcon(pub Timer);

impl AttackIcon {
    pub(crate) fn new(duration: f32) -> Self {
        Self(Timer::from_seconds(duration, TimerMode::Once))
    }
}

pub fn on_attacked(on: On<Attacked>, mut commands: Commands, atlas: Res<SpriteAtlas>) {
    let defender = on.event_target();
    let sprite = atlas.sprite_from_idx(TileIdx::SlashDiagonal);

    commands
        .entity(defender)
        .with_child((AttackIcon::new(0.5), sprite));
}

pub(crate) fn animate_icons(
    mut commands: Commands,
    time: Res<Time>,
    anims: Populated<(Entity, &mut AttackIcon)>,
) {
    for (nt, mut icon) in anims {
        icon.0.tick(time.delta());
        if icon.0.is_finished() {
            commands.entity(nt).despawn();
        }
    }
}

/// Adds combat parameters and health to entities that have received a Combatant component.
/// They will only receive Parameters if they don't have any, but they always receive health.
pub fn init_combatants(
    mut commands: Commands,
    combatants: Populated<(Entity, &TileIdx, &Name, Option<&Parameters>), Added<Combatant>>,
) {
    for (entity, tile_idx, name, params_opt) in combatants.into_iter() {
        let params = params_opt
            .copied()
            .or_else(|| Bestiary::from_tile(tile_idx))
            .or_else(|| Bestiary::from_name(name))
            .unwrap_or_default();

        if params.is_default() {
            warn!("{entity:?} {name} {tile_idx} uses default combat Parameters");
        }

        let health = Health {
            hp: params.max_hp.cast_signed(),
            is_dead: false,
        };

        info!("initialized combatant {entity:?}: {params:?} and {health:?}");

        commands
            .entity(entity)
            .insert_if_new(params)
            .insert(health)
            .observe(on_attacked);
    }
}

#[derive(Component, Default, Reflect)]
pub struct Combatant;

#[derive(Bundle, Default)]
pub struct CombatantBundle {
    pub combatant: Combatant,
    pub awareness: Awareness,
    pub turn: Turn,
}

#[derive(Message, Debug, Copy, Clone, Reflect)]
pub struct Attack {
    pub attacker: Entity,
    pub target: Entity,
}

pub fn process_attacks(
    mut commands: Commands,
    mut combatants: Query<(Entity, &Name, &Parameters, &mut Health)>,
    mut attacks: MessageReader<Attack>,
    mut log: ResMut<MessageLog>,
    asset_server: Res<AssetServer>,
) {
    let font: Handle<Font> = asset_server.load("fonts/Kenney Mini.ttf");

    for attack in attacks.read() {
        info!("{attack:?}");
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

        if defender.is_dead {
            log.add(
                format!("{defender_name} is already dead"),
                colors::KENNEY_GOLD,
            );
            continue;
        }

        let damage = atk_params.attack - def_params.defense;
        if damage >= 0 {
            commands.entity(defender_id).trigger(Hit);
            defender.hp = defender.hp.saturating_sub(damage);
            log.add(
                format!("{attacker_name} hits {defender_name}!"),
                colors::KENNEY_GOLD,
            );

            if defender.hp <= 0 {
                commands.entity(defender_id).trigger(Died);
                defender.is_dead = true;
                log.add(format!("{defender_name} is dead"), colors::KENNEY_RED);
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
                format!("{attacker_name} does no damage"),
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
