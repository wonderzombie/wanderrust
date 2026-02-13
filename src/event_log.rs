use std::collections::VecDeque;

use bevy::prelude::*;

#[derive(Debug, Default, Resource)]
pub struct MessageLog {
    messages: VecDeque<String>,
    max_lines: usize,
}

impl MessageLog {
    pub fn new(max_lines: usize) -> Self {
        Self {
            messages: VecDeque::with_capacity(max_lines),
            max_lines,
        }
    }

    pub fn add(&mut self, msg: impl Into<String>) {
        if self.messages.len() >= self.max_lines {
            self.messages.pop_front();
        }
        self.messages.push_back(msg.into());
    }

    pub fn as_text(&self) -> String {
        self.messages.iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Component)]
pub struct LogDisplay;

pub fn setup_log(mut commands: Commands) {
    commands.spawn((
        LogDisplay,
        Text::new(""),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            width: Val::Px(600.0),
            height: Val::Px(100.0),
            ..default()
        },
    ));
}

pub fn update_log_display(
    log: Res<MessageLog>,
    mut display: Query<&mut Text, With<LogDisplay>>,
) {
    if log.is_changed() {
        if let Ok(mut text) = display.single_mut() {
            text.0 = log.as_text();
        }
    }
}
