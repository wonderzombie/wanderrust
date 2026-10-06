use anyhow::Context;
use bevy::{ecs::system::SystemParam, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    cell::Cell,
    ldtk_loader::{LdtkCell, LdtkEntity},
    tilemap::Dimensions,
};

pub fn plugin(app: &mut App) {
    app.add_observer(on_cell_inserted);
}

/// [`Cell`] is immutable so [`Insert`] is a mutation.
pub fn on_cell_inserted(
    on: On<Insert, Cell>,
    mut route_assist: RouteAssist,
    cells: Query<&Cell, (With<crate::mobs::Mob>, With<OnRoute>)>,
) -> Result<()> {
    let dest = route_assist
        .destination(on.event_target())
        .with_severity(Severity::Error)?;
    let curr = cells
        .get(on.event_target())
        .with_severity(Severity::Error)?;

    if dest == *curr {
        route_assist
            .advance(on.event_target())
            .with_severity(Severity::Error)?;
    }

    Ok(())
}

#[derive(Component, Debug, PartialEq, Clone, Reflect, Deserialize, Serialize, Eq)]
pub struct RouteId(pub String);

impl From<&str> for RouteId {
    fn from(value: &str) -> Self {
        Self(String::from(value))
    }
}

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
        self.mobs
            .get(follower)
            .context("unable to find follower among mobs")?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mobs::Mob, tiles::TileIdx};
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn test_assign_route_destination() -> Result {
        let mut world = World::new();

        let cells = vec![Cell::new(1, 1), Cell::new(2, 2)];

        let route = world
            .spawn(Route {
                id: "the_route".into(),
                points: cells.clone(),
            })
            .id();

        let mob = world
            .spawn((
                Mob {
                    name: "bat".into(),
                    tile_idx: TileIdx::Bat,
                    route_id: Some("the_route".into()),
                },
                Cell::new(0, 0),
            ))
            .id();

        world
            .run_system_once::<_, (), _>(move |mut assist: RouteAssist| {
                assist.assign(mob, &"the_route".into())
            })
            .unwrap();

        let dest: Cell = world
            .run_system_once(move |assist: RouteAssist| assist.destination(mob))
            .unwrap();

        world.flush();

        let on_route = world.query::<&OnRoute>().single(&world).unwrap();

        assert_eq!(on_route.route, route);
        assert_eq!(on_route.idx, 0);
        assert_eq!(cells[on_route.idx], dest);

        Ok(())
    }

    #[test]
    fn test_assign_route_advance() {
        let mut world = World::new();

        let cells = vec![Cell::new(1, 1), Cell::new(2, 2)];

        world.spawn(Route {
            id: "the_route".into(),
            points: cells.clone(),
        });

        let mob = world
            .spawn((
                Mob {
                    name: "bat".into(),
                    tile_idx: TileIdx::Bat,
                    route_id: Some("the_route".into()),
                },
                Cell::new(0, 0),
            ))
            .id();

        world
            .run_system_once::<_, (), _>(move |mut assist: RouteAssist| {
                assist.assign(mob, &"the_route".into())
            })
            .unwrap();

        // Advance once, beyond index 0
        world
            .run_system_once::<_, (), _>(move |mut assist: RouteAssist| {
                assist.advance(mob).unwrap();
            })
            .unwrap();

        world.flush();

        let idx = idx_for(&world, mob);
        assert_eq!(idx, 1);

        let dest: Cell = world
            .run_system_once(move |assist: RouteAssist| assist.destination(mob))
            .unwrap();
        assert_eq!(cells[idx], dest);

        // As if we have wrapped around a two-waypoint-route
        world
            .run_system_once::<_, (), _>(move |mut assist: RouteAssist| {
                assist.advance(mob).unwrap();
            })
            .unwrap();
        let idx = idx_for(&world, mob);
        assert_eq!(idx, 0);

        let dest: Cell = world
            .run_system_once(move |assist: RouteAssist| assist.destination(mob))
            .unwrap();
        assert_eq!(cells[idx], dest);
    }

    fn idx_for(w: &World, e: Entity) -> usize {
        w.entity(e).get::<OnRoute>().unwrap().idx
    }

    #[test]
    fn test_on_cell_inserted() -> Result {
        let mut world = World::default();

        let chicken = world
            .spawn((
                Mob {
                    name: "Poppy".into(),
                    tile_idx: TileIdx::Chicken,
                    route_id: Some("fake_route".into()),
                },
                Cell::new(0, 0),
            ))
            .id();

        world.spawn(Route {
            id: "fake_route".into(),
            points: vec![Cell::new(0, 2), Cell::new(2, 2)],
        });

        world.flush();

        world
            .run_system_once::<_, (), _>(move |mut assist: RouteAssist| {
                assist.assign(chicken, &"fake_route".into())
            })
            .unwrap();

        world.add_observer(on_cell_inserted);

        // As if we're progressing along the route.
        world.entity_mut(chicken).insert(Cell::new(0, 1));
        world.flush();
        assert_eq!(idx_for(&world, chicken), 0);

        // As if we've arrived at the first waypoint.
        world.entity_mut(chicken).insert(Cell::new(0, 2));
        world.flush();
        assert_eq!(idx_for(&world, chicken), 1);

        // As if we've made a massive detour for some reason.
        // Regardless, our progress remains the same.
        world.entity_mut(chicken).insert(Cell::new(0, 22));
        world.flush();
        assert_eq!(idx_for(&world, chicken), 1);

        Ok(())
    }
}
