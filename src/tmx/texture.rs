use std::hash::Hasher;
use std::sync::Arc;

use anyhow::*;
use async_mutex::Mutex;
use bevy::asset::{Handle, LoadContext, LoadedAsset};
use bevy::render::texture::{Extent3d, Texture as BevyTexture, TextureDimension, TextureFormat};
use image::{load_from_memory, GenericImage, RgbaImage};
use std::path::PathBuf;

/// A shared image
#[derive(Clone)]
pub struct Texture {
    data: Arc<Mutex<Inner>>,
    label: Arc<str>,
    width: u32,
    height: u32,
}

enum Inner {
    Defined { path: PathBuf },
    Decoded { buffer: RgbaImage },
    Loaded { handle: Handle<BevyTexture> },
}

pub(crate) struct TexturePtr(Arc<str>);

impl Texture {
    pub(crate) fn from_bytes(data: &[u8], label: impl Into<Arc<str>>) -> Result<Self> {
        let buffer = load_from_memory(data)?.to_rgba8();
        let width = buffer.width();
        let height = buffer.height();
        Ok(Texture {
            data: Arc::new(Mutex::new(Inner::Decoded { buffer })),
            label: label.into(),
            width,
            height,
        })
    }

    pub(crate) fn from_path(path: PathBuf) -> Self {
        let label = format!("{}", path.display()).into();
        Texture {
            data: Arc::new(Mutex::new(Inner::Defined { path })),
            label,
            width: 0,
            height: 1,
        }
    }

    pub(crate) async fn resize(&self, width: u32, height: u32) -> Result<Self> {
        if width != self.width && height != self.height {
            let data = self.data.lock().await;
            match &*data {
                Inner::Defined { path } => Ok(Texture {
                    data: Arc::new(Mutex::new(Inner::Defined { path: path.clone() })),
                    label: format!("{}#{}x{}", self.label, width, height).into(),
                    width,
                    height,
                }),
                Inner::Decoded { buffer } => {
                    let mut new_image: RgbaImage = RgbaImage::new(width, height);
                    new_image.copy_from(buffer, 0, 0)?;
                    Ok(Texture {
                        data: Arc::new(Mutex::new(Inner::Decoded { buffer: new_image })),
                        label: format!("{}#{}x{}", self.label, width, height).into(),
                        width,
                        height,
                    })
                }
                Inner::Loaded { .. } => unreachable!(),
            }
        } else {
            Ok(self.clone())
        }
    }

    pub(crate) async fn load(
        &self,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Handle<BevyTexture>> {
        let mut data = self.data.lock().await;

        let handle = match &mut *data {
            Inner::Defined { path } => {
                let mut buffer =
                    load_from_memory(load_context.read_asset_bytes(path).await?.as_slice())?
                        .to_rgba8();
                if self.width > 0 && self.height > 0 {
                    let mut new_image: RgbaImage = RgbaImage::new(self.width, self.height);
                    new_image.copy_from(&buffer, 0, 0)?;
                    buffer = new_image;
                }

                load_context.set_labeled_asset(
                    self.label.as_ref(),
                    LoadedAsset::new(BevyTexture::new(
                        Extent3d {
                            width: buffer.width(),
                            height: buffer.height(),
                            depth: 1,
                        },
                        TextureDimension::D2,
                        buffer.into_raw(),
                        TextureFormat::Rgba8Unorm,
                    )),
                )
            }
            Inner::Decoded { buffer } => load_context.set_labeled_asset(
                self.label.as_ref(),
                LoadedAsset::new(BevyTexture::new(
                    Extent3d {
                        width: self.width,
                        height: self.height,
                        depth: 1,
                    },
                    TextureDimension::D2,
                    std::mem::take(buffer).into_raw(),
                    TextureFormat::Rgba8Unorm,
                )),
            ),
            Inner::Loaded { handle } => handle.clone(),
        };

        *data = Inner::Loaded {
            handle: handle.clone(),
        };

        Ok(handle)
    }

    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    pub(crate) fn height(&self) -> u32 {
        self.height
    }
}

impl From<&Texture> for TexturePtr {
    fn from(image: &Texture) -> Self {
        Self(image.label.clone())
    }
}

impl std::hash::Hash for TexturePtr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl std::cmp::PartialEq for TexturePtr {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl std::cmp::Eq for TexturePtr {}
