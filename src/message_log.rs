use bevy::prelude::*;

use crate::{colors, ui::theme::pcsr_font};

pub(crate) fn plugin(app: &mut App) {
    app.add_message::<LogEvent>()
        .add_systems(Update, update_log);
}

#[derive(Component, Copy, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct MessageLog;

#[derive(Message, Clone, Debug, Default)]
pub struct LogEvent {
    pub txt: String,
    pub color: Option<Color>,
}

impl LogEvent {
    pub fn from_txt(txt: impl AsRef<str>) -> Self {
        Self {
            txt: txt.as_ref().into(),
            color: None,
        }
    }
}

impl From<(&str, Color)> for LogEvent {
    fn from(value: (&str, Color)) -> Self {
        Self {
            txt: value.0.into(),
            color: Some(value.1),
        }
    }
}

impl From<&str> for LogEvent {
    fn from(value: &str) -> Self {
        Self::from_txt(value)
    }
}

fn update_log(
    mut commands: Commands,
    mut log_events: MessageReader<LogEvent>,
    log_entity: Single<Entity, With<MessageLog>>,
) {
    for LogEvent { txt, color } in log_events.read() {
        let out_txt = txt.to_ascii_uppercase().to_string();
        let out_color = color.unwrap_or(colors::KENNEY_OFF_WHITE);
        let log_nt = *log_entity;

        commands.spawn_scene(bsn! {
            Node {
                width: percent(100),
            }
            pcsr_font(12)
            Text::new(out_txt)
            TextColor(out_color)
            ChildOf(log_nt)
        });
    }
}

pub(crate) fn scene() -> impl Scene {
    bsn! {
        MessageLog
        Visibility::Inherited
        Node {
            flex_direction: FlexDirection::Column,
            overflow: Overflow::scroll_y(),
            padding: UiRect::all(px(8)),
        }
        BackgroundColor(colors::KENNEY_BG)
        ScrollPosition(Vec2 { x: 0., y: f32::MAX })
    }
}
