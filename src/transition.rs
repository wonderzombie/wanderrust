use bevy::ecs::{component::Component, resource::Resource};
use serde::{Deserialize, Serialize};

use crate::{cell::Cell, light::LightLevel};

/// Uniquely identifies a Zone.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct ZoneId(pub String);

/// Uniquely identifies an Entry within a Zone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EntryId(pub String);

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub id: ZoneId,
    pub name: String,
    pub map_path: String,
    pub ambient_light: LightLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntrySpec {
    pub id: EntryId,
    pub cell: Cell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExitSpec {
    pub cell: Cell,
    pub zone: ZoneId,
    pub arrive_at: EntryId,
}

#[derive(Component, Debug, Clone)]
pub struct EntryPoint {
    pub id: EntryId,
}

#[derive(Component, Debug, Clone)]
pub struct Exit {
    pub zone: ZoneId,
    pub arrive_at: EntryId,
}

#[derive(Resource, Debug)]
pub struct PendingTransition {
    pub zone: ZoneId,
    pub arrive_at: EntryId,
}
