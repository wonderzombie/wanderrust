use std::time::Duration;

use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        // `after()` ensures that ApplyDeferred occurs between building and starting.
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
pub(crate) struct Writing {
    revealed_idx: usize,
    timer: Timer,
}

#[derive(Component, Debug, Clone, Default, Copy)]
pub(crate) struct Finished;

#[derive(EntityEvent)]
pub(crate) struct FinishNow(pub Entity);

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

        commands.entity(nt).insert(Writing {
            revealed_idx: 0,
            timer: Timer::new(tw.per_char, TimerMode::Repeating),
        });
        debug!("{:?}: typewriter built with {} items", nt, bundles.len());
        // Ordering is VERY important — rely on the fact that these bundles
        // will be inserted in the same order *and* that the Relationship
        // mechanisms will ensure that `Children` exist in that order.
        commands.spawn_batch(bundles);
    }
}

fn advance_typewriter(
    mut commands: Commands,
    time: Res<Time>,
    mut revealing: Query<(Entity, &Children, &Typewriter, &mut Writing)>,
    mut colors: Query<&mut TextColor, With<TextSpan>>,
) {
    for (nt, children, tw, mut rev) in revealing.iter_mut() {
        trace!("ticking entity {:?} with rev {:?}", nt, rev);
        rev.timer.tick(time.delta());

        let times_finished = rev.timer.times_finished_this_tick();

        for _ in 0..times_finished {
            // If children are no longer inserted instantly and therefore in
            // deterministic order, this logic breaks down.
            let Some(&next_nt) = children.get(rev.revealed_idx) else {
                info!("nt done revealing {:?}", nt);
                commands.entity(nt).remove::<Writing>().insert(Finished);
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
    on: On<FinishNow>,
    mut commands: Commands,
    typewriters: Query<(&Children, &Typewriter)>,
    mut colors: Query<&mut TextColor, With<TextSpan>>,
) {
    info!("asked to finish now {:?}", on.0);
    let Ok((children, tw)) = typewriters.get(on.0) else {
        info!("no typewriter found");
        return;
    };
    info!("finishing {tw:?}");

    for child in children.iter() {
        if let Ok(mut color) = colors.get_mut(child) {
            color.set_if_neq(TextColor(tw.tint));
        }
    }

    info!("{:?} now revealed", on.0);
    commands.entity(on.0).insert(Finished).remove::<Writing>();
}
