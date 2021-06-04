use std::hash::Hasher;
use std::ops::Deref;
use std::sync::Arc;

use anyhow::*;
use image::{load_from_memory, GenericImage, ImageBuffer, Rgba, RgbaImage};

/// A shared image
#[derive(Clone)]
pub struct Texture(ImageBuffer<Rgba<u8>, Arc<[u8]>>, Arc<str>);

pub(crate) struct TexturePtr(Arc<[u8]>);

impl Texture {
    pub(crate) fn from_bytes(data: &[u8], label: &str) -> Result<Self> {
        let rgba_image = load_from_memory(data)?.to_rgba8();
        Ok(Texture(
            ImageBuffer::from_raw(
                rgba_image.width(),
                rgba_image.height(),
                rgba_image.into_raw().into(),
            )
            .unwrap(),
            Arc::from(label.to_string()),
        ))
    }

    pub(crate) fn subimage(&self, x: u32, y: u32, w: u32, h: u32) -> Result<Self> {
        let mut new_image: RgbaImage = RgbaImage::new(w, h);
        new_image.copy_from(&self.0, x, y)?;
        Ok(Texture(
            ImageBuffer::<Rgba<u8>, Arc<[u8]>>::from_raw(w, h, new_image.into_raw().into())
                .unwrap(),
            self.1.clone(),
        ))
    }

    pub(crate) fn label(&self) -> &str {
        self.1.as_ref()
    }

    pub(crate) fn width(&self) -> u32 {
        self.0.width()
    }

    pub(crate) fn height(&self) -> u32 {
        self.0.height()
    }
}

impl Deref for Texture {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl From<&Texture> for TexturePtr {
    fn from(image: &Texture) -> Self {
        Self(image.0.as_raw().clone())
    }
}

impl std::hash::Hash for TexturePtr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = self.0.as_ptr() as usize;
        state.write_usize(ptr);
    }
}

impl std::cmp::PartialEq for TexturePtr {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl std::cmp::Eq for TexturePtr {}
