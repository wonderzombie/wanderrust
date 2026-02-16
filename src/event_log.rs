use std::collections::VecDeque;

use bevy_egui::{EguiContexts, egui};

use bevy::prelude::*;

use crate::colors::ColorExt;

pub fn draw_message_log_ui(
    mut contexts: EguiContexts,
    log: Res<MessageLog>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::Window::new("Messages")
        .fixed_pos([600.0, 200.0])  // Position it where you want
        .fixed_size([200.0, 400.0])
        .movable(false)
        .show(ctx, |ui| {
            for (msg, color) in log.as_color_text() {
                // Convert Bevy Color to egui Color32
                ui.colored_label(color.to_egui(), msg.to_uppercase());
            }
        });
}

#[derive(Debug, Default, Resource)]
pub struct MessageLog {
    messages: VecDeque<String>,
    color_messages: VecDeque<(String, Color)>,
    max_lines: usize,
}

impl MessageLog {
    pub fn new(max_lines: usize) -> Self {
        Self {
            messages: VecDeque::with_capacity(max_lines),
            max_lines,
            ..Default::default()
        }
    }

    pub fn add(&mut self, msg: impl Into<String>, color: impl Into<Color>) {
        if self.messages.len() >= self.max_lines {
            self.messages.pop_front();
        }
        self.color_messages.push_back((msg.into(), color.into()));
    }

    pub fn as_color_text(&self) -> VecDeque<(String, Color)>  {
        self.color_messages.clone()
    }
}
