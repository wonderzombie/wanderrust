use bevy::prelude::*;

use crate::{
    actors::{Action, Actor, DisplayName, Player},
    atlas::SpriteAtlas,
    cell::Cell,
    colors, event_log,
    interactions::Interactable,
    tilemap::Portal,
    tiles::TileIdx,
};

/// Marker struct for the entity with a tooltip background sprite and text.
#[derive(Component)]
struct Tooltip;

pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>, atlas: Res<SpriteAtlas>) {
    let font: Handle<Font> = asset_server.load("fonts/Kenney Mini.ttf");
    let mut sprite = atlas.sprite_from_idx(TileIdx::Blank);
    sprite.custom_size = Some(Vec2::new(32., 12.));
    sprite.color = Color::BLACK;

    commands.spawn((
        sprite,
        Tooltip,
        TextFont {
            font: font.clone(),
            font_size: 8.,
            ..Default::default()
        },
        TextColor(colors::KENNEY_OFF_WHITE),
        Text2d::new(""),
        // children![TooltipText,],
        Visibility::Hidden,
    ));

    commands
        .add_observer(click_observer)
        .insert(Name::new("Click Observer"));
    commands
        .add_observer(over_observer)
        .insert(Name::new("Over Observer"));
    commands
        .add_observer(out_observer)
        .insert(Name::new("Out Observer"));
}

fn make_label<T>(text: T, interact_opt: Option<&Interactable>) -> String
where
    T: std::fmt::Display + AsRef<str>,
{
    let brackets: &'static str = match interact_opt {
        Some(interactbl) => match interactbl {
            Interactable::Combatant => "<>",
            _ => "  ",
        },
        None => "  ",
    };

    format!(
        "{} {} {}",
        brackets.chars().next().unwrap_or('?'),
        text,
        brackets.chars().nth(1).unwrap_or('?'),
    )
}

fn over_observer(
    on: On<Pointer<Over>>,
    actors: Query<
        (
            Entity,
            &TileIdx,
            Option<&Player>,
            Option<&DisplayName>,
            Option<&Interactable>,
            Option<&Portal>,
        ),
        With<Actor>,
    >,
    tooltip_bg: Single<(Entity, &mut Sprite), With<Tooltip>>,
    mut commands: Commands,
) {
    let Ok((over_entity, tile, player_opt, name_opt, interact_opt, portal_opt)) =
        actors.get(on.entity)
    else {
        return;
    };

    let label = if player_opt.is_some() {
        " player ".to_string()
    } else if portal_opt.is_some() {
        " exit ".to_string()
    } else if let Some(name) = tile.label() {
        format!(" {name} ")
    } else {
        let ty = name_opt.map_or_else(|| format!("{tile}"), |n| n.0.clone());
        make_label(ty, interact_opt)
    };

    // Get label and calculate an estimate of width.
    let width = label.len() as f32 * 5.;

    // Resize sprite.
    let (entity, mut sprite) = tooltip_bg.into_inner();
    sprite.custom_size = Some(Vec2::new(width, 12.));

    commands.entity(entity).insert((
        Visibility::Visible,
        ChildOf(over_entity),
        // Position relative to the over_entity, not the world origin.
        Transform::from_xyz(0., 16., 1.),
        Text2d::new(label.to_ascii_uppercase()),
    ));
}

fn out_observer(
    _on: On<Pointer<Out>>,
    tooltip: Single<Entity, With<Tooltip>>,
    mut commands: Commands,
) {
    commands.entity(*tooltip).insert(Visibility::Hidden);
    commands.entity(*tooltip).remove::<Text2d>();
}

fn click_observer(
    on: On<Pointer<Click>>,
    input: Res<ButtonInput<KeyCode>>,
    tile_cells: Query<(&TileIdx, &Cell)>,
    player: Single<(Entity, &Cell), With<Player>>,
    mut log: ResMut<event_log::MessageLog>,
    mut actions: MessageWriter<Action>,
) {
    let (entity, &origin_cell) = *player;
    match tile_cells.get(on.event_target()) {
        Ok((tile_idx, &cell)) => {
            let orig = origin_cell;
            let delta = orig - cell;

            if !input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
                // Find direction relative to the player
                let d = delta.as_vec().normalize_or_zero();
                if d == Vec2::ZERO {
                    return;
                }
                let direction = Cell::from_vec(d);
                let target_cell = origin_cell - direction;

                if target_cell == origin_cell {
                    return;
                }

                let action = Action {
                    entity,
                    origin_cell,
                    target_cell,
                };
                info!("action: {:?}", action);
                actions.write(action);
            } else {
                log.add(format!("{} = {:?}", cell, tile_idx), Color::WHITE);
            }
        }
        Err(err) => {
            trace!("couldn't get_entity() on.event_target(): {:?}", err);
        }
    }
}
