use std::marker::PhantomData;

use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {}

pub enum Outcome {
    Failure,
    Success,
}

pub trait Interacticator: Command + Send + 'static {
    type Subject: Interxn<Default = Self>;
    type Result;
    fn perform(self, world: &mut World) -> Self::Result;
}

pub trait Interxn: Component {
    type Default: Interacticator<Subject = Self>;
    fn default_action(actor: Entity, target: Entity) -> Self::Default;
}

trait InteractExt {
    fn interact<T: Interxn + Command>(&mut self, actor: Entity, target: Entity);
}

impl InteractExt for Commands<'_, '_> {
    fn interact<T: Interxn>(&mut self, actor: Entity, target: Entity) {
        self.queue(T::default_action(actor, target));
    }
}

// struct Args<T: Component> {
//     actor: Entity,
//     target: Entity,
//     _t: PhantomData<T>,
// }

#[macro_export]
macro_rules! interacticator {
    ( $verb:ident => $obj:ident => $fxn:path ) => {
        use crate::interacticator::*;

        #[derive(Component)]
        struct $obj;

        impl Interxn for $obj {
            type Default = $verb;

            fn default_action(actor: Entity, target: Entity) -> Self::Default {
                $verb { actor, target }
            }
        }

        impl Command for $verb {
            type Out = ();
            fn apply(self, world: &mut World) -> Self::Out {
                let _ = self.perform(world);
            }
        }

        impl Interacticator for $verb {
            type Subject = $obj;
            type Result = Result<Outcome, anyhow::Error>;

            fn perform(self, world: &mut World) -> Self::Result {
                world
                    .run_system_cached_with($fxn, self)
                    .map_err(|e| anyhow::anyhow!(e))
            }
        }
    };
}
