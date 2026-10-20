
use bevy::prelude::*;
use bevy_replique::{
    call::DialogueToken,
    message::{DialogueLine, ResumeDialogue},
    prelude::*,
};

use crate::{gamestate::Modal, ui::theme::pcsr_font};

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Modal::Dialogue), setup)
        .add_systems(
            Update,
            (
                interaction_system,
                on_dialogue_line_message,
                on_dialogue_finished_message,
            )
                .run_if(in_state(Modal::Dialogue)),
        )
        .add_observer(on_dialogue_start);
}

fn setup(mut commands: Commands) {
    info!("setup");
    commands.spawn_scene(scene());
}

#[derive(Event, Debug, Deref)]
pub struct DialogueStart(pub Entity);

pub fn on_dialogue_start(
    on: On<DialogueStart>,
    mut commands: Commands,
    mut dialogue_start: MessageWriter<StartDialogue>,
) {
    info!("dialogue start");
    commands.set_state_if_different(Modal::Dialogue);
    dialogue_start.write(StartDialogue {
        runner: **on,
        node: "start".into(),
    });
}

#[derive(Component, Debug, Deref)]
pub struct NextToken(pub DialogueToken);

fn on_dialogue_line_message(
    mut commands: Commands,
    mut lines: MessageReader<DialogueLine>,
    speech_box: Single<Entity, With<SpeechBox>>,
    mut name_text: Single<&mut Text, (With<SpeakerName>, Without<SpeakerText>)>,
    mut speech_text: Single<&mut Text, (With<SpeakerText>, Without<SpeakerName>)>,
) {
    for line in lines.read() {
        info!("dialogue line");
        let speaker_name = line.speaker.clone().unwrap_or_default().to_uppercase();
        let speaker_text = line.text.clone().to_uppercase();

        name_text.set_if_neq(speaker_name.into());
        speech_text.set_if_neq(speaker_text.into());

        commands.entity(*speech_box).insert(NextToken(line.token));
    }
}

fn interaction_system(
    mut resume_dialogue: MessageWriter<ResumeDialogue>,
    input: Res<ButtonInput<KeyCode>>,
    next_token: Query<&NextToken, With<SpeechBox>>,
) {
    if input.is_changed()
        && input.any_just_pressed([KeyCode::Space, KeyCode::Enter, KeyCode::Escape])
    {
        if let Ok(next_token) = next_token.single() {
            info!("next line");
            resume_dialogue.write(ResumeDialogue {
                token: **next_token,
                input: ResumeInput::Advance,
            });
        } else {
            error!("unable to find next dialogue token");
        }
    }
}

fn on_dialogue_finished_message(
    mut commands: Commands,
    mut finished: MessageReader<DialogueFinished>,
) {
    for _ in finished.read() {
        info!("dialogue complete");
        commands.set_state_if_different(Modal::None);
    }
}

#[derive(Component, Debug, Clone, Default)]
pub struct SpeechBox;

#[derive(Component, Debug, Clone, Default)]
pub struct SpeakerText;

#[derive(Component, Debug, Clone, Default)]
pub struct SpeakerName;

fn scene() -> impl Scene {
    bsn! {
        #DialogueScene
        SpeechBox
        DespawnOnExit<Modal>(Modal::Dialogue)
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::End,
            justify_content: JustifyContent::Center,
        }
        Children [
            Node {
                width: vw(75.),
                height: vh(20.),
                row_gap: px(4.),
                flex_direction: FlexDirection::Column,
            }
            BackgroundColor(Color::BLACK)
            Children [
                    Node {
                        padding: UiRect::all(px(4)),
                    }
                    SpeakerName
                    Text::new("Metir")
                    TextLayout::justify(Justify::Center)
                    @pcsr_font(16)
                --
                    Node {
                        padding: UiRect::all(px(4)),
                    }
                    SpeakerText
                    Text::new("")
                    TextLayout::justify(Justify::Left)
                    @pcsr_font(14)
            ]
        ]
    }
}
