use bevy::prelude::*;

use crate::{colors, event_log::MessageLog};

#[derive(Component, Debug, Default)]
pub struct CombatStats {
    pub nameplate: String,
    pub hp: i32,
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub is_dead: bool,
}

#[derive(Message, Debug, Copy, Clone)]
pub struct AttackAttempt {
    pub attacker: Entity,
    pub target: Entity,
}

pub fn init_combatants(mut combatants: Query<&mut CombatStats, Added<CombatStats>>) {
    for mut combatant in combatants.iter_mut() {
        combatant.hp = combatant.max_hp;
    }
}

/// Routes [ActionAttempt] messages to one of four outcomes: move, portal, interact, or blocked.
/// Interaction execution is handled by [process_interactions].
pub fn process_attacks(
    mut combatants: Query<&mut CombatStats>,
    mut attacks: MessageReader<AttackAttempt>,
    mut log: ResMut<MessageLog>,
) {
    for attack in attacks.read() {
        let Ok([attacker, mut defender]) =
            combatants.get_many_mut([attack.attacker, attack.target])
        else {
            continue;
        };

        if defender.is_dead {
            log.add(
                format!("{} is already dead", defender.nameplate),
                colors::KENNEY_GOLD,
            );
            continue;
        }

        log.add(
            format!("{} attacks {}", attacker.nameplate, defender.nameplate),
            colors::KENNEY_GOLD,
        );

        let damage = attacker.attack - defender.defense;
        if damage >= 0 {
            defender.hp = defender.hp.saturating_sub(damage);
            log.add(
                format!("{} hits {}!", attacker.nameplate, defender.nameplate),
                colors::KENNEY_GOLD,
            );

            if defender.hp <= 0 {
                defender.is_dead = true;
                log.add(
                    format!("{} is dead", defender.nameplate),
                    colors::KENNEY_RED,
                );
            }
        }
    }
}
