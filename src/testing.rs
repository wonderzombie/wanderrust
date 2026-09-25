use bevy::prelude::*;

pub fn init_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app
}
