use bevy::prelude::*;
use bevy::window::WindowMode;

use bevy_tmx::TmxPlugin;

fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            title: "Isometric".to_string(),
            width: 1024.,
            height: 720.,
            vsync: false,
            resizable: true,
            mode: WindowMode::Windowed,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_plugin(
            TmxPlugin::default()
                .scale(Vec2::new(2.0, -2.0))
                .visit_objects(|object, _entity| {
                    println!("{:?}", object);
                }),
        )
        .add_startup_system(spawn_scene.system())
        .run()
}

fn spawn_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_scene(asset_server.load("isometric_grass_and_water.tmx"));
    commands.spawn().insert_bundle(OrthographicCameraBundle {
        transform: Transform::from_xyz(1600.0, -800.0, 50.0),
        ..OrthographicCameraBundle::new_2d()
    });
}
