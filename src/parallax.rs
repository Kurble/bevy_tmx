use bevy_ecs::{reflect::ReflectComponent, system::Query};
use bevy_math::{vec3, Vec2};
use bevy_transform::components::{GlobalTransform, Transform};
use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::camera::Camera;

/// Component for sprites that should render according to a parallax relative to the camera.
/// Note that the parallax_transform_system will overwrite the `Transform` component,
///  so if you want to modify the transform of an entity that has a `Parallax` component you should
///  modify the `transform` field of `Parallax` instead of modifying the `Transform` component directly.
#[derive(Debug, Default, Clone, TypeUuid, Reflect)]
#[reflect(Component)]
#[uuid = "0e436fcb-7b34-420c-92df-6fda230332d8"]
pub struct Parallax {
    /// Parallax factor per axis.
    /// Factors below 1.0 will make entities appear further away from the camera,
    ///  while factors above 1.0 will make them appear closer.
    /// You can think of the camera as being on factor 1.0.
    pub factor: Vec2,
    /// The source transform to use when performing parallax transformation.
    pub transform: Transform,
}

/// System that updates the `Transform` component of `Parallax` entities.
pub fn parallax_transform_system(
    cameras: Query<(&GlobalTransform, &Camera)>,
    mut parallax: Query<(&mut Transform, &Parallax)>,
) {
    if let Some((camera_transform, _camera)) = cameras.iter().next() {
        let translation = camera_transform.translation;

        for (mut transform, parallax) in parallax.iter_mut() {
            transform.translation = parallax.transform.translation
                + translation * vec3(1.0, 1.0, 0.0)
                - translation * parallax.factor.extend(0.0);
            transform.rotation = parallax.transform.rotation;
            transform.scale = parallax.transform.scale;
        }
    }
}

impl Parallax {
    /// Construct a new `Parallax`.
    pub fn new(factor: Vec2, transform: Transform) -> Self {
        Self { factor, transform }
    }
}
