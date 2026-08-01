use bevy::{prelude::*, text::FontSourceTemplate};

use crate::colors;

pub(crate) fn pcsr_font(font_size: i32) -> impl Scene {
    let font = FontSourceTemplate::Handle("fonts/pcsenior.ttf".into());
    bsn! {
        TextFont {
            font,
            font_size: px(font_size),
            font_smoothing: FontSmoothing::None
        }
    }
}

pub(crate) fn label_row(s: &'static str) -> impl Scene {
    bsn! {
        Node
        Text::new(s)
        pcsr_font(12)
        TextColor(colors::KENNEY_OFF_WHITE)
    }
}
