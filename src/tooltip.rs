use bevy::prelude::*;

use crate::{
    actors::{Actor, DisplayName, Player},
    atlas::SpriteAtlas,
    colors,
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
        Visibility::Hidden,
    ));

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
    tooltip: Single<(Entity, &mut Sprite, &mut Text2d), With<Tooltip>>,
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
    let (entity, mut sprite, mut text) = tooltip.into_inner();
    sprite.custom_size = Some(Vec2::new(width, 12.));
    text.0 = label.to_ascii_uppercase();

    commands.entity(entity).insert((
        Visibility::Visible,
        // TODO: use absolute world positioning rather than parenting. That is,
        // parent the tooltip to the window after creation and don't change that.
        // Instead, get the mouse coords in world space and move it there,
        // alongside the adjustment implied by Transform below.
        ChildOf(over_entity),
        // Position relative to the over_entity, not the world origin.
        Transform::from_xyz(0., 16., 1.),
    ));
}

fn out_observer(
    _on: On<Pointer<Out>>,
    tooltip: Single<(Entity, &mut Text2d), With<Tooltip>>,
    mut commands: Commands,
) {
    let (entity, mut text) = tooltip.into_inner();

    commands.entity(entity).insert(Visibility::Hidden);
    commands.entity(entity).remove::<ChildOf>();

    text.0.clear();
}
