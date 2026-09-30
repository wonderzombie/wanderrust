use bevy::prelude::*;

use crate::{
    cell::Cell,
    ldtk_loader::{LdtkCell, LdtkEntity},
    tilemap::Dimensions,
};

#[derive(Debug, PartialEq, Clone, Reflect)]
pub struct RouteId(pub String);

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
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
