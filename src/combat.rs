use bevy::prelude::*;
use bevy_northstar::prelude::AgentOfGrid;

use crate::{
    actors::{Dead, DisplayName},
    colors,
    event_log::MessageLog,
    gamestate::Turn,
};

#[derive(Component, Debug, Default)]
pub struct CombatStats {
    pub hp: i32,
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub is_dead: bool,
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
) {
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

        log.add(
            format!("{} attacks {}", attacker_name, defender_name),
            colors::KENNEY_GOLD,
        );

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
                commands
                    .entity(defender_entity)
                    .insert(Dead)
                    .remove::<AgentOfGrid>()
                    .remove::<Turn>();
            }
        } else {
            log.add(
                format!("{} does no damage", attacker_name),
                colors::KENNEY_GOLD,
            )
        }
    }
}
