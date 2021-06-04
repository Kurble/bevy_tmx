use super::*;

/// Tiled has three different rendering types: orthographic, isometric and hexagonal. They are represented through this enum.
#[derive(Debug, Clone, Copy)]
pub enum TileType {
    /// Orthographic (square/rectangle) rendering mode
    Ortho {
        /// Width in pixels of a single tile
        width: u32,
        /// Height in pixels of a single tile
        height: u32,
        /// RenderOrder of tiles. Todo.
        render_order: RenderOrder,
    },
    /// Isometric rendering mode
    Isometric {
        /// Width in pixels at the widest point in a single tile
        width: u32,
        /// Height in pixels at the tallest point in a single tile
        height: u32,
        /// Whether to render in a _staggered_ mode
        stagger: bool,
        /// When rendering staggered, whether odd or even columns/rows are shorter.
        stagger_odd: bool,
        /// When rendering staggered, whether to stagger the x or y axis.
        stagger_y: bool,
        /// RenderOrder of tiles. Todo.
        render_order: RenderOrder,
    },
    /// Hexagonal rendering mode
    Hexagonal {
        /// Width in pixels at the widest point in a single tile
        width: u32,
        /// Height in pixels at the tallest point in a single tile
        height: u32,
        /// Whether odd or even columns/rows are shorter.
        stagger_odd: bool,
        /// Whether to stagger the x or y axis.
        stagger_y: bool,
        /// Width or height in pixels at the flat side of a hex tile, depending on `stagger_y`.  
        side_length: u32,
        /// RenderOrder of tiles. Todo.
        render_order: RenderOrder,
    },
}

impl TileType {
    /// Convert tile coordinates to it's top left coordinates in pixels.
    pub fn coord_to_pos(&self, layer_height: i32, x: i32, y: i32) -> (i32, i32) {
        match *self {
            TileType::Ortho { width, height, .. } => (x * width as i32, y * height as i32),

            TileType::Isometric {
                width,
                height,
                stagger,
                stagger_odd,
                stagger_y,
                ..
            } => {
                if stagger {
                    if stagger_y {
                        let rx = if (y % 2 == 1) == stagger_odd {
                            x * width as i32 + width as i32 / 2
                        } else {
                            x * width as i32
                        };
                        let ry = (height as i32 * y) / 2;
                        (rx, ry)
                    } else {
                        let rx = (width as i32 * x) / 2;
                        let ry = if (x % 2 == 1) == stagger_odd {
                            y * height as i32 + height as i32 / 2
                        } else {
                            y * height as i32
                        };
                        (rx, ry)
                    }
                } else {
                    let rx = (width as i32 * x + width as i32 * (layer_height - 1 - y)) / 2;
                    let ry = (height as i32 * x + height as i32 * y) / 2;
                    (rx, ry)
                }
            }

            TileType::Hexagonal {
                width,
                height,
                stagger_odd,
                stagger_y,
                side_length,
                ..
            } => {
                if stagger_y {
                    let rx = if (y % 2 == 1) == stagger_odd {
                        x * width as i32 + width as i32 / 2
                    } else {
                        x * width as i32
                    };
                    let ry = ((height + side_length) / 2 - 1) as i32 * y;
                    (rx, ry)
                } else {
                    let rx = ((width + side_length) / 2 - 1) as i32 * x;
                    let ry = if (x % 2 == 1) == stagger_odd {
                        y * height as i32 + height as i32 / 2
                    } else {
                        y * height as i32
                    };
                    (rx, ry)
                }
            }
        }
    }

