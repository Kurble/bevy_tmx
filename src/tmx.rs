use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use bevy::math::Vec2;

pub use layer::Layer;
pub use map::Map;
pub use property::Property;
pub use texture::Texture;
pub(crate) use texture::TexturePtr;
pub use tile_type::TileType;

mod layer;
mod map;
mod parse;
mod property;
mod texture;
mod tile_type;

/// Render order for tiles in layers.
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub enum RenderOrder {
    RightDown,
    RightUp,
    LeftDown,
    LeftUp,
}

/// A tileset
pub struct Tileset {
    /// The global tile id of the first tile in this tileset.
    pub first_gid: u32,
    /// The source file of this tileset, or it's name if it's an embedded tileset.
    pub source: String,
    /// The tiles contained in this tileset.
    pub tiles: Vec<Option<Tile>>,
    /// The image that the tiles are taken from, or `None` if all tiles provide their own image.
    pub image: Option<Texture>,
    /// The size in pixels of tiles in this tileset
    pub tile_size: Vec2,
}

/// A single tile description
pub struct Tile {
    /// The image that this tile was taken from
    pub image: Option<Texture>,
    /// The top left UV coordinates of this tile within `image.
    pub top_left: Vec2,
    /// The bottom right UV coordinates of this tile within `image.
    pub bottom_right: Vec2,
    /// The width in pixels of this tile.
    pub width: i32,
    /// The height in pixels of this tile.
    pub height: i32,
    #[allow(missing_docs)]
    pub animation: Vec<Frame>,
    /// Custom properties defined on this tile.
    pub properties: HashMap<String, Property>,
    /// Custom shapes defined on this tile.
    pub shapes: Vec<Shape>,
}

/// Animation frame within a tile
pub struct Frame {
    /// Global tile id of the animation frame.
    pub tile: u32,
    /// Duration in ms
    pub duration: u32,
}

/// Object description
#[derive(Clone, Debug)]
pub struct Object {
    /// Unique id for the object.
    pub id: u32,
    /// Custom properties defined on the object.
    pub properties: HashMap<String, Property>,
    /// Global tile id defining an optional sprite for this object.
    pub tile: Option<u32>,
    /// An optional custom shape for this object.
    pub shape: Option<Shape>,
    /// Custom name for the object
    pub name: String,
    /// Custom type for the object
    pub ty: String,
    /// left X coordinate in pixels where the object is positioned.
    pub x: f32,
    /// bottom Y coordinate in pixels where the object is positioned.
    pub y: f32,
    /// Width in pixels of the object.
    pub width: f32,
    /// Height in pixels of the object.
    pub height: f32,
    /// Rotation around (x,y) in degrees of the object.
    pub rotation: f32,
    /// Whether the object is visible. Invisible objects have their `Draw` component set to invisible.
    pub visible: bool,
}

/// A shape.
#[derive(Clone, Debug)]
pub struct Shape {
    /// Point defining the shape.
    pub points: Vec<Vec2>,
    /// Whether the last point should be connected to the first point.
    pub closed: bool,
}
