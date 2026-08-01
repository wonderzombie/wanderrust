use bevy::{prelude::*, text::FontSourceTemplate};

use crate::{colors, gamestate::Screen};

pub struct MessageLogPlugin;

impl Plugin for MessageLogPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<LogEvent>()
            .add_systems(OnEnter(Screen::Playing), setup.run_if(run_once))
            .add_systems(Update, update_log);
    }
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

fn setup(mut commands: Commands, mut writer: MessageWriter<LogEvent>) {
    commands.spawn_scene(log_bundle());
    writer.write("welcome to wanderrust".into());
}

fn update_log(
    mut commands: Commands,
    mut log_events: MessageReader<LogEvent>,
    log_entity: Single<Entity, With<MessageLog>>,
) {
    for LogEvent { txt, color } in log_events.read() {
        let out_txt = format!("> {}", txt.to_ascii_uppercase());
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
        GlobalZIndex(1)
        Node {
            width: px(480),
            height: px(180),
            top: percent(70),
            right: percent(100),
            left: percent(40),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::scroll_y(),
            padding: UiRect::all(px(8)),
            // border: UiRect::all(px(4)),
            // border_radius: BorderRadius::all(px(8)),
        }
        BackgroundColor(colors::KENNEY_BG)
        BorderColor::all(colors::KENNEY_OFF_WHITE)
        ScrollPosition(Vec2 { x: 0., y: f32::MAX })
    }
}
