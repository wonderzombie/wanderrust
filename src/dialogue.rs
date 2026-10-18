use bevy::{
    input::common_conditions::{input_just_pressed, input_pressed},
    prelude::*,
};
use bevy_replique::prelude::*;

use crate::gamestate::GameState;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Loading), setup)
        .add_systems(
            Update,
            (
                (start_dialogue.run_if(
                    input_just_pressed(KeyCode::KeyV).and_then(input_pressed(KeyCode::AltLeft)),
                )),
                show_line,
                next_line.run_if(
                    input_just_pressed(KeyCode::KeyC).and_then(input_pressed(KeyCode::AltLeft)),
                ),
                restart_dialogue,
            )
                .run_if(in_state(GameState::AwaitingInput)),
        );
}

#[derive(Resource, Reflect, Debug, Deref)]
#[reflect(Resource)]
struct Runner(pub Entity);

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let handle = assets.load("dialogue/test.rep");
    info!("loading dialogue: {handle:?}");
    let id = commands.spawn(DialogueRunner::new(handle)).id();
    commands.insert_resource(Runner(id));
}

fn start_dialogue(runner: Res<Runner>, mut start: MessageWriter<StartDialogue>) {
    info!("dialogue start: {}", **runner);
    start.write(StartDialogue {
        runner: **runner,
        node: "start".into(),
    });
}

#[derive(Resource, Debug)]
struct Token(pub DialogueToken);

fn show_line(mut lines: PopulatedMessageReader<DialogueLine>, mut commands: Commands) {
    info!("next dialogue");
    for line in lines.read() {
        commands.insert_resource(Token(line.token));
        info!("dialogue line: {}", line.text);
    }
}

fn next_line(mut resume_dialogue: MessageWriter<ResumeDialogue>, token: Res<Token>) {
    resume_dialogue.write(ResumeDialogue {
        token: token.0,
        input: ResumeInput::Advance,
    });
}

fn restart_dialogue(
    mut finished: MessageReader<DialogueFinished>,
    mut restart: MessageWriter<StartDialogue>,
) {
    for DialogueFinished { runner } in finished.read() {
        info!("dialogue finished; restarting {runner}");
        restart.write(StartDialogue {
            runner: *runner,
            node: "start".into(),
        });
    }
}
