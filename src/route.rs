use bevy::{ecs::system::SystemParam, prelude::*};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    cell::Cell,
    ldtk_loader::{LdtkCell, LdtkEntity},
    mobs,
    tilemap::Dimensions,
};

#[derive(Component, Debug, PartialEq, Clone, Reflect, Deserialize, Serialize, Eq)]
pub struct RouteId(pub String);

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[component(immutable)]
#[reflect(Component)]
pub struct Route {
    pub id: RouteId,
    pub points: Vec<Cell>,
}

impl Route {
    // Generates a new [`Route`]. In particular, it translates LDtk cell
    // ([`crate::ldtk_loader::LdtkCell`]) coordinates to wanderrust coordinates. In Bevy, Y+ is up;
    // in LDtk Y- is up.
    pub fn new(dimensions: &Dimensions, entity: &LdtkEntity) -> Self {
        let name = entity
            .get_name_field()
            .unwrap_or_else(|| format!("WaypointNameMissing"));

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

/// Type alias for selecting any kind of waypoint. This is useful because the first waypoint will
/// have `NextWaypoint` but not `PrevWaypoint`, and the last waypoint will have `PrevWaypoing` but
/// not `NextWaypoint`.
type AnyWaypoint = AnyOf<(&'static NextWaypoint, &'static PrevWaypoint)>;

#[derive(SystemParam)]
pub struct RouteAssist<'w, 's> {
    commands: Commands<'w, 's>,
    mobs: Query<'w, 's, (&'static Cell, &'static EnRouteTo), With<mobs::Mob>>,
    routes: Query<'w, 's, (&'static Route, &'static FirstWaypoint)>,
    waypoints: Query<'w, 's, (&'static Cell, AnyWaypoint, &'static ChildOf)>,
}

impl<'w, 's> RouteAssist<'w, 's> {
    pub fn assign(&mut self, assignee: Entity, route_id: RouteId) -> Result<()> {
        self.mobs.get(assignee)?;

        let (_, first_wp) = self
            .routes
            .iter()
            .find(|(route, _)| route.id.eq(&route_id))
            .ok_or_else(|| format!("{route_id:?}: no such route exists"))?;

        self.commands.entity(assignee).insert(EnRouteTo(**first_wp));

        Ok(())
    }

    /// Returns the next waypoint for an entity [`Following`] a route, which may be their current
    /// destination [`Cell`] if they've yet to reach it, or the next waypoint if they have reached
    /// the current waypoint's [`Cell`].
    ///
    /// Suitable for use with `set_if_neq` when [`Cell`] changes.
    pub fn next_waypoint(&self, follower: Entity) -> Result<Entity> {
        let (mob_cell, en_route_to) = self.mobs.get(follower)?;
        let (wp_cell, (next_opt, _), child_of) = self.waypoints.get(**en_route_to)?;

        if mob_cell != wp_cell {
            return Ok(**en_route_to);
        }

        match next_opt {
            Some(next_wp) => Ok(**next_wp),
            None => {
                let (_, first_wp) = self.routes.get(child_of.parent())?;
                Ok(**first_wp)
            }
        }
    }
}

/// Applied to a [`Route`] entity to get the first waypoint.
#[derive(Component, Deref)]
pub struct FirstWaypoint(pub Entity);

/// [`EnRouteTo`] marks one entity as pursuing the next waypoint identified by [`Entity`].
#[derive(Component, Debug, Deref, Reflect)]
#[reflect(Component)]
pub struct EnRouteTo(pub Entity);

/// [`NextWaypoint`] marks an entity as part of a chain of waypoints, identifying the next entity.
/// The last waypoint in the list will not (must not) have this component.
#[derive(Component, Reflect, Debug, Deref)]
#[relationship(relationship_target = PrevWaypoint)]
#[reflect(Component)]
pub struct NextWaypoint(pub Entity);

/// [`PrevWaypoint`] marks an entity as preceding another entity in a chain of waypoints.
/// The first waypoint in the list will not (must not) have this component.
#[derive(Component, Reflect, Debug, Deref)]
#[relationship_target(relationship = NextWaypoint)]
#[reflect(Component)]
pub struct PrevWaypoint(Entity);

// /// [`Following`] identifies an [`Entity`] as following a particular [`Route`] entity's points.
// #[derive(Component, Reflect, Debug, Deref)]
// #[relationship(relationship_target = Followers)]
// #[reflect(Component)]
// pub struct Following(pub Entity);

// /// [`Followers`] is a list of entities that are following this particular [`Route`].
// #[derive(Component, Reflect, Debug)]
// #[relationship_target(relationship = Following)]
// #[reflect(Component)]
// pub struct Followers(Vec<Entity>);

////// Commands //////

/// Spawns waypoints for the [`Route`] as a child of [`Entity`], presumably a [`Level`].
pub struct SpawnWaypoints(pub Entity, pub Route);

impl Command for SpawnWaypoints {
    type Out = Result<()>;

    fn apply(self, world: &mut World) -> Self::Out {
        let route_id = world.spawn_empty().id();

        let point_ids = &self
            .1
            .points
            .iter()
            .map(|c| world.spawn((*c, ChildOf(route_id))).id())
            .collect_vec();

        for (prev_id, next_id) in point_ids.iter().tuple_windows() {
            world.entity_mut(*prev_id).insert(NextWaypoint(*next_id));
        }

        let first_wp = point_ids
            .first()
            .ok_or_else(|| "unable to access first waypoint entity id?!")?;

        world
            .entity_mut(route_id)
            .insert((ChildOf(self.0), self.1, FirstWaypoint(*first_wp)));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::*;

    fn insert_route(app: &mut App, new_route: Route) {
        let parent = app.world_mut().commands().spawn_empty().id();
        app.world_mut()
            .commands()
            .queue(SpawnWaypoints(parent, new_route));
    }

    fn points() -> Vec<Cell> {
        vec![
            Cell::new(0, 0),
            Cell::new(2, 0),
            Cell::new(2, 2),
            Cell::new(0, 2),
        ]
    }

    fn setup() -> App {
        let mut app = init_app();
        insert_route(
            &mut app,
            Route {
                id: RouteId("route_one".to_string()),
                points: points(),
            },
        );
        app.update();
        app
    }

    #[test]
    fn test_spawn_waypoints_route_has_children() -> Result<()> {
        let mut app = setup();

        let route_nt = app
            .world_mut()
            .query_filtered::<Entity, With<Route>>()
            .single(app.world())?;

        let children = app
            .world()
            .get::<Children>(route_nt)
            .expect("expected route entity to have Children component")
            .collection();

        assert_eq!(4, children.len());

        Ok(())
    }

    #[test]
    fn test_spawn_waypoints_first_waypoint() -> Result<()> {
        let mut app = setup();

        let (route_nt, first_wp) = app
            .world_mut()
            .query_filtered::<(Entity, &FirstWaypoint), With<Route>>()
            .single(app.world())
            .expect("expected exactly one entity with first waypoint");

        let children = app
            .world()
            .get::<Children>(route_nt)
            .expect("expected route entity to have Children component")
            .collection()
            .clone();

        let first_child = children.first().expect("expected Children not to be empty");

        assert_eq!(*first_child, first_wp.0);

        let second_child = children
            .get(1)
            .expect("expected Children two have at least two entities");

        let next_wp = app
            .world()
            .get::<NextWaypoint>(*first_child)
            .expect("expected first child to have NextWaypoint");

        assert_eq!(*second_child, next_wp.0);

        Ok(())
    }

    #[test]
    fn test_spawn_waypoints_waypoint_cells() -> Result<()> {
        let mut app = setup();

        // `setup()` uses this to initialize waypoints.
        let points = points();

        let route_nt = app
            .world_mut()
            .query_filtered::<Entity, With<Route>>()
            .single(app.world())
            .expect("expected exactly one entity with Route");

        let children = app
            .world()
            .get::<Children>(route_nt)
            .expect("expected Children to be present on Route entity");

        for (i, cell) in points.iter().enumerate() {
            let wp_nt = children
                .get(i)
                .expect("expected an equal number of Children and Cells");

            let actual_cell = app
                .world()
                .get::<Cell>(*wp_nt)
                .expect("expected each waypoint to have a Cell");

            assert_eq!(cell, actual_cell);
        }

        Ok(())
    }

    #[test]
    fn test_spawn_waypoints_next_and_prev_waypoint() {
        let mut app = setup();

        let children = app
            .world_mut()
            .query_filtered::<&Children, With<Route>>()
            .single(app.world())
            .expect("expected exactly one entity with first waypoint");

        let world = app.world();

        // This presumes a list of waypoints greater than one.
        for (i, (left, right)) in children.collection().iter().tuple_windows().enumerate() {
            let the_next_wp = world
                .get::<NextWaypoint>(*left)
                .expect("expected not-last-child of Route to have NextWaypoint");

            assert_eq!(
                *right, the_next_wp.0,
                "{left} @ {i} did not point to {right}"
            );

            let the_prev_wp = world
                .get::<PrevWaypoint>(*right)
                .expect("expected next waypoint's PrevWaypoint to exist");

            assert_eq!(
                *left, the_prev_wp.0,
                "{right} @ {i} did not point back to {left}"
            );
        }

        let last_wp = children
            .last()
            .expect("expected there to be a last waypoint in list");

        assert_eq!(world.entity(*last_wp).contains::<NextWaypoint>(), false);
    }

    #[test]
    fn test_spawn_waypoints_with_next_without_prev() -> Result<()> {
        let mut app = setup();

        let no_predecessor = app
            .world_mut()
            .query_filtered::<Entity, (With<NextWaypoint>, Without<PrevWaypoint>)>()
            .single(app.world())?;

        let first = app
            .world_mut()
            .query::<&FirstWaypoint>()
            .single(app.world())?;

        assert_eq!(no_predecessor, first.0);

        Ok(())
    }

    #[test]
    fn test_spawn_waypoints_without_next_with_prev() -> Result<()> {
        let mut app = setup();

        let no_successor = app
            .world_mut()
            .query_filtered::<Entity, (With<PrevWaypoint>, Without<NextWaypoint>)>()
            .single(app.world())?;

        let last = app
            .world_mut()
            .query_filtered::<&Children, With<Route>>()
            .single(app.world())?
            .last()
            .expect("expected Children to have a non-zero number of entities");

        assert_eq!(no_successor, *last);

        Ok(())
    }
}
