use super::*;

/// A tiled map loaded from a .tmx file.
pub struct Map {
    /// Custom properties.
    pub properties: HashMap<String, Property>,
    /// Tilesets used in the map.
    pub tilesets: Vec<Arc<Tileset>>,
    /// Layers contained in the map.
    pub layers: Vec<Layer>,

    /// The total width of the map, measured in tiles.
    pub width: u32,
    /// The total height of the map, measured in tiles.
    pub height: u32,
    /// The rendering type of the map.
    pub tile_type: TileType,

    /// Background color of the map.
    pub background: [u8; 4],
}

pub struct Objects<'a> {
    l: &'a [Layer],
    i: usize,
    z: f32,

    sub: Option<Box<Objects<'a>>>,
}

impl Map {
    /// Retrieve the tileset associated with the global tile id (gid).
    /// If no tileset is associated with the gid, `None` is returned.
    pub fn get_tileset(&self, gid: u32) -> Option<Arc<Tileset>> {
        for tileset in self.tilesets.iter().rev() {
            if gid >= tileset.first_gid {
                return Some(tileset.clone());
            }
        }
        None
    }

    /// Retrieve the tile metadata associated with the global tile id (gid).
    /// If no tile metadata is associated with the gid, `None` is returned.
    pub fn get_tile(&self, gid: u32) -> Option<&Tile> {
        for tileset in self.tilesets.iter().rev() {
            if gid >= tileset.first_gid {
                let id = gid - tileset.first_gid;
                return if let Some(&Some(ref tile)) = tileset.tiles.get(id as usize) {
                    Some(&tile)
                } else {
                    None
                };
            }
        }
        None
    }

    /// Iterate over all the objects in the map
    pub fn objects(&self) -> Objects {
        Objects {
            l: self.layers.as_slice(),
            i: 0,
            z: 0.0,
            sub: None,
        }
    }
}

impl<'a> Iterator for Objects<'a> {
    type Item = (f32, &'a Object);

    fn next(&mut self) -> Option<(f32, &'a Object)> {
        if let Some(sub) = self.sub.as_mut().and_then(|s| s.next()) {
            return Some(sub);
        } else if self.sub.is_some() {
            self.z = self.sub.take().unwrap().z + 1.0;
            self.sub = None;
        }

        if !self.l.is_empty() {
            match &self.l[0] {
                Layer::Group { layers, .. } => {
                    self.sub = Some(Box::new(Objects {
                        l: layers.as_slice(),
                        i: 0,
                        z: self.z,
                        sub: None,
                    }));
                }

                Layer::ObjectLayer { objects, .. } => {
                    if self.i < objects.len() {
                        self.i += 1;
                        return Some((self.z, &objects[self.i - 1]));
                    }
                }

                _ => {}
            }

            self.l = &self.l[1..];
            self.i = 0;
            self.z += 1.0;
            return self.next();
        }

        None
    }
}
