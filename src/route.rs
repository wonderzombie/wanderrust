use bevy::{ecs::system::SystemParam, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    cell::Cell,
    ldtk_loader::{LdtkCell, LdtkEntity},
    tilemap::Dimensions,
};

pub fn plugin(app: &mut App) {
    app.add_observer(on_cell_update);
}

pub fn on_cell_update(
    on: On<Insert, Cell>,
    mut route_assist: RouteAssist,
    cells: Query<&Cell, With<crate::mobs::Mob>>,
) -> Result<()> {
    let dest_cell = route_assist.destination(on.event_target())?;
    let curr_cell = cells.get(on.event_target())?;

    if dest_cell == *curr_cell {
        route_assist.advance(on.event_target())?;
    }

    Ok(())
}

#[derive(Component, Debug, PartialEq, Clone, Reflect, Deserialize, Serialize, Eq)]
pub struct RouteId(pub String);

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[component(immutable)]
#[reflect(Component)]
pub struct Route {
    pub id: RouteId,
    pub points: Vec<Cell>,
}

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component)]
pub struct OnRoute {
    pub route: Entity,
    pub idx: usize,
}

impl Route {
    // Generates a new [`Route`]. In particular, it translates LDtk cell
    // ([`crate::ldtk_loader::LdtkCell`]) coordinates to wanderrust coordinates. In Bevy, Y+ is up;
    // in LDtk Y- is up.
    pub fn new(dimensions: &Dimensions, entity: &LdtkEntity) -> Self {
        let name = entity
            .get_name_field()
            .or_else(|| entity.get_string("route_id"))
            .unwrap_or_else(|| format!("RouteNameMissing"));

        let points: Vec<Cell> = entity
            .get_point_array("waypoints")
            .unwrap_or_default()
            .iter()
            .map(|it| {
                let l_cell: LdtkCell = it.into();
                l_cell.to_wandrs(dimensions.height.cast_signed(), dimensions.depth)
            })
            .collect();

        Self {
            id: RouteId(name),
            points,
        }
    }
}

#[derive(SystemParam)]
pub struct RouteAssist<'w, 's> {
    commands: Commands<'w, 's>,
    routes: Query<'w, 's, (Entity, &'static Route)>,
    mobs: Query<'w, 's, &'static Cell, With<crate::mobs::Mob>>,
    on_routes: Query<'w, 's, &'static mut OnRoute>,
}

impl<'w, 's> RouteAssist<'w, 's> {
    pub fn assign(&mut self, follower: Entity, route_id: &RouteId) -> Result<()> {
        self.mobs.get(follower)?;

        let (route_nt, _) = self
            .routes
            .iter()
            .find(|(_, r)| r.id.eq(route_id))
            .ok_or_else(|| {
                format!("failed to assign route for {follower}: unable to find route {route_id:?}")
            })?;

        self.commands.entity(follower).insert(OnRoute {
            route: route_nt,
            idx: 0usize,
        });

        Ok(())
    }

    pub fn destination(&self, follower: Entity) -> Result<Cell> {
        let OnRoute { route, idx } = self.on_routes.get(follower)?;
        let (_, route) = self.routes.get(*route)?;
        Ok(route.points[*idx])
    }

    pub fn advance(&mut self, follower: Entity) -> Result<()> {
        let mut on_route = self.on_routes.get_mut(follower)?;
        let (_, route) = self.routes.get(on_route.route)?;
        on_route.idx = (on_route.idx + 1) % route.points.len();
        Ok(())
    }
}
