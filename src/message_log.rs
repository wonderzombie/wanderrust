use bevy::{prelude::*, text::FontSourceTemplate};

use crate::{colors::Palette, gamestate::Screen};

pub struct MessageLogPlugin;

impl Plugin for MessageLogPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<LogEvent>()
            .add_systems(OnEnter(Screen::Playing), setup.run_if(run_once))
            .add_systems(Update, update_log.run_if(in_state(Screen::Playing)));
    }
}

const WRAP_CHARS: usize = 26;
const INIT_INDENT: &str = "• ";

#[derive(Component, Copy, Clone, Debug, Default)]
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

fn setup(mut commands: Commands) {
    commands.spawn_scene(log_bundle());
}

fn update_log(
    mut commands: Commands,
    mut log_events: MessageReader<LogEvent>,
    log_entity: Single<Entity, With<MessageLog>>,
) {
    for LogEvent { txt, color } in log_events.read() {
        let options = textwrap::Options::new(WRAP_CHARS).initial_indent(INIT_INDENT);

        let out_txt = textwrap::fill(txt.to_ascii_uppercase().as_str(), options);
        let out_color = color.unwrap_or_else(|| Palette::KenneyOffWhite.srgb());
        let log_nt = *log_entity;

        commands.spawn_scene(bsn! {
            Node
            pcsr_font(12)
            Text::new(out_txt)
            TextColor(out_color)
            ChildOf(log_nt)
        });
    }
}

fn pcsr_font(font_size: i32) -> impl Scene {
    let font = FontSourceTemplate::Handle("fonts/pcsenior.ttf".into());
    bsn! {
        TextFont { font, font_size: px(font_size) }
    }
}

fn log_bundle() -> impl Scene {
    bsn! {
        MessageLog
        Visibility::Inherited
        Node {
            width: px(384),
            height: px(160),
            top: percent(70),
            right: percent(100),
            left: percent(60),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::scroll_y(),
        }
        ScrollPosition(Vec2 { x: 0., y: f32::MAX })
        Children [
            Node
            Text::new("welcome to wanderrust.") pcsr_font(12),
            Node
            Text::new("stay a while.") pcsr_font(12),
            Node
            Text::new("stay forever!") pcsr_font(12),
        ]
    }
}
