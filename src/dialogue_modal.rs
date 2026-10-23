use bevy::prelude::*;
use bevy_replique::{
    call::DialogueToken,
    message::{DialogueLine, ResumeDialogue},
    prelude::*,
};

use crate::{gamestate::Modal, interxables::speaker::DialogueHandle, ui::theme::pcsr_font};

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Modal::Dialogue), scene.spawn())
        .add_systems(
            Update,
            (
                handle_interaction,
                handle_dialogue_line,
                handle_dialogue_finished,
            )
                .run_if(in_state(Modal::Dialogue)),
        )
        .add_observer(on_dialogue_start);
}

/// Initiates dialogue with [`InitiateDialogue::entity`] as the [`Speaker`].
///
/// [`Speaker`]: crate::interxables::speaker::Speaker
#[derive(EntityEvent, Debug)]
pub struct InitiateDialogue {
    pub entity: Entity,
    pub start_node: String,
}

/// Spawns, initializes and starts the [`DialogueRunner`] using the
/// [`DialogueHandle`] stored on the target entity, entering
/// [`Modal::Dialogue`].
pub fn on_dialogue_start(
    start_entity: On<InitiateDialogue>,
    mut commands: Commands,
    mut dialogue_start: MessageWriter<StartDialogue>,
    handles: Query<&DialogueHandle>,
) {
    info!("dialogue start");
    commands.set_state_if_different(Modal::Dialogue);

    let InitiateDialogue { entity, start_node } = start_entity.event();

    let DialogueHandle(handle) = handles
        .get(*entity)
        .with_context(|| format!("failed to acquire handle for {entity} {start_node}"))
        .unwrap();

    // Spawn a new entity with a `DialogueRunner` for this conversation.
    // Messages from `bevy_replique` that concern the `DialogueRunner` will
    // include the runner ID, so we don't need to store it.
    let runner = commands
        .spawn((
            DespawnOnExit(Modal::Dialogue),
            DialogueRunner::new(handle.clone()),
        ))
        .id();

    dialogue_start.write(StartDialogue {
        runner,
        node: start_node.to_owned(),
    });
}

/// [`NextToken`] is inserted on the [`SpeechBox`] entity to persist [`DialogueToken`].
#[derive(Component, Debug, Deref)]
pub struct NextToken(pub DialogueToken);

/// Handles [`DialogueLine`] messages and displays them via [`SpeechText`].
fn handle_dialogue_line(
    mut commands: Commands,
    mut lines: MessageReader<DialogueLine>,
    speech_box: Single<Entity, With<SpeechBox>>,
    mut name_text: Single<&mut Text, (With<SpeakerName>, Without<SpeechText>)>,
    mut speech_text: Single<&mut Text, (With<SpeechText>, Without<SpeakerName>)>,
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

/// Handles user input in order to advance dialogue to the next line (if any) via [`ResumeDialogue`].
fn handle_interaction(
    mut resume_dialogue: MessageWriter<ResumeDialogue>,
    input: Res<ButtonInput<KeyCode>>,
    next_token: Query<&NextToken, With<SpeechBox>>,
) {
    // TODO: pick & use input manager of some sort.
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

/// Handles end of dialogue via [`DialogueFinish`]. Exiting [`Modal::Dialogue`]
/// will cause the [`DialogueRunner`] to despawn automatically via
/// [`DespawnOnExit`].
fn handle_dialogue_finished(mut commands: Commands, mut finished: MessageReader<DialogueFinished>) {
    for _ in finished.read() {
        info!("dialogue complete");
        commands.set_state_if_different(Modal::None);
    }
}

/// Marker component for the root of the dialogue dialog.
#[derive(Component, Debug, Clone, Default)]
pub struct SpeechBox;

/// Marker component for the node which displays speech text.
#[derive(Component, Debug, Clone, Default)]
pub struct SpeechText;

/// Marker component for the node which displays the speaker's name.
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
                    Text::new("")
                    TextLayout::justify(Justify::Center)
                    @pcsr_font(16)
                --
                    Node {
                        padding: UiRect::all(px(4)),
                    }
                    SpeechText
                    Text::new("")
                    TextLayout::justify(Justify::Left)
                    @pcsr_font(14)
            ]
        ]
    }
}
