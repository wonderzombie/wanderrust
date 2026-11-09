#![allow(unused)]
use bevy::prelude::*;

use crate::equipment::MeleeFlags;

pub(super) fn plugin(app: &mut App) {}

#[derive(Message, Copy, Clone, Debug, Reflect)]
pub struct Attack {
    pub attacker: Entity,
    pub target: Option<Entity>,
    pub kind: AttackKind,
    pub role: AttackRole,
}

impl Attack {
    pub fn basic(attacker: Entity, defender: Entity) -> Attack {
        Self::with_weapon(attacker, Some(defender), None)
    }

    fn with_weapon(attacker: Entity, target: Option<Entity>, weapon: Option<Entity>) -> Attack {
        let kind = AttackKind::melee(weapon);
        let role = AttackRole::Primary;
        Attack {
            attacker,
            target,
            kind,
            role,
        }
    }

    fn with_kind(attacker: Entity, target: Option<Entity>, kind: AttackKind) -> Attack {
        Attack {
            attacker,
            target,
            kind,
            role: AttackRole::Primary,
        }
    }

    fn with_role(&self, role: AttackRole) -> Attack {
        Attack { role, ..*self }
    }

    fn cleave_into(&self, target: Entity) -> Attack {
        Attack {
            target: Some(target),
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

/// The kind of attack, whether it be ranged or melee.
#[derive(Copy, Clone, Debug, Reflect)]
pub enum AttackKind {
    Melee {
        /// Entity for the weapon used, if any.
        weapon: Option<Entity>,
        flags: MeleeFlags,
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
            flags: MeleeFlags::default(),
        }
    }

    fn melee_cleaving(weapon: Option<Entity>) -> AttackKind {
        let flags = MeleeFlags {
            can_cleave: true,
            ..Default::default()
        };
        AttackKind::Melee { weapon, flags }
    }

    fn melee_follow_up(weapon: Option<Entity>) -> AttackKind {
        let flags = MeleeFlags {
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
                flags: MeleeFlags {
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
                flags: MeleeFlags {
                    can_follow_up: true,
                    ..
                },
                ..
            }
        )
    }
}

#[derive(Copy, Clone, Debug, Reflect, Default)]
pub enum AttackRole {
    #[default]
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
