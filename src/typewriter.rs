use std::time::Duration;

use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (build_typewriter, advance_typewriter.after(build_typewriter)),
    )
    .add_observer(finish_now);
}

#[derive(Component, Clone, FromTemplate, Debug)]
#[require(Text, TextLayout)]
pub struct Typewriter {
    pub txt: String,
    pub tint: Color,
    pub per_char: Duration,
}

#[derive(Component, Debug)]
pub(crate) struct Revealing {
    revealed_idx: usize,
    timer: Timer,
}

#[derive(Event)]
pub(crate) struct FinishNow;

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
        debug!("{:?}: typewriter built with {} items", nt, bundles.len());
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
        trace!("ticking entity {:?} with rev {:?}", nt, rev);
        rev.timer.tick(time.delta());

        let times_finished = rev.timer.times_finished_this_tick();

        for _ in 0..times_finished {
            let Some(&next_nt) = children.get(rev.revealed_idx) else {
                trace!("nt done revealing {:?}", nt);
                commands.entity(nt).remove::<Revealing>();
                break;
            };

            if let Ok(mut txt_color) = colors.get_mut(next_nt) {
                if txt_color.as_ref().0 != tw.tint {
                    *txt_color = TextColor(tw.tint);
                }
            }

            rev.revealed_idx += 1;
            trace!("rev is now {:?}", rev);
        }
    }
}

fn finish_now(
    _on: On<FinishNow>,
    mut commands: Commands,
    revealing: Single<(Entity, &Children, &Typewriter)>,
    colors: Query<Entity, (With<TextSpan>, With<TextColor>)>,
) {
    debug!("asked to finish now");
    let (parent, children, tw) = *revealing;

    for child_nt in colors.iter_many(children) {
        commands.entity(child_nt).insert(TextColor(tw.tint));
    }

    commands.entity(parent).remove::<Revealing>();
}