    /// Convert coordinates in pixels to tile coordinates.
    pub fn pos_to_coord(&self, layer_height: i32, x: i32, y: i32) -> (i32, i32) {
        match *self {
            TileType::Ortho { width, height, .. } => (x / width as i32, y / height as i32),

            TileType::Isometric {
                width,
                height,
                stagger,
                stagger_odd,
                stagger_y,
                ..
            } => {
                if stagger {
                    let half_w = width as i32 / 2;
                    let half_h = height as i32 / 2;

                    let (x, y, off_x, off_y) = match (stagger_odd, stagger_y) {
                        (true, _) => (x, y, 0, 0),
                        (false, false) => (x - half_w, y, 1, 0),
                        (false, true) => (x, y - half_h, 0, 1),
                    };

                    let ref_x = div2(x, width as i32);
                    let ref_y = div2(y, height as i32);
                    let rel_x = x - ref_x * width as i32;
                    let rel_y = y - ref_y * height as i32;

                    let offset = if rel_y < half_h {
                        (half_h - rel_y % half_h) * half_w / half_h
                    } else {
                        (rel_y % half_h) * half_w / half_h
                    };

                    let top = rel_y < half_h;
                    let left = rel_x < offset;
                    let right = rel_x > width as i32 - offset;

                    let (x, y) = match (stagger_y, top, left, right) {
                        (true, true, true, false) => (ref_x - 1, ref_y * 2 - 1),
                        (true, true, false, true) => (ref_x, ref_y * 2 - 1),
                        (true, false, true, false) => (ref_x - 1, ref_y * 2 + 1),
                        (true, false, false, true) => (ref_x, ref_y * 2 + 1),
                        (true, _, _, _) => (ref_x, ref_y * 2),

                        (false, true, true, false) => (ref_x * 2 - 1, ref_y - 1),
                        (false, true, false, true) => (ref_x * 2 + 1, ref_y - 1),
                        (false, false, true, false) => (ref_x * 2 - 1, ref_y),
                        (false, false, false, true) => (ref_x * 2 + 1, ref_y),
                        (false, _, _, _) => (ref_x * 2, ref_y),
                    };

                    (x + off_x, y + off_y)
                } else {
                    let origin = (width as i32 * layer_height) / 2;
                    let x = x - origin;
                    let rx = y / (height as i32) + x / (width as i32);
                    let ry = y / (height as i32) - x / (width as i32);
                    (rx, ry)
                }
            }

            TileType::Hexagonal {
                width,
                height,
                stagger_odd,
                stagger_y,
                side_length,
                ..
            } => {
                if stagger_y {
                    let col_w = width as i32;
                    let row_h = (height as i32 - side_length as i32) / 2 + side_length as i32 - 1;
                    let half_w = width as i32 / 2;
                    let half_h = height as i32 / 2;

                    let ref_x = div2(x, col_w);
                    let ref_y = div2(y, row_h);
                    let rel_x = x - ref_x * col_w;
                    let rel_y = y - ref_y * row_h;

                    let centers = if (mod2(ref_y, 2) == 1) == stagger_odd {
                        [
                            (half_w, -row_h + half_h, ref_x, ref_y - 1),
                            (0, half_h, ref_x - 1, ref_y),
                            (col_w, half_h, ref_x, ref_y),
                        ]
                    } else {
                        [
                            (half_w, half_h, ref_x, ref_y),
                            (0, -row_h + half_h, ref_x - 1, ref_y - 1),
                            (col_w, -row_h + half_h, ref_x, ref_y - 1),
                        ]
                    };

                    // find nearest center
                    centers
                        .iter()
                        .min_by_key(|&(x, y, _, _)| {
                            (x - rel_x) * (x - rel_x) + (y - rel_y) * (y - rel_y)
                        })
                        .map(|&(_, _, x, y)| (x, y))
                        .unwrap()
                } else {
                    let col_w = (width as i32 - side_length as i32) / 2 + side_length as i32 - 1;
                    let row_h = height as i32;
                    let half_w = width as i32 / 2;
                    let half_h = height as i32 / 2;

                    let ref_x = div2(x, col_w);
                    let ref_y = div2(y, row_h);
                    let rel_x = x - ref_x * col_w;
                    let rel_y = y - ref_y * row_h;

                    let centers = if (mod2(ref_x, 2) == 1) == stagger_odd {
                        [
                            (-col_w + half_w, half_h, ref_x - 1, ref_y),
                            (half_w, 0, ref_x, ref_y - 1),
                            (half_w, row_h, ref_x, ref_y),
                        ]
                    } else {
                        [
                            (half_w, half_h, ref_x, ref_y),
                            (-col_w + half_w, 0, ref_x - 1, ref_y - 1),
                            (-col_w + half_w, row_h, ref_x - 1, ref_y),
                        ]
                    };

                    // find nearest center
                    centers
                        .iter()
                        .min_by_key(|&(x, y, _, _)| {
                            (x - rel_x) * (x - rel_x) + (y - rel_y) * (y - rel_y)
                        })
                        .map(|&(_, _, x, y)| (x, y))
                        .unwrap()
                }
            }
        }
    }

    /// Get the tile width of this tile type.
    pub fn tile_width(&self) -> u32 {
        match *self {
            TileType::Ortho { width, .. } => width,
            TileType::Isometric { width, .. } => width,
            TileType::Hexagonal { width, .. } => width,
        }
    }

    /// Get the tile height of this tile type.
    pub fn tile_height(&self) -> u32 {
        match *self {
            TileType::Ortho { height, .. } => height,
            TileType::Isometric { height, .. } => height,
            TileType::Hexagonal { height, .. } => height,
        }
    }
}

fn mod2(x: i32, m: i32) -> i32 {
    let y = x % m;
    if y >= 0 {
        y
    } else {
        m + y
    }
}

fn div2(x: i32, d: i32) -> i32 {
    if x >= 0 {
        x / d
    } else {
        x / d - 1
    }
}
