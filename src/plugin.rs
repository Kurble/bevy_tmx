use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use anyhow::*;
use bevy_app::{Plugin, AppBuilder};
use bevy_asset::{AssetLoader, AddAsset, BoxedFuture, LoadContext, LoadedAsset};
use bevy_ecs::{world::{EntityMut, World}, system::IntoSystem};
use bevy_math::*;

use crate::tmx::{Map, Object};
use crate::parallax::{parallax_transform_system, Parallax};
use crate::scene::{proto_sprite_upgrade_system, ProtoSprite, SceneBuilder, ObjectVisitor, ImageVisitor, MapVisitor};

/// Plugin that adds support for .tmx asset loading. Loading behaviour can be customized on creation.
pub struct TmxPlugin {
    object_visitor: Option<Arc<ObjectVisitor>>,
    image_visitor: Option<Arc<ImageVisitor>>,
    map_visitor: Option<Arc<MapVisitor>>,
    scale: Vec3,
}

#[derive(Default)]
struct TmxSceneLoader {
    object_visitor: Option<Arc<ObjectVisitor>>,
    image_visitor: Option<Arc<ImageVisitor>>,
    map_visitor: Option<Arc<MapVisitor>>,
    scale: Vec3,
}

#[derive(Clone)]
pub(crate) struct TmxLoadContext<'a> {
    relative: Arc<Path>,
    context: &'a LoadContext<'a>,
}

impl TmxPlugin {
    /// Adds some custom loading functionality for objects in tmx assets
    pub fn visit_objects<F: 'static + for<'w> Fn(&Object, &mut EntityMut<'w>) + Send + Sync>(
        mut self,
        f: F,
    ) -> Self {
        self.object_visitor = Some(Arc::new(f));
        self
    }

    /// Adds some custom loading functionality for image layers in tmx assets
    pub fn visit_images<F: 'static + for<'w> Fn(&mut EntityMut<'w>) + Send + Sync>(
        mut self,
        f: F,
    ) -> Self {
        self.image_visitor = Some(Arc::new(f));
        self
    }

    /// Allows to modify the `World` loaded from a .tmx asset right before it's converted to a `Scene`.
    pub fn visit_map<F: 'static + for<'w> Fn(&Map, &mut World) + Send + Sync>(
        mut self,
        f: F,
    ) -> Self {
        self.map_visitor = Some(Arc::new(f));
        self
    }

    /// Sets the scale to apply to the coordinate system of loaded .tmx assets. Defaults to (1, -1), since bevy's y axis points up where tiled's y axis points down.
    pub fn scale(mut self, scale: Vec2) -> Self {
        self.scale.x = scale.x;
        self.scale.y = scale.y;
        self
    }

    /// Sets the depth added after each layer. Defaults to 1.
    pub fn depth_scale(mut self, depth_scale: f32) -> Self {
        self.scale.z = depth_scale;
        self
    }
}

impl Plugin for TmxPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.register_type::<ProtoSprite>();
        app.register_type::<Parallax>();
        app.add_asset::<Map>();

        let asset_loader = TmxSceneLoader {
            object_visitor: self.object_visitor.clone(),
            image_visitor: self.image_visitor.clone(),
            map_visitor: self.map_visitor.clone(),
            scale: self.scale,
        };

        app.add_asset_loader(asset_loader);
        app.add_system(proto_sprite_upgrade_system.system());
        app.add_system(parallax_transform_system.system());
    }
}

impl AssetLoader for TmxSceneLoader {
    fn load<'a, 'b>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext<'b>,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let env = TmxLoadContext {
                relative: Arc::from(
                    load_context
                        .path()
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .to_path_buf(),
                ),
                context: load_context,
            };

            let map = Map::load_from_xml_reader(env, xml::EventReader::new(bytes)).await?;
            let builder = SceneBuilder::new(
                load_context,
                &map,
                self.object_visitor.as_deref(),
                self.image_visitor.as_deref(),
                self.map_visitor.as_deref(),
                self.scale,
            );
            let scene = builder.build().await?;

            load_context.set_labeled_asset("map", LoadedAsset::new(map));
            load_context.set_default_asset(LoadedAsset::new(scene));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["tmx"]
    }
}

impl Default for TmxPlugin {
    fn default() -> Self {
        TmxPlugin {
            object_visitor: None,
            image_visitor: None,
            map_visitor: None,
            scale: Vec3::new(1.0, -1.0, 1.0),
        }
    }
}

impl<'a> TmxLoadContext<'a> {
    pub async fn load_file<'p>(&'p self, path: impl AsRef<Path> + Send + 'p) -> Result<Vec<u8>> {
        Ok(self.context.read_asset_bytes(self.file_path(path)).await?)
    }

    pub fn file_path(&self, path: impl AsRef<Path>) -> PathBuf {
        let mut joined = PathBuf::new();
        for c in self.relative.join(path.as_ref()).components() {
            match c {
                Component::Prefix(prefix) => joined.push(prefix.as_os_str()),
                Component::RootDir => joined.push("/"),
                Component::CurDir => (),
                Component::ParentDir => {
                    joined.pop();
                }
                Component::Normal(c) => joined.push(c),
            }
        }
        joined
    }

    pub fn file_directory(&self, path: impl AsRef<Path>) -> Self {
        Self {
            relative: if let Some(parent) = path.as_ref().parent() {
                Arc::from(self.relative.join(parent))
            } else {
                self.relative.clone()
            },
            context: self.context,
        }
    }
}
