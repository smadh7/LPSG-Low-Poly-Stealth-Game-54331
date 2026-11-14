use bevy::prelude::*;
use hitman_lowpoly::GamePlugin;

fn main() -> AppExit {
    App::new().add_plugins(GamePlugin).run()
}
