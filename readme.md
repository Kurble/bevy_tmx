# bevy_tmx
[![Documentation](https://docs.rs/bevy_tmx/badge.svg)](https://docs.rs/bevy_tmx)
[![Crates.io](https://img.shields.io/crates/v/bevy_tmx.svg)](https://crates.io/crates/bevy_tmx)
![License](https://img.shields.io/crates/l/bevy_tmx.svg)

bevy_tmx is a plugin for the bevy game engine that allows you to read .tmx files from the [tiled map editor](https://www.mapeditor.org/) as scenes. The plugin can be configured so that you can add more of your own components to the entities of the scene.

Currently, the tile maps being rendered are fairly simple, they are loaded as simple sprite entities, one per layer and sprite sheet.

# Features
- All tile layout modes supported by tiled:
    - Orthogonal
    - Isometric staggered and non-staggered
    - Hexagonal staggered
- Object layers with support for custom object processing
- Image layers with support for custom image layer processing
- Parallax rendering
 
# Todo
- Infinite map support
- All render orders other than `RightDown`

# Overview
Using bevy_tmx is supposed to be really simple, just add the `TmxPlugin` to your `App` and load a scene. 
If you need to add custom functionality to the entities loaded from the `.tmx` file, you can customize the `TmxLoader` to do so during load time.

### Example
```rust
use bevy::prelude::*;
use bevy::window::WindowMode;

use bevy_tmx::TmxPlugin;

struct PlayerComponent;

fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            title: "Ortho".to_string(),
            width: 1024.,
            height: 720.,
            vsync: false,
            resizable: true,
            mode: WindowMode::Windowed,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(TmxPlugin::default()
            // Note that in tiled, the y axis points down, but in bevy it points up. The default scale is (1.0, -1.0).
            .scale(Vec2::new(3.0, -3.0))
            // This is the place to add more functionality to your objects
            .visit_objects(|object, entity| {
                if object.ty == "player" {
                    entity.insert(PlayerComponent);
                }
            })
        )
        .add_startup_system(spawn_scene.system())
        .run()
}

fn spawn_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_scene(asset_server.load("ortho-map.tmx"));
    commands.spawn().insert_bundle(OrthographicCameraBundle {
        transform: Transform::from_xyz(600.0, -600.0, 50.0),
        ..OrthographicCameraBundle::new_2d()
    });
}
```