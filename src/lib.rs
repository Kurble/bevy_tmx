//! # bevy_tmx
//! bevy_tmx is a plugin for the bevy game engine that allows you to read .tmx files from the [tiled map editor](https://www.mapeditor.org/) as scenes.
//! The plugin can be configured so that you can add more of your own components to the entities of the scene.
//!
//! Currently, the tile maps being rendered are fairly simple, they are loaded as simple sprite entities, one per layer and sprite sheet.
//!
//! # Features
//! - All tile layout modes supported by tiled:
//!     - Orthogonal
//!     - Isometric staggered and non-staggered
//!     - Hexagonal staggered
//! - Object layers with support for custom object processing
//! - Image layers with support for custom image layer processing
//! - Parallax rendering
//!  
//! # Todo
//! - Infinite map support
//! - All render orders other than `RightDown`
//!
//! # Overview
//! Using bevy_tmx is supposed to be really simple, just add the `TmxPlugin` to your `App` and load a scene.
//! If you need to add custom functionality to the entities loaded from the `.tmx` file, you can customize the `TmxLoader` to do so during load time.

#![deny(missing_docs)]

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use anyhow::*;
use bevy_app::{Plugin, AppBuilder};
use bevy_asset::{AssetLoader, AddAsset, BoxedFuture, LoadContext, LoadedAsset};
use bevy_ecs::{world::{EntityMut, World}, system::IntoSystem};
use bevy_math::*;

use tmx::{Map, Object};

use crate::parallax::{parallax_transform_system, Parallax};
use crate::scene::{proto_sprite_upgrade_system, ProtoSprite, SceneBuilder};

/// Component and system for parallax rendering
pub mod parallax;
mod scene;
/// Representation of the .tmx file format
pub mod tmx;

/// Plugin that adds support for .tmx asset loading. Loading behaviour can be customized on creation.
pub struct TmxPlugin {
    object_visitor: Option<Arc<scene::ObjectVisitor>>,
    image_visitor: Option<Arc<scene::ImageVisitor>>,
    map_visitor: Option<Arc<scene::MapVisitor>>,
    scale: Vec3,
}

#[derive(Default)]
struct TmxSceneLoader {
    object_visitor: Option<Arc<scene::ObjectVisitor>>,
    image_visitor: Option<Arc<scene::ImageVisitor>>,
    map_visitor: Option<Arc<scene::MapVisitor>>,
    scale: Vec3,
}

#[derive(Clone)]
struct TmxLoadContext<'a> {
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
    async fn load_file<'p>(&'p self, path: impl AsRef<Path> + Send + 'p) -> Result<Vec<u8>> {
        Ok(self.context.read_asset_bytes(self.file_path(path)).await?)
    }

    fn file_path(&self, path: impl AsRef<Path>) -> PathBuf {
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

    fn file_directory(&self, path: impl AsRef<Path>) -> Self {
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
