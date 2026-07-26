use std::time::Duration;

use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (build_typewriter, advance_typewriter.after(build_typewriter)),
    );
}

#[derive(Component, Clone)]
#[require(Text, TextLayout)]
pub struct Typewriter {
    pub txt: String,
    pub tint: Color,
    pub per_char: Duration,
}

#[derive(Component)]
struct Revealing {
    revealed_idx: usize,
    timer: Timer,
}

fn build_typewriter(
    mut commands: Commands,
    added: Query<(Entity, &Typewriter, &TextFont), Added<Typewriter>>,
) {
    for (nt, tw, font) in added.iter() {
        let bundles = tw
            .txt
            .chars()
            .map(|c| {
                (
                    TextSpan(c.to_string()),
                    TextColor(Color::NONE),
                    font.clone(),
                    ChildOf(nt),
                )
            })
            .collect::<Vec<_>>();

        commands.entity(nt).insert(Revealing {
            revealed_idx: 0,
            timer: Timer::new(tw.per_char, TimerMode::Repeating),
        });
        commands.spawn_batch(bundles);
    }
}

fn advance_typewriter(
    mut commands: Commands,
    time: Res<Time>,
    mut revealing: Query<(Entity, &Children, &Typewriter, &mut Revealing)>,
    mut colors: Query<&mut TextColor, With<TextSpan>>,
) {
    for (nt, children, tw, mut rev) in revealing.iter_mut() {
        rev.timer.tick(time.delta());

        let times_finished = rev.timer.times_finished_this_tick();

        for _ in 0..times_finished {
            let Some(&next_nt) = children.get(rev.revealed_idx) else {
                commands.entity(nt).remove::<Revealing>();
                break;
            };

            if let Ok(mut txt_color) = colors.get_mut(next_nt) {
                if txt_color.as_ref().0 != tw.tint {
                    *txt_color = TextColor(tw.tint);
                }
            }

            rev.revealed_idx += 1;
        }
    }
}
