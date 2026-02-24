use bevy::ecs::resource::Resource;

#[derive(Resource, Debug)]
pub struct PlayerStats {
    pub vision_range: u32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self { vision_range: 10 }
    }
}
