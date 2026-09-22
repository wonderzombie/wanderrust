use bevy::{prelude::*, sprite::Text2dShadow};
use bevy_northstar::prelude::{AgentOfGrid, AgentPos, Blocking, Pathfind};

use crate::{
    actors::{Dead, Player},
    atlas::SpriteAtlas,
    cell::Cell,
    colors,
    gamestate::{AddRecovery, PlayerDied, RecoveryNow, Turn},
    message_log::LogEvent,
    parameters::*,
    sounds,
    tilemap::DenizenOf,
    tiles::TileIdx,
};

#[derive(EntityEvent, Debug)]
pub(crate) struct Attacked(pub Entity);

#[derive(EntityEvent, Debug)]
pub(crate) struct Hit(pub Entity);

#[derive(EntityEvent, Debug)]
pub(crate) struct Died(pub Entity);

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
    combatants: Populated<
        (Entity, &Cell, &Name, &BaseParameters),
        Or<(Added<Combatant>, Added<NeedsRespawn>)>,
    >,
    respawn_info: Query<(Has<NeedsRespawn>, &SpawnPoint)>,
) {
    for (entity, cell, name, base_params) in combatants.into_iter() {
        info!("{name}: init at {cell}");

        let health = base_params.health();

        let mut ecmd = commands.entity(entity);
        ecmd.insert(health)
            .remove::<(Dead, NeedsRespawn)>()
            .queue(RecoveryNow)
            .insert_if_new(CombatantBundle::default());

        match respawn_info.get(entity).ok() {
            Some((
                true,
                SpawnPoint {
                    respawn_cell,
                    level_nt,
                },
            )) => {
                ecmd.insert(ChildOf(*level_nt)).insert(*respawn_cell);
            }
            Some((false, _)) | None => (),
        }
    }
}

/// Set respawn points for combatants who don't have one. This explicitly
/// excludes players to avoid conflicts with `[LastRespawnPoint]`.
pub fn set_mob_spawns(
    mut commands: Commands,
    combatants: Populated<(Entity, &Cell, &DenizenOf), (With<Combatant>, Without<SpawnPoint>)>,
) {
    let mut count = 0;
    // TODO: this may not work if `[tilemap::snapshot_denizens()]` doesn't run before this.
    for (nt, respawn_cell, DenizenOf(level_nt)) in combatants {
        commands
            .entity(nt)
            .insert(SpawnPoint {
                respawn_cell: *respawn_cell,
                level_nt: *level_nt,
            })
            .observe(on_attacked);
        count += 1;
    }

    if count > 0 {
        info!("set_mob_respawn: {count} handled");
    }
}

#[derive(Component, Default, Reflect)]
pub struct Combatant;

#[derive(Component, Reflect, Debug)]
pub struct SpawnPoint {
    pub respawn_cell: Cell,
    pub level_nt: Entity,
}

#[derive(Component, Default, Reflect)]
pub struct NeedsRespawn;

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
    mut combatants: Query<(Entity, &Name, &Parameters, &mut Health, Has<Player>)>,
    mut attacks: MessageReader<Attack>,
    mut log: MessageWriter<LogEvent>,
    asset_server: Res<AssetServer>,
) {
    let font: Handle<Font> = asset_server.load("fonts/Kenney Mini.ttf");

    for attack in attacks.read() {
        trace!("{attack:?}");
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

        let (defender_id, defender_name, def_params, mut defender, is_player) = defender;
        let (attacker_id, attacker_name, atk_params, _, _) = attacker;

        commands
            .entity(attacker_id)
            .queue(AddRecovery(atk_params.attack_speed));

        if defender.is_dead {
            log.write(LogEvent {
                txt: format!("{defender_name} is already dead"),
                color: Some(colors::KENNEY_GOLD),
            });
            continue;
        }
        let damage = atk_params.attack - def_params.defense;
        if damage >= 0 {
            commands.entity(defender_id).trigger(Hit);
            defender.hp = defender.hp.saturating_sub(damage);
            // log.write(LogEvent {
            //     txt: format!("{attacker_name} hits {defender_name}!"),
            //     color: Some(colors::KENNEY_GOLD),
            // });

            if defender.hp <= 0 {
                defender.is_dead = true;
                log.write(LogEvent {
                    txt: format!("{defender_name} is dead"),
                    color: Some(colors::KENNEY_RED),
                });
                spawn_floating_text(
                    &mut commands,
                    colors::KENNEY_RED,
                    &font,
                    defender_id,
                    "*DEAD*",
                );
                commands
                    .entity(defender_id)
                    .insert(Dead)
                    .trigger(Died)
                    .remove::<(AgentOfGrid, AgentPos, Pathfind, Blocking)>()
                    .remove::<CombatantBundle>();

                if is_player {
                    commands.trigger(PlayerDied);
                } else {
                    commands.trigger(sounds::EnemyDefeated);
                }
            } else {
                spawn_floating_text(&mut commands, Color::WHITE, &font, defender_id, damage);
                commands.trigger(Attacked(defender_id))
            }
        } else {
            log.write(LogEvent {
                txt: format!("{attacker_name} does no damage"),
                color: Some(colors::KENNEY_GOLD),
            });
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
            font: FontSource::Handle(font.clone()),
            font_size: FontSize::Px(12.),
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
