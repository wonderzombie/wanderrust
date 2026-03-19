use bevy::ecs::component::Component;

#[derive(Component, Debug, Default)]
pub struct CombatStats {
    pub nameplate: String,
    pub hp: i32,
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub is_dead: bool,
}
