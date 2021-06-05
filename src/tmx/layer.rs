use bevy::math::{IVec2, UVec2, Vec4};

use super::*;

/// A layer
pub enum Layer {
    /// A layer densely populated with tiles.
    TileLayer {
        /// The amount of tiles in the x and y axis.
        size: UVec2,
        /// Position offset of the layer, measured in tiles.
        position: IVec2,
        /// Position offset of the layer, measured in pixels.
        offset: IVec2,
        /// Parallax factor for this layer.
        parallax: Vec2,
        /// Color to multiply the contents of this layer with.
        color: Vec4,
        /// Whether this layer is visible or not.
        /// Contents of invisible layers will have their `Draw` component set to invisible.
        visible: bool,
        /// Tile data (global tile ids) for this layer, row by row.
        data: Vec<u32>,
    },
    /// A layer populated with individual objects.
    ObjectLayer {
        /// Whether to draw objects ordered by index of appearance (true) or y coordinate (false).
        draworder_index: bool,
        /// The objects in the layer.
        objects: Vec<Object>,
        /// Position offset of the layer, measured in tiles.
        offset: IVec2,
        /// Parallax factor for this layer.
        parallax: Vec2,
        /// Color to multiply the contents of this layer with.
        color: Vec4,
        /// Whether this layer is visible or not.
        /// Contents of invisible layers will have their `Draw` component set to invisible.
        visible: bool,
    },
    /// A layer populated with a single big image, like a background.
    ImageLayer {
        /// The image contained in this layer.
        image: Texture,
        /// Position offset of the layer, measured in tiles.
        offset: IVec2,
        /// Parallax factor for this layer.
        parallax: Vec2,
        /// Color to multiply the contents of this layer with.
        color: Vec4,
        /// Whether this layer is visible or not.
        /// Contents of invisible layers will have their `Draw` component set to invisible.
        visible: bool,
    },
    /// A set of layers grouped together, mainly for convenience in the map editor.
    Group {
        /// The layers that were grouped together.
        layers: Vec<Layer>,
    },
}

impl Layer {
    /*pub(crate) fn set_visible(&mut self, new_visible: bool) {
        match self {
            Layer::TileLayer { visible, .. }
            | Layer::ObjectLayer { visible, .. }
            | Layer::ImageLayer { visible, .. } => *visible = new_visible,
            Layer::Group { layers } => {
                for l in layers.iter_mut() {
                    l.set_visible(new_visible);
                }
            }
        }
    }*/

    pub(crate) fn add_offset(&mut self, x: i32, y: i32) {
        match self {
            Layer::TileLayer { offset, .. } => {
                offset.x += x;
                offset.y += y;
            }
            Layer::ObjectLayer { offset, .. } => {
                offset.x += x;
                offset.y += y;
            }
            Layer::ImageLayer { offset, .. } => {
                offset.x += x;
                offset.y += y;
            }
            Layer::Group { layers } => {
                for l in layers.iter_mut() {
                    l.add_offset(x, y);
                }
            }
        }
    }

    pub(crate) fn mul_parallax(&mut self, x: f32, y: f32) {
        match self {
            Layer::TileLayer { parallax, .. } => {
                parallax.x *= x;
                parallax.y *= y;
            }
            Layer::ObjectLayer { parallax, .. } => {
                parallax.x *= x;
                parallax.y *= y;
            }
            Layer::ImageLayer { parallax, .. } => {
                parallax.x *= x;
                parallax.y *= y;
            }
            Layer::Group { layers } => {
                for l in layers.iter_mut() {
                    l.mul_parallax(x, y);
                }
            }
        }
    }

    pub(crate) fn mul_color(&mut self, o: Vec4) {
        match self {
            Layer::TileLayer { color, .. }
            | Layer::ObjectLayer { color, .. }
            | Layer::ImageLayer { color, .. } => {
                *color *= o;
            }
            Layer::Group { layers } => {
                for l in layers.iter_mut() {
                    l.mul_color(o);
                }
            }
        }
    }
}
