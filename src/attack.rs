#![allow(dead_code)]
use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {}

#[derive(Copy, Clone, Debug)]
struct Attack {
    attacker: Entity,
    defender: Option<Entity>,
    kind: AttackKind,
    role: AttackRole,
}

impl Attack {
    fn with_weapon(attacker: Entity, defender: Option<Entity>, weapon: Option<Entity>) -> Attack {
        let kind = AttackKind::melee(weapon);
        let role = AttackRole::Primary;
        Attack {
            attacker,
            defender,
            kind,
            role,
        }
    }

    fn with_kind(attacker: Entity, defender: Option<Entity>, kind: AttackKind) -> Attack {
        Attack {
            attacker,
            defender,
            kind,
            role: AttackRole::Primary,
        }
    }

    fn with_role(&self, role: AttackRole) -> Attack {
        Attack { role, ..*self }
    }

    fn cleave_into(&self, target: Entity) -> Attack {
        Attack {
            defender: Some(target),
            role: AttackRole::Cleave,
            ..*self
        }
    }

    fn is_primary(&self) -> bool {
        matches!(self.role, AttackRole::Primary)
    }

    fn can_cleave(&self) -> bool {
        self.is_primary() && self.kind.could_cleave()
    }

    fn can_follow_up(&self) -> bool {
        self.is_primary() && self.kind.could_follow_up()
    }
}

#[derive(Default, Copy, Clone, Debug)]
struct WeaponFlags {
    /// Whether this attack should damage adjacent enemies.
    // TODO: should go on a weapon.
    can_cleave: bool,
    /// Whether this attack should damage the target more than once.
    // TODO: should go on a weapon.
    can_follow_up: bool,
    /// Whether this attack can reach beyond just adjacent tiles.
    has_reach: bool,
}

/// The kind of attack, whether it be ranged or melee.
#[derive(Copy, Clone, Debug)]
enum AttackKind {
    Melee {
        /// Entity for the weapon used, if any.
        weapon: Option<Entity>,
        flags: WeaponFlags,
    },
    Ranged {
        /// The projectile that will be launched, thrown, etc.
        projectile: Entity,
    },
}

impl AttackKind {
    fn melee(weapon: Option<Entity>) -> AttackKind {
        AttackKind::Melee {
            weapon,
            flags: WeaponFlags::default(),
        }
    }

    fn melee_cleaving(weapon: Option<Entity>) -> AttackKind {
        let flags = WeaponFlags {
            can_cleave: true,
            ..Default::default()
        };
        AttackKind::Melee { weapon, flags }
    }

    fn melee_follow_up(weapon: Option<Entity>) -> AttackKind {
        let flags = WeaponFlags {
            can_follow_up: true,
            ..Default::default()
        };
        AttackKind::Melee { weapon, flags }
    }

    fn ranged(projectile: Entity) -> AttackKind {
        AttackKind::Ranged { projectile }
    }

    fn could_cleave(&self) -> bool {
        matches!(
            self,
            AttackKind::Melee {
                flags: WeaponFlags {
                    can_cleave: true,
                    ..
                },
                ..
            }
        )
    }

    fn could_follow_up(&self) -> bool {
        matches!(
            self,
            AttackKind::Melee {
                flags: WeaponFlags {
                    can_follow_up: true,
                    ..
                },
                ..
            }
        )
    }
}

#[derive(Copy, Clone, Debug)]
enum AttackRole {
    Primary,
    Cleave,
    FollowUp,
}

//////////

// Prototype.
fn resolve(attack: Attack) {
    match attack.role {
        AttackRole::Primary => {
            println!("primary attack!");

            if attack.can_cleave() {
                println!("... and then");
                for _ in 0..2 {
                    resolve(attack.with_role(AttackRole::Cleave));
                }
            }

            if attack.can_follow_up() {
                println!("... and then");
                resolve(attack.with_role(AttackRole::FollowUp));
            }
        }
        AttackRole::Cleave => {
            println!("cleave attack!");
        }
        AttackRole::FollowUp => {
            println!("follow-up attack!");
        }
    }
}
