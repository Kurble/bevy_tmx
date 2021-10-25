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

/// Component and system for parallax rendering
#[cfg(feature = "plugin")]
pub mod parallax;
#[cfg(feature = "plugin")]
mod scene;
/// Representation of the .tmx file format
pub mod tmx;
#[cfg(feature = "plugin")]
mod plugin;

#[cfg(not(feature = "plugin"))]
mod loader {
    use super::tmx::Map;
    use anyhow::*;
    use std::sync::Arc;
    use std::path::{Path, PathBuf, Component};

    #[derive(Clone)]
    pub(crate) struct TmxLoadContext<'a> {
        relative: Arc<Path>,
        lifetime: &'a (),
    }

    impl<'a> TmxLoadContext<'a> {
        pub async fn load_file<'p>(&'p self, path: impl AsRef<Path> + Send + 'p) -> Result<Vec<u8>> {
            Ok(std::fs::read(self.file_path(path))?)
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
                lifetime: self.lifetime,
            }
        }
    }
    

    /// Load tmx::Map from a file.
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Map> {
        let path = path.as_ref();
        let context = ();
        let context = if let Some(parent) = path.parent() {
            TmxLoadContext {
                relative: Arc::from(parent.to_path_buf()),
                lifetime: &context,
            }
        } else {
            TmxLoadContext {
                relative: Path::new(".").to_path_buf().into(),
                lifetime: &context,
            }
        };

        let reader = xml::EventReader::new(std::fs::File::open(path)?);

        Ok(Map::load_from_xml_reader(context, reader).await?)
    }
}

#[cfg(feature = "plugin")]
pub use plugin::*;
#[cfg(not(feature = "plugin"))]
pub use loader::*;