pub mod chest;
pub mod door;
pub mod shrine;
pub mod speaker;

use bevy::prelude::*;

use crate::interacticator::dispatch_default;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            dispatch_default::<door::Door>,
            dispatch_default::<chest::Chest>,
            dispatch_default::<speaker::Speaker>,
            dispatch_default::<shrine::Shrine>,
        ),
    );
}
