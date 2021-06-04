use bevy::prelude::*;
use bevy::window::WindowMode;

use bevy_tmx::TmxPlugin;

fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            title: "Hexagonal".to_string(),
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
                .scale(Vec2::new(4.0, -4.0))
                .visit_objects(|object, _entity| {
                    println!("{:?}", object);
                }),
        )
        .add_startup_system(spawn_scene.system())
        .run()
}

fn spawn_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_scene(asset_server.load("hexagonal-mini.tmx"));
    commands.spawn().insert_bundle(OrthographicCameraBundle {
        transform: Transform::from_xyz(560.0, -380.0, 50.0),
        ..OrthographicCameraBundle::new_2d()
    });
}
