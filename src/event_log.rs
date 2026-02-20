use std::collections::VecDeque;

use bevy_egui::{
    EguiContexts,
    egui::{self, Align2, Vec2},
};

use bevy::prelude::*;

use crate::colors::ColorExt;

pub fn setup_egui_fonts(mut contexts: EguiContexts) {
    let Ok(ctx) = contexts.ctx_mut() else {
        warn!("Egui context not ready yet");
        return;
    };

    let mut fonts = egui::FontDefinitions::default();

    // Load your font file
    fonts.font_data.insert(
        "kenney_mini".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Kenney Mini.ttf")).into(),
    );

    // Set it as the default proportional font (used by labels)
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "kenney_mini".to_owned());

    // Install the fonts
    ctx.set_fonts(fonts);
}

pub fn draw_message_log_ui(mut contexts: EguiContexts, log: Res<MessageLog>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    // TODO: set visuals.extreme_bg_color to hide area behind log.
    egui::Area::new(egui::Id::new("Messages"))
        .anchor(Align2::RIGHT_BOTTOM, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(18.0, egui::FontFamily::Proportional),
            );

            for (msg, color) in log.as_color_text() {
                ui.set_min_width(172.0);
                ui.colored_label(color.to_egui(), msg.to_uppercase());
            }
        });
}

#[derive(Resource, Debug, Default)]
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

    pub fn as_color_text(&self) -> VecDeque<(String, Color)> {
        self.color_messages.clone()
    }
}
