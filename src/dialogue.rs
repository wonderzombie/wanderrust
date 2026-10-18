use bevy::prelude::*;
use bevy_replique::prelude::*;

use crate::gamestate::GameState;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Loading), setup);
}

#[derive(Resource, Reflect, Debug, Default)]
#[reflect(Resource)]
struct DialogueHandle(pub Handle<RepliqueDialogue>);

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let handle = assets.load("dialogue/test.rep");
    commands.insert_resource(DialogueHandle(handle));
}
