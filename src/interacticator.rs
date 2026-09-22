use bevy::prelude::*;

use crate::interactions::Examine;

pub enum Outcome {
    Failure,
    Success,
}

#[derive(Copy, Clone)]
pub struct Actors {
    pub actor: Entity,
    pub target: Entity,
}

impl From<&Examine> for Actors {
    fn from(Examine { actor, target }: &Examine) -> Self {
        Self {
            actor: *actor,
            target: *target,
        }
    }
}

pub fn dispatch_default<T: Interxable<Args = Actors>>(
    mut examines: MessageReader<Examine>,
    targets: Query<(), With<T>>,
    mut commands: Commands,
) {
    for ex in examines.read() {
        trace!("ex: {} {ex:#?}", T::name());
        if targets.contains(ex.target) {
            info!("interacted with {}", T::name());
            commands.interact::<T>(Actors {
                actor: ex.actor,
                target: ex.target,
            });
        }
    }
}

pub trait Interacticator: Command + Send + 'static {
    /// This verb operates on a Subject that is Interactable.
    type Subject: Interxable;
    type Result;

    fn perform(self, world: &mut World) -> Self::Result;
}

pub trait Interxable: Component {
    type DefaultAction: Interacticator<Subject = Self>;
    type Args: Into<Self::DefaultAction>;

    fn default_action(args: Self::Args) -> Self::DefaultAction {
        Self::Args::into(args)
    }

    fn name() -> &'static str;
}

pub trait InteractCommand {
    fn interact<T: Interxable>(&mut self, args: T::Args);
}

impl InteractCommand for Commands<'_, '_> {
    fn interact<T: Interxable>(&mut self, args: T::Args) {
        self.queue(T::default_action(args));
    }
}

/// Pronounced "interaction-able."
#[macro_export]
macro_rules! interaxnable {
    ( $obj:ident defaults to $verb:ident ) => {
        impl crate::interacticator::Interxable for $obj {
            type DefaultAction = $verb;
            type Args = $crate::interacticator::Actors;

            fn name() -> &'static str {
                stringify!($obj)
            }
        }
    };
}

#[macro_export]
macro_rules! interacticator {
    ( $verb:ident on $obj:ident via $fxn:path ) => {
        pub struct $verb {
            actor: ::bevy::prelude::Entity,
            target: ::bevy::prelude::Entity,
        }

        impl From<$crate::interacticator::Actors> for $verb {
            fn from(
                $crate::interacticator::Actors { actor, target }: $crate::interacticator::Actors,
            ) -> Self {
                $verb { actor, target }
            }
        }

        impl ::bevy::prelude::Command for $verb {
            type Out = ::bevy::prelude::Result<()>;
            fn apply(self, world: &mut ::bevy::prelude::World) -> Self::Out {
                $crate::interacticator::Interacticator::perform(self, world).map(|_| ())
            }
        }

        impl $crate::interacticator::Interacticator for $verb {
            type Subject = $obj;
            type Result = ::bevy::prelude::Result<$crate::interacticator::Outcome>;

            fn perform(self, world: &mut ::bevy::prelude::World) -> Self::Result {
                world.run_system_cached_with($fxn, self)?
            }
        }
    };
}
