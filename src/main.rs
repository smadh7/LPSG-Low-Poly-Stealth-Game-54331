//! HITMAN LOW-POLY — entry point.
//!
//! Everything else lives in the library crate so it can be unit tested and,
//! later, reused by tooling (e.g. a mission editor or a headless test harness).

use bevy::prelude::*;
use hitman_lowpoly::GamePlugin;

fn main() -> AppExit {
    App::new().add_plugins(GamePlugin).run()
}
