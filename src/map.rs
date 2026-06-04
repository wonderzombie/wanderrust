use crate::{
    colors,
    light::{AmbientLight, LightLevel},
    tilemap::{ActiveLevel, Level},
    tiles::{Highlighted, MapTile, Occupied, Opaque, Revealed, TileIdx, Walkable},
};

use bevy::ecs::query::QueryData;
use bevy::prelude::*;

/// Key for the map
#[derive(QueryData)]
#[query_data(derive(Debug))]
pub struct SyncProps {
    walkable: Has<Walkable>,
    opaque: Has<Opaque>,
    pickable: Has<Pickable>,
}

/// Sync [TileIdx] and [Sprite] visuals along with their gameplay properties.
pub fn sync_tiles(
    mut commands: Commands,
    mut tiles: Query<(Entity, &mut Sprite, &TileIdx, SyncProps), Changed<TileIdx>>,
) {
    // This method only runs when [TileIdx] or [TilePreview] changes, so we
    // apply most changes in some unconditional fashion.
    for (entity, mut sprite, tile_idx, sync_props) in tiles.iter_mut() {
        let mut entity_command = commands.entity(entity);

        let walkable = sync_props.walkable;
        let transparent = !sync_props.opaque;
        let pickable = sync_props.pickable;

        // Apply the texture atlas index unconditionally since it has changed.
        if let Some(texture_atlas) = &mut sprite.texture_atlas {
            texture_atlas.index = tile_idx.into();
        }

        // Update tile Walkable only when necessary.
        if tile_idx.is_walkable() && !walkable {
            entity_command.insert(Walkable);
        } else if !tile_idx.is_walkable() && walkable {
            entity_command.remove::<Walkable>();
        }

        // Update tile Opaque only when necessary.
        if tile_idx.is_transparent() && !transparent {
            entity_command.remove::<Opaque>();
        } else if !tile_idx.is_transparent() && transparent {
            entity_command.insert(Opaque);
        }

        if !pickable {
            entity_command.insert(Pickable {
                should_block_lower: false,
                is_hoverable: true,
            });
        }
    }
}

pub fn update_level_visuals(
    active_level: Single<(Entity, Ref<ActiveLevel>)>,
    all_levels: Query<(&Level, &mut Visibility)>,
) {
    let (active_level, ref active_ref) = *active_level;
    if !active_ref.is_changed() {
        return;
    }

    for (Level(level_nt, _), mut vis) in all_levels {
        if *level_nt == active_level {
            info!("Level active: {level_nt}");
            *vis = Visibility::Inherited;
        } else {
            info!("Level inactive: {level_nt}");
            *vis = Visibility::Hidden;
        }
    }
}

/// Sync [MapTile] [Sprite] visual effects with the tile's logical state. This
/// is orthogonal to [TileIdx].
pub fn update_tile_visuals(
    mut tiles: Query<(&mut Sprite, &mut Visibility, VisualProps, &ChildOf)>,
    level_light: Query<&AmbientLight, With<Level>>,
) {
    for (mut sprite, mut vis, t, child_of) in tiles.iter_mut() {
        let ambient = level_light
            .get(child_of.parent())
            .ok()
            .map(|al| al.0)
            .unwrap_or_default();

        *vis = if t.revealed() && !t.occupied() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };

        sprite.color = if t.highlighted() {
            colors::KENNEY_GOLD
        } else if t.revealed() && !t.occupied() {
            Color::WHITE.with_alpha(t.light_or(&ambient).into())
        } else {
            Color::NONE
        };
    }
}

#[derive(QueryData)]
#[query_data(derive(Debug))]
pub struct VisualProps {
    _mt: &'static MapTile,
    occupied: Option<&'static Occupied>,
    highlighted: Option<&'static Highlighted>,
    revealed: Option<&'static Revealed>,
    light_level: Option<&'static LightLevel>,
}

impl<'w, 's> VisualPropsItem<'w, 's> {
    pub fn revealed(&self) -> bool {
        self.revealed.is_some_and(|r| r.0)
    }

    pub const fn highlighted(&self) -> bool {
        self.highlighted.is_some()
    }

    pub fn light_or(&self, other: &LightLevel) -> LightLevel {
        *self.light_level.unwrap_or(other)
    }

    pub const fn occupied(&self) -> bool {
        self.occupied.is_some()
    }
}
