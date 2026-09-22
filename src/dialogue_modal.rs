use bevy::prelude::*;

use crate::{gamestate::Modal, interxables::speaker::Dialogue, ui::theme::pcsr_font};

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Modal::Dialogue), (setup, populate).chain())
        .add_systems(OnExit(Modal::Dialogue), discard)
        .add_systems(Update, interaction_system.run_if(in_state(Modal::Dialogue)))
        .add_observer(on_dialogue_start);
}

fn setup(mut commands: Commands) {
    info!("setup");
    commands.spawn_scene(scene());
}

fn discard(mut commands: Commands, scene: Single<Entity, With<SpeechBox>>) {
    info!("discard");
    commands.entity(*scene).despawn();
}

#[derive(Event, Debug)]
pub struct DialogueStart(pub Entity);

#[derive(Resource)]
pub struct DialogueEntity(pub Entity);

pub fn on_dialogue_start(on: On<DialogueStart>, mut commands: Commands) {
    let DialogueStart(speaker_nt) = *on;
    commands.insert_resource(DialogueEntity(speaker_nt));
    commands.set_state_if_neq(Modal::Dialogue)
}

fn populate(
    current_speaker: Res<DialogueEntity>,
    mut dialogues: Query<(&Name, &mut Dialogue)>,
    mut name_text: Single<&mut Text, (With<SpeakerName>, Without<SpeakerText>)>,
    mut speech_text: Single<&mut Text, (With<SpeakerText>, Without<SpeakerName>)>,
) {
    let DialogueEntity(speaker_nt) = *current_speaker;
    let Some((name, mut dialogue)) = dialogues.get_mut(speaker_nt).ok() else {
        error!("entity has no dialogue: {speaker_nt}");
        return;
    };

    name_text.set_if_neq(format!("[{}]", name.to_ascii_uppercase()).into());

    let speech = dialogue.advance().map(|it| it.to_ascii_uppercase());
    speech_text.set_if_neq(format!("\"{}\"", speech.unwrap_or_default()).into());
}

fn interaction_system(
    mut commands: Commands,
    interactions: Query<&Interaction, Changed<Interaction>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.is_changed()
        && input.any_just_pressed([KeyCode::Space, KeyCode::Enter, KeyCode::Escape])
    {
        commands.set_state_if_neq(Modal::None);
        return;
    }

    for interaction in interactions {
        if interaction == &Interaction::Pressed {
            commands.set_state_if_neq(Modal::None);
            return;
        }
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
                (
                    Node {
                        padding: UiRect::all(px(4)),
                    }
                    SpeakerName
                    Text::new("Metir")
                    TextLayout::justify(Justify::Center)
                    pcsr_font(16)
                ),
                (
                    Node {
                        padding: UiRect::all(px(4)),
                    }
                    SpeakerText
                    Text::new("")
                    TextLayout::justify(Justify::Left)
                    pcsr_font(14)
                )
            ]
        ]
    }
}
