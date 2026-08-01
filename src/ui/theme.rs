use bevy::{prelude::*, text::FontSourceTemplate};

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
