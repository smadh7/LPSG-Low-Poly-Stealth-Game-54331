//! HITMAN — Coral Palm Resort: entry point.
//!
//! Everything else lives in the library crate so it can be unit tested and
//! reused by tooling. This file's only real job (apart from starting Bevy) is to
//! make the game **portable**: assets are located relative to the executable,
//! with fallbacks for running from a Cargo checkout.

use bevy::prelude::*;
use hitman_lowpoly::GamePlugin;
use std::path::{Path, PathBuf};

/// Find the directory that contains the `textures/` folder.
///
/// Search order, first hit wins:
/// 1. `HITMAN_ASSET_ROOT` (explicit override for tests and tooling).
/// 2. `<exe dir>/assets` — the shipped layout: `Game.exe` + `assets/`.
/// 3. `<exe dir>` itself, for a build whose assets were copied next to it.
/// 4. `<cwd>/assets` and `<cwd>`, so `cargo run` from the repository works.
/// 5. `../assets` and `../../assets`, for `target/debug/game.exe` layouts.
fn locate_asset_root() -> PathBuf {
    if let Ok(explicit) = std::env::var("HITMAN_ASSET_ROOT") {
        let path = PathBuf::from(explicit);
        if path.join("textures").is_dir() {
            return path;
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("assets"));
            candidates.push(dir.to_path_buf());
            candidates.push(dir.join("..").join("assets"));
            candidates.push(dir.join("..").join("..").join("assets"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("assets"));
        candidates.push(cwd.clone());
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        candidates.push(Path::new(&manifest).join("assets"));
    }

    for candidate in &candidates {
        if candidate.join("textures").is_dir() {
            return candidate.clone();
        }
    }
    // Nothing found: return the most likely location so the error message is
    // actionable rather than a silent black screen.
    PathBuf::from("assets")
}

fn main() -> AppExit {
    let asset_root = locate_asset_root();
    // Bevy's asset server resolves paths against this environment variable, which
    // is exactly how a shipped build ends up reading `<game folder>/assets`.
    std::env::set_var("BEVY_ASSET_ROOT", &asset_root);
    if !asset_root.join("textures").is_dir() {
        eprintln!(
            "warning: could not find an 'assets' folder next to the executable.\n\
             Looked in: {}\n\
             The game will start, but models and textures will be missing.",
            asset_root.display()
        );
    }

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    // Paths inside the game are written as "textures/x.png", and
                    // the base path above already points at the `assets` folder.
                    file_path: String::new(),
                    // A shipped game has no .meta files; never look for them.
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "HITMAN — Coral Palm Resort".into(),
                        resolution: (1600.0, 900.0).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(GamePlugin)
        .run()
}
