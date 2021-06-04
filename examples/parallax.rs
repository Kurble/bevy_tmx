use bevy::prelude::*;
use bevy::window::WindowMode;

use bevy::render::camera::Camera;
use bevy_tmx::TmxPlugin;

fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            title: "Parallax".to_string(),
            width: 1024.,
            height: 720.,
            vsync: true,
            resizable: true,
            mode: WindowMode::Windowed,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::TEAL))
        .add_plugins(DefaultPlugins)
        .add_plugin(TmxPlugin::default())
        .add_startup_system(spawn_scene.system())
        .add_system(circle_camera.system())
        .run()
}

fn spawn_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_scene(asset_server.load("sticker/sandbox.tmx"));
    commands.spawn().insert_bundle(OrthographicCameraBundle {
        transform: Transform::from_xyz(600.0, -600.0, 50.0),
        ..OrthographicCameraBundle::new_2d()
    });
}

fn circle_camera(time: Res<Time>, mut camera: Query<(&mut Transform, &Camera)>) {
    let rads = time.seconds_since_startup();
    for (mut transform, _camera) in camera.iter_mut() {
        transform.translation.x = 1264.0 + rads.cos() as f32 * 600.0;
        transform.translation.y = -720.0 + rads.sin() as f32 * 200.0;
    }
}
