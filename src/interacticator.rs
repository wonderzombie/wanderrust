use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {}

pub enum Outcome {
    Failure,
    Success,
}

pub trait Interactuator: Command + Send + 'static {
    type Subject: Interxn<Default = Self>;
    type Result;
    fn perform(self, world: &mut World) -> Self::Result;
}

pub trait Interxn: Component {
    type Default: Interactuator<Subject = Self>;
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
