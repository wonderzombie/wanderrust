use bevy::ecs::resource::Resource;

#[derive(Resource, Debug)]
pub struct PlayerStats {
    pub vision_range: u32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self { vision_range: 5 }
    }
}

impl PlayerStats {
    const DEFAULT_VISION: u32 = 5;

    pub fn set_vision_range(&mut self, vision_range: u32) {
        self.vision_range = vision_range;
    }

    pub fn reset_vision_range(&mut self) {
        self.vision_range = PlayerStats::DEFAULT_VISION;
    }

    pub fn is_default(&self) -> bool {
        self.vision_range == PlayerStats::DEFAULT_VISION
    }
}
