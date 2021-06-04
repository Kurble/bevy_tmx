use std::io::{BufReader, Read};
use std::path::Path;
use std::pin::Pin;

use anyhow::*;
use bevy::math::{IVec2, UVec2, Vec4};
use xml::attribute::OwnedAttribute;
use xml::reader::{EventReader, XmlEvent};

use crate::tmx::map::Map;
use crate::TmxLoadContext;

use super::*;

enum Data {
    U8(Vec<u8>),
    U32(Vec<u32>),
}

impl Data {
    fn into_vec_u8(self) -> Vec<u8> {
        match self {
            Data::U8(v) => v,
            Data::U32(_) => unimplemented!("u8 to u32 conversion is not needed"),
        }
    }

    fn into_vec_u32(self) -> Vec<u32> {
        match self {
            Data::U8(v) => v
                .chunks_exact(4)
                .map(|chunk| {
                    (chunk[0] as u32)
                        | (chunk[1] as u32) << 8
                        | (chunk[2] as u32) << 16
                        | (chunk[3] as u32) << 24
                })
                .collect(),
            Data::U32(v) => v,
        }
    }
}

impl Map {
    pub(crate) async fn load_from_xml_reader<R: Read + Send>(
        env: TmxLoadContext<'_>,
        mut reader: EventReader<R>,
    ) -> Result<Self> {
        loop {
            if let XmlEvent::StartElement {
                name, attributes, ..
            } = reader.next()?
            {
                if name.local_name == "map" {
                    return Map::parse(env, attributes, &mut reader).await;
                } else {
                    parse_empty(&mut reader)?;
                }
            }
        }
    }

    async fn parse<R: Read + Send>(
        env: TmxLoadContext<'_>,
        attributes: Vec<OwnedAttribute>,
        reader: &mut EventReader<R>,
    ) -> Result<Self> {
        let mut result = Map {
            properties: HashMap::new(),
            tilesets: Vec::new(),
            layers: Vec::new(),

            width: 0,
            height: 0,
            tile_type: TileType::Ortho {
                width: 0,
                height: 0,
                render_order: RenderOrder::RightDown,
            },

            background: [0; 4],
        };

        let mut render_order = RenderOrder::RightDown;
        let mut tile_type = 0;
        let mut tile_width = 0;
        let mut tile_height = 0;
        let mut stagger_y = false;
        let mut stagger_i = true;
        let mut hex_side_length = 0;

        for a in attributes {
            match a.name.local_name.as_ref() {
                "width" => result.width = a.value.parse()?,
                "height" => result.height = a.value.parse()?,
                "tilewidth" => tile_width = a.value.parse()?,
                "tileheight" => tile_height = a.value.parse()?,
                "renderorder" => {
                    render_order = match a.value.as_ref() {
                        "right-down" => RenderOrder::RightDown,
                        "right-up" => RenderOrder::RightUp,
                        "left-down" => RenderOrder::LeftDown,
                        "left-up" => RenderOrder::LeftUp,
                        _ => bail!("invalid renderorder"),
                    }
                }
                "orientation" => {
                    tile_type = match a.value.as_ref() {
                        "orthogonal" => 0,
                        "isometric" => 1,
                        "staggered" => 2,
                        "hexagonal" => 3,
                        _ => bail!("invalid orientation"),
                    }
                }
                "backgroundcolor" => {
                    result.background = [1; 4];
                }
                "staggeraxis" => {
                    stagger_y = match a.value.as_ref() {
                        "x" => false,
                        "y" => true,
                        _ => bail!("invalid staggeraxis"),
                    }
                }
                "staggerindex" => {
                    stagger_i = match a.value.as_ref() {
                        "odd" => true,
                        "even" => false,
                        _ => bail!("invalid staggerindex"),
                    }
                }
                "hexsidelength" => hex_side_length = a.value.parse()?,
                _ => (), // skip
            }
        }

        result.tile_type = match tile_type {
            0 => TileType::Ortho {
                width: tile_width,
                height: tile_height,
                render_order,
            },
            1 => TileType::Isometric {
                width: tile_width,
                height: tile_height,
                stagger: false,
                stagger_odd: stagger_i,
                stagger_y,
                render_order,
            },
            2 => TileType::Isometric {
                width: tile_width,
                height: tile_height,
                stagger: true,
                stagger_odd: stagger_i,
                stagger_y,
                render_order,
            },
            3 => TileType::Hexagonal {
                width: tile_width,
                height: tile_width,
                stagger_odd: stagger_i,
                stagger_y,
                side_length: hex_side_length,
                render_order,
            },
            _ => unreachable!(),
        };

        while match reader.next()? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                match name.local_name.as_ref() {
                    "properties" => {
                        result.properties = parse_properties(reader)?;
                    }
                    "tileset" => {
                        result.tilesets.push(Arc::new(
                            Tileset::parse(env.clone(), attributes, reader).await?,
                        ));
                    }
                    "layer" => {
                        result.layers.push(Layer::parse_tiles(attributes, reader)?);
                    }
                    "objectgroup" => {
                        result = Layer::parse_objects(env.clone(), attributes, reader)
                            .await?
                            .process(result)
                            .await?;
                    }
                    "imagelayer" => {
                        result
                            .layers
                            .push(Layer::parse_image(env.clone(), attributes, reader).await?);
                    }
                    "group" => {
                        result
                            .layers
                            .push(Layer::parse_group(env.clone(), attributes, reader).await?);
                    }
                    _ => parse_empty(reader)?, // skip
                }

                true
            }
            XmlEvent::EndElement { .. } => false,
            _ => true,
        } {
            continue;
        }

        Ok(result)
    }
}

impl Tileset {
    /// Parse a tileset element. This can be either an external reference or an actual tileset.
    async fn parse<R: Read + Send>(
        env: TmxLoadContext<'_>,
        attributes: Vec<OwnedAttribute>,
        reader: &mut EventReader<R>,
    ) -> Result<Self> {
        let mut result = Tileset {
            first_gid: 0,
            source: "embedded#".to_string(),
            tiles: Vec::new(),
            image: None,
            tile_size: Vec2::ZERO,
        };

        let mut found_source = false;

        for a in attributes.iter() {
            match a.name.local_name.as_ref() {
                "firstgid" => {
                    result.first_gid = a.value.parse()?;
                }
                "name" => {
                    result.source = format!("embedded#{}", a.value.clone());
                }
                "source" => {
                    found_source = true;
                    let source_path = Path::new(a.value.as_str());
                    let file_name = env.unique_name(source_path);
                    let sub_env = env.file_directory(source_path);
                    let file = env.load_file(source_path).await?;
                    let file = BufReader::new(file.as_slice());
                    let mut reader = EventReader::new(file);
                    loop {
                        if let XmlEvent::StartElement {
                            name, attributes, ..
                        } = reader.next()?
                        {
                            if name.local_name == "tileset" {
                                result =
                                    Tileset::parse_tsx(result, sub_env, attributes, &mut reader)
                                        .await?;
                                result.source = file_name;
                                break;
                            } else {
                                parse_empty(&mut reader)?;
                            }
                        }
                    }
                }
                _ => (),
            }
        }

        if found_source {
            // The actual XML element will be parsed in Tileset::parse_tmx(..).
            // If we parse the TMX from an external file, this means the element is not handled. To correct for
            //  this we call parse_empty(..) if an external file was found.
            parse_empty(reader)?;
            Ok(result)
        } else {
            Tileset::parse_tsx(result, env, attributes, reader).await
        }
    }

    /// Parse the actual tileset content
    async fn parse_tsx<R: Read + Send>(
        mut tileset: Tileset,
        env: TmxLoadContext<'_>,
        attributes: Vec<OwnedAttribute>,
        reader: &mut EventReader<R>,
    ) -> Result<Tileset> {
        let mut tile_width = 0;
        let mut tile_height = 0;
        let mut spacing = 0;
        let mut margin = 0;
        let mut tile_count: Option<u32> = None;
        let mut columns: Option<i32> = None;

        for a in attributes.iter() {
            match a.name.local_name.as_ref() {
                "tilewidth" => tile_width = a.value.parse()?,
                "tileheight" => tile_height = a.value.parse()?,
                "spacing" => spacing = a.value.parse()?,
                "margin" => margin = a.value.parse()?,
                "tilecount" => tile_count = Some(a.value.parse()?),
                "columns" => columns = Some(a.value.parse()?),
                _ => (),
            }
        }

        tileset.tile_size.x = tile_width as f32;
        tileset.tile_size.y = tile_height as f32;

        while match reader.next()? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                match name.local_name.as_ref() {
                    "image" => {
                        let columns = columns;
                        let spacing = spacing;
                        let margin = margin;
                        let tile_width = tile_width;
                        let mut tiles_added = 0;
                        let image = parse_image(env.clone(), attributes, reader).await?;
                        tileset.image = Some(image.clone());

                        let (width, height) = (image.width(), image.height());
                        let (width, height) = (width as i32, height as i32);
                        let columns = columns.unwrap_or_else(|| {
                            let mut space = width - margin * 2;
                            let mut cols = 0;
                            while space >= tile_width {
                                space -= tile_width + spacing;
                                space -= spacing;
                                cols += 1;
                            }
                            cols
                        });
                        let rows = {
                            let mut space = height - margin * 2;
                            let mut rows = 0;
                            while space >= tile_height {
                                space -= tile_height + spacing;
                                rows += 1;
                            }
                            rows
                        };

                        for y in 0..rows {
                            for x in 0..columns {
                                if tile_count.map_or(true, |tc| tiles_added < tc) {
                                    let u = (margin + x * tile_width + x * spacing) as f32
                                        / width as f32;
                                    let v = (margin + y * tile_height + y * spacing) as f32
                                        / height as f32;
                                    let w = tile_width as f32 / width as f32;
                                    let h = tile_height as f32 / height as f32;

                                    tileset.tiles.push(Some(Tile {
                                        image: Some(image.clone()),
                                        top_left: Vec2::new(u, v),
                                        bottom_right: Vec2::new(u + w, v + h),
                                        width: tile_width,
                                        height: tile_height,
                                        animation: Vec::new(),
                                        properties: HashMap::new(),
                                        shapes: Vec::new(),
                                    }));

                                    tiles_added += 1;
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                    "tile" => {
                        let (id, tile) = Tile::parse(env.clone(), attributes, reader).await?;

                        if id < tileset.tiles.len() {
                            if tileset.tiles[id].is_none() {
                                tileset.tiles[id].replace(tile);
                            } else {
                                // we already checked if the tile exists, unwrap is safe.
                                tileset.tiles[id].as_mut().unwrap().join(tile);
                            }
                        } else {
                            while id > tileset.tiles.len() {
                                tileset.tiles.push(None);
                            }
                            tileset.tiles.push(Some(tile));
                        }
                    }
                    _ => parse_empty(reader)?, // skip
                }

                true
            }
            XmlEvent::EndElement { .. } => false,
            _ => true,
        } {
            continue;
        }

        Ok(tileset)
    }
}

impl Tile {
    fn join(&mut self, mut new_data: Tile) {
        self.properties = new_data.properties;
        self.animation = new_data.animation;
        if new_data.image.is_some() {
            self.top_left = new_data.top_left;
            self.bottom_right = new_data.bottom_right;
            self.image = new_data.image;
        }
        self.shapes.append(&mut new_data.shapes);
    }

    async fn parse<R: Read + Send>(
        env: TmxLoadContext<'_>,
        attributes: Vec<OwnedAttribute>,
        reader: &mut EventReader<R>,
    ) -> Result<(usize, Tile)> {
        let mut id = 0;

        for a in attributes.iter() {
            if a.name.local_name == "id" {
                id = a.value.parse()?
            }
        }

        let mut result = Tile {
            image: None,
            top_left: Vec2::new(0.0, 0.0),
            bottom_right: Vec2::new(1.0, 1.0),
            width: 0,
            height: 0,
            animation: Vec::new(),
            properties: HashMap::new(),
            shapes: Vec::new(),
        };

        while match reader.next()? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                match name.local_name.as_ref() {
                    "properties" => {
                        result.properties = parse_properties(reader)?;
                    }
                    "image" => {
                        let image = parse_image(env.clone(), attributes, reader).await?;
                        result.width = image.width() as i32;
                        result.height = image.height() as i32;
                        result.image = Some(image);
                    }
                    "animation" => {
                        result.animation = parse_animation(reader)?;
                    }
                    "objectgroup" => {
                        let group = Layer::parse_objects(env.clone(), attributes, reader).await?;
                        if let Layer::ObjectLayer { objects, .. } = group {
                            for object in objects {
                                if let Some(mut shape) = object.shape {
                                    for point in shape.points.iter_mut() {
                                        point.x += object.x;
                                        point.y += object.y;
                                    }
                                    result.shapes.push(shape);
                                }
                            }
                        }
                    }
                    _ => parse_empty(reader)?, // skip
                }

                true
            }
            XmlEvent::EndElement { .. } => false,
            _ => true,
        } {
            continue;
        }

        Ok((id, result))
    }
}

impl Layer {
    fn parse_tiles<R: Read + Send>(
        attributes: Vec<OwnedAttribute>,
        reader: &mut EventReader<R>,
    ) -> Result<Self> {
        let mut position = IVec2::ZERO;
        let mut size = UVec2::ZERO;
        let mut color = Vec4::new(1.0, 1.0, 1.0, 1.0);
        let mut visible = true;
        let mut offset = IVec2::ZERO;
        let mut parallax = Vec2::new(1.0, 1.0);
        let mut data = Vec::new();

        for a in attributes {
            match a.name.local_name.as_ref() {
                "x" => position.x = a.value.parse()?,
                "y" => position.y = a.value.parse()?,
                "width" => size.x = a.value.parse()?,
                "height" => size.y = a.value.parse()?,
                "offsetx" => offset.x = a.value.parse()?,
                "offsety" => offset.y = a.value.parse()?,
                "parallaxx" => parallax.x = a.value.parse()?,
                "parallaxy" => parallax.y = a.value.parse()?,
                "opacity" => color.w *= a.value.parse::<f32>()?,
                "tintcolor" => color *= parse_color_vec4(a.value.as_str())?,
                "visible" => visible = a.value == "true",
                _ => (), // skip
            }
        }

        while match reader.next()? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                match name.local_name.as_ref() {
                    "data" => data = parse_data(attributes, reader)?.into_vec_u32(),
                    _ => parse_empty(reader)?, // skip
                }

                true
            }
            XmlEvent::EndElement { .. } => false,
            _ => true,
        } {}

        Ok(Layer::TileLayer {
            position,
            size,
            color,
            visible,
            offset,
            parallax,
            data,
        })
    }

    async fn parse_objects<R: Read + Send>(
        env: TmxLoadContext<'_>,
        attributes: Vec<OwnedAttribute>,
        reader: &mut EventReader<R>,
    ) -> Result<Self> {
        let mut offset = IVec2::ZERO;
        let mut parallax = Vec2::new(1.0, 1.0);
        let mut color = Vec4::new(1.0, 1.0, 1.0, 1.0);
        let mut visible = true;
        let mut draworder_index = false;
        let mut objects = Vec::new();

        for a in attributes {
            match a.name.local_name.as_ref() {
                "offsetx" => offset.x = a.value.parse()?,
                "offsety" => offset.y = a.value.parse()?,
                "parallaxx" => parallax.x = a.value.parse()?,
                "parallaxy" => parallax.y = a.value.parse()?,
                "opacity" => color.w *= a.value.parse::<f32>()?,
                "tintcolor" => color *= parse_color_vec4(a.value.as_str())?,
                "visible" => visible = a.value == "true",
                "draworder" => draworder_index = a.value == "index",
                _ => (), // skip
            }
        }

        while match reader.next()? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                match name.local_name.as_ref() {
                    "object" => {
                        objects.push(Object::parse(env.clone(), attributes, reader).await?);
                    }
                    _ => parse_empty(reader)?, // skip
                }

                true
            }
            XmlEvent::EndElement { .. } => false,
            _ => true,
        } {
            continue;
        }

        Ok(Layer::ObjectLayer {
            offset,
            parallax,
            color,
            visible,
            draworder_index,
            objects,
        })
    }

    async fn process(mut self, mut map: Map) -> Result<Map> {
        //let mut new_tilesets = Vec::new();
        //let mut next_first_gid = map.tilesets
        //	.last()
        //	.map(|ts| ts.first_gid + ts.tiles.len() as u32)
        //	.unwrap_or(1);

        match &mut self {
            Layer::ObjectLayer { objects, .. } => {
                for object in objects.iter_mut() {
                    if let Some(&Property::File(ref tileset_source)) =
                        object.properties.get("__include_tileset__")
                    {
                        let mut found = false;
                        for tileset in map.tilesets.iter() {
                            if tileset.source == tileset_source.as_ref() {
                                object.tile = object.tile.map(|t| tileset.first_gid + t);
                                found = true;
                            }
                        }

                        if !found {
                            // tileset needs to be added to the map
                            //object.tile = object.tile.map(|t| tileset.)

                            println!("Can't find the tileset back in the map!!");
                            println!(
                                "Tilesets in map: {:#?}",
                                map.tilesets
                                    .iter()
                                    .map(|ts| ts.source.as_str())
                                    .collect::<Vec<_>>()
                            );
                            println!("Tileset in template: {}", tileset_source);

                            todo!("Tilesets referenced in templates must also exist in the map for now.");

                            //
                        }
                    }
                }
            }
            &mut _ => unreachable!(),
        }

        map.layers.push(self);

        Ok(map)
    }

    async fn parse_image<R: Read + Send>(
        env: TmxLoadContext<'_>,
        attributes: Vec<OwnedAttribute>,
        reader: &mut EventReader<R>,
    ) -> Result<Self> {
        let mut image = Err(anyhow!("no image found"));

        let mut offset = IVec2::ZERO;
        let mut parallax = Vec2::new(1.0, 1.0);
        let mut color = Vec4::new(1.0, 1.0, 1.0, 1.0);
        let mut visible: bool = true;

        for a in attributes {
            match a.name.local_name.as_ref() {
                "offsetx" => offset.x = a.value.parse()?,
                "offsety" => offset.y = a.value.parse()?,
                "parallaxx" => parallax.x = a.value.parse()?,
                "parallaxy" => parallax.y = a.value.parse()?,
                "opacity" => color.w *= a.value.parse::<f32>()?,
                "tintcolor" => color *= parse_color_vec4(a.value.as_str())?,
                "visible" => visible = a.value == "true",
                _ => (), // skip
            }
        }

        while match reader.next()? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                match name.local_name.as_ref() {
                    "image" => {
                        image = parse_image(env.clone(), attributes, reader).await;
                    }
                    _ => parse_empty(reader)?, // skip
                }

                true
            }
            XmlEvent::EndElement { .. } => false,
            _ => true,
        } {
            continue;
        }

        image.map(|image| Layer::ImageLayer {
            image,
            color,
            visible,
            offset,
            parallax,
        })
    }

    fn parse_group<'a, R: Read + Send>(
        env: TmxLoadContext<'a>,
        attributes: Vec<OwnedAttribute>,
        reader: &'a mut EventReader<R>,
    ) -> Pin<Box<dyn Future<Output = Result<Self>> + Send + 'a>> {
        Box::pin(async move {
            let mut offset = IVec2::ZERO;
            let mut parallax = Vec2::new(1.0, 1.0);
            let mut color = Vec4::new(1.0, 1.0, 1.0, 1.0);
            //let mut visible: Option<bool> = None;

            for a in attributes {
                match a.name.local_name.as_ref() {
                    "offsetx" => offset.x = a.value.parse()?,
                    "offsety" => offset.y = a.value.parse()?,
                    "parallaxx" => parallax.x = a.value.parse()?,
                    "parallaxy" => parallax.y = a.value.parse()?,
                    "opacity" => color.w *= a.value.parse::<f32>()?,
                    "tintcolor" => color *= parse_color_vec4(a.value.as_str())?,
                    //"visible" => visible = Some(a.value == "true"),
                    _ => (), // skip
                }
            }

            let mut layers = Vec::new();

            while match reader.next()? {
                XmlEvent::StartElement {
                    name, attributes, ..
                } => {
                    match name.local_name.as_ref() {
                        "layer" => {
                            layers.push(Layer::parse_tiles(attributes, reader)?);
                        }
                        "objectgroup" => {
                            layers
                                .push(Layer::parse_objects(env.clone(), attributes, reader).await?);
                        }
                        "imagelayer" => {
                            layers.push(Layer::parse_image(env.clone(), attributes, reader).await?);
                        }
                        "group" => {
                            layers.push(Layer::parse_group(env.clone(), attributes, reader).await?);
                        }
                        _ => parse_empty(reader)?, // skip
                    }

                    true
                }
                XmlEvent::EndElement { .. } => false,
                _ => true,
            } {
                continue;
            }

            for l in layers.iter_mut() {
                l.add_offset(offset.x, offset.y);
                l.mul_parallax(parallax.x, parallax.y);
                l.mul_color(color);
            }
            Ok(Layer::Group { layers })
        })
    }
}

impl Object {
    fn parse<'a, R: Read + Send>(
        env: TmxLoadContext<'a>,
        attributes: Vec<OwnedAttribute>,
        reader: &'a mut EventReader<R>,
    ) -> Pin<Box<dyn Future<Output = Result<Object>> + Send + 'a>> {
        Box::pin(async move {
            let mut result = Object {
                id: 0,
                properties: HashMap::new(),
                tile: None,
                shape: None,
                name: String::from(""),
                ty: String::from(""),
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
                rotation: 0.0,
                visible: true,
            };

            // see if there is a template
            for a in attributes.iter() {
                if a.name.local_name == "template" {
                    let sub_env = env.file_directory(Path::new(a.value.as_str()));

                    let file = env
                        .load_file(Path::new(a.value.as_str()).to_path_buf())
                        .await?;
                    let file = BufReader::new(file.as_slice());
                    let mut reader = EventReader::new(file);

                    loop {
                        if let XmlEvent::StartElement { name, .. } = reader.next()? {
                            if name.local_name == "template" {
                                result =
                                    Object::parse_template(sub_env.clone(), &mut reader).await?;
                                break;
                            } else {
                                parse_empty(&mut reader)?;
                            }
                        }
                    }
                }
            }

            // apply properties
            for a in attributes.iter() {
                match a.name.local_name.as_ref() {
                    "id" => result.id = a.value.parse()?,
                    "gid" => result.tile = Some(a.value.parse()?),
                    "name" => result.name = a.value.clone(),
                    "type" => result.ty = a.value.clone(),
                    "x" => result.x = a.value.parse()?,
                    "y" => result.y = a.value.parse()?,
                    "width" => result.width = a.value.parse()?,
                    "height" => result.height = a.value.parse()?,
                    "rotation" => result.rotation = a.value.parse()?,
                    "visible" => result.visible = a.value == "true",
                    _ => (),
                }
            }

            while match reader.next()? {
                XmlEvent::StartElement {
                    name, attributes, ..
                } => {
                    match name.local_name.as_ref() {
                        "properties" => {
                            for (k, v) in parse_properties(reader)?.into_iter() {
                                result.properties.insert(k, v);
                            }
                        }
                        "polyline" | "polygon" => {
                            let points: Result<Vec<Vec2>> = attributes
                                .iter()
                                .filter(|a| a.name.local_name == "points")
                                .flat_map(|a| a.value.split(' '))
                                .map(|pt| {
                                    let mut i = pt.split(',').map(|x| x.parse::<f32>());
                                    let x = i.next();
                                    let y = i.next();
                                    match (x, y) {
                                        (Some(Ok(x)), Some(Ok(y))) => Ok(Vec2::new(x, y)),
                                        _ => Err(anyhow!("invalid point")),
                                    }
                                })
                                .fold(Ok(Vec::new()), |vec, result| match vec {
                                    Ok(mut vec) => {
                                        vec.push(result?);
                                        Ok(vec)
                                    }
                                    Err(e) => Err(e),
                                });

                            result.shape = Some(Shape {
                                points: points?,
                                closed: name.local_name == "polygon",
                            });
                            parse_empty(reader)?;
                        }
                        _ => parse_empty(reader)?, // skip
                    }

                    true
                }
                XmlEvent::EndElement { .. } => false,
                _ => true,
            } {}

            Ok(result)
        })
    }

    async fn parse_template<R: Read + Send>(
        env: TmxLoadContext<'_>,
        reader: &mut EventReader<R>,
    ) -> Result<Object> {
        let mut tileset = Err(anyhow!("tileset not found"));
        let mut object = Err(anyhow!("object not found"));

        while match reader.next()? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                match name.local_name.as_ref() {
                    "tileset" => {
                        let mut first_gid = 0;
                        let mut source = "".to_string();
                        for a in attributes.iter() {
                            match a.name.local_name.as_ref() {
                                "firstgid" => first_gid = a.value.parse()?,
                                "source" => {
                                    source = env.unique_name(Path::new(a.value.as_str()));
                                }
                                _ => (),
                            }
                        }

                        tileset = Ok((first_gid, source));
                        parse_empty(reader)?;
                    }
                    "object" => {
                        object = Object::parse(env.clone(), attributes, reader).await;
                    }
                    _ => parse_empty(reader)?, // skip
                }

                true
            }
            XmlEvent::EndElement { .. } => false,
            _ => true,
        } {
            continue;
        }

        let mut object = object?;
        if object.tile.is_some() {
            let (first_gid, source) = tileset?;
            object.tile = object.tile.map(|t| t - first_gid);
            object
                .properties
                .insert("__include_tileset__".to_string(), Property::File(source));
        }
        Ok(object)
    }
}

async fn parse_image<R: Read + Send>(
    env: TmxLoadContext<'_>,
    attributes: Vec<OwnedAttribute>,
    reader: &mut EventReader<R>,
) -> Result<texture::Texture> {
    let mut source: Option<String> = None;
    //let mut trans: Option<[u8; 4]> = None;
    let mut width: Option<u32> = None;
    let mut height: Option<u32> = None;
    let mut data: Option<Vec<u8>> = None;
    //let mut format: Option<String> = None;

    for a in attributes.iter() {
        match a.name.local_name.as_ref() {
            "source" => source = Some(a.value.clone()),
            //"trans" => trans = Some(parse_color(a.value.as_str())),
            "width" => width = Some(a.value.parse()?),
            "height" => height = Some(a.value.parse()?),
            //"format" => format = Some(a.value.clone()),
            _ => (),
        }
    }

    while match reader.next()? {
        XmlEvent::StartElement {
            name, attributes, ..
        } => {
            match name.local_name.as_ref() {
                "data" => data = Some(parse_data(attributes, reader)?.into_vec_u8()),
                _ => parse_empty(reader)?, // skip
            }

            true
        }
        XmlEvent::EndElement { .. } => false,
        _ => true,
    } {
        continue;
    }

    let label = source.as_deref().unwrap_or("embedded#");
    let mut image = if let Some(source) = source.as_ref() {
        Texture::from_bytes(&env.load_file(Path::new(source)).await?, label)?
    } else if let Some(data) = data {
        Texture::from_bytes(data.as_slice(), label)?
    } else {
        bail!("invalid image")
    };

    if let (Some(width), Some(height)) = (width, height) {
        image = image.subimage(0, 0, width, height)?;
    }
    Ok(image)
}

fn parse_data<R: Read + Send>(
    attributes: Vec<OwnedAttribute>,
    reader: &mut EventReader<R>,
) -> Result<Data> {
    let mut decode_csv = false;
    let mut decode_base64 = false;
    let mut decompress_z = false;
    let mut decompress_g = false;

    for a in attributes.iter() {
        match a.name.local_name.as_ref() {
            "encoding" => match a.value.as_ref() {
                "csv" => decode_csv = true,
                "base64" => decode_base64 = true,
                _ => (),
            },
            "compression" => match a.value.as_ref() {
                "zlib" => decompress_z = true,
                "glib" => decompress_g = true,
                _ => (),
            },
            _ => (),
        }
    }

    let mut result = Data::U32(Vec::new());

    while match reader.next()? {
        XmlEvent::StartElement { .. } => {
            parse_empty(reader)?;
            true
        }
        XmlEvent::Characters(s) => {
            if decode_csv {
                result = Data::U32(
                    s.split(',')
                        .filter(|v| v.trim() != "")
                        .map(|v| v.replace('\r', "").parse().unwrap_or(0))
                        .collect(),
                );
            } else if decode_base64 {
                let bytes = base64::decode(s.trim().as_bytes())?;

                let bytes = if decompress_z {
                    let mut zd = libflate::zlib::Decoder::new(BufReader::new(&bytes[..]))?;
                    let mut bytes = Vec::new();
                    zd.read_to_end(&mut bytes)?;

                    bytes
                } else if decompress_g {
                    let mut zd = libflate::gzip::Decoder::new(BufReader::new(&bytes[..]))?;
                    let mut bytes = Vec::new();
                    zd.read_to_end(&mut bytes)?;

                    bytes
                } else {
                    bytes
                };

                result = Data::U8(bytes)
            } else {
                bail!("<tile> based data is not supported");
            }

            true
        }
        XmlEvent::EndElement { .. } => false,
        _ => true,
    } {
        continue;
    }

    Ok(result)
}

fn parse_properties<R: Read + Send>(
    reader: &mut EventReader<R>,
) -> Result<HashMap<String, Property>> {
    let mut result = HashMap::new();

    while match reader.next()? {
        XmlEvent::StartElement {
            name, attributes, ..
        } => {
            match name.local_name.as_ref() {
                "property" => {
                    let (k, v) = parse_property(attributes, reader)?;
                    result.insert(k, v);
                }
                _ => parse_empty(reader)?, // skip
            }

            true
        }
        XmlEvent::EndElement { .. } => false,
        _ => true,
    } {
        continue;
    }

    Ok(result)
}

fn parse_property<R: Read + Send>(
    attributes: Vec<OwnedAttribute>,
    reader: &mut EventReader<R>,
) -> Result<(String, Property)> {
    let mut key = String::from("");
    let mut value = Property::Int(0);
    let mut ty = 0;

    for a in attributes {
        match a.name.local_name.as_ref() {
            "name" => key = a.value.clone(),
            "type" => {
                ty = match a.value.as_ref() {
                    "string" => 0,
                    "int" => 1,
                    "float" => 2,
                    "bool" => 3,
                    "color" => 4,
                    "file" => 5,
                    _ => bail!("invalid property type"),
                }
            }
            "value" => {
                value = match ty {
                    0 => Property::String(a.value.clone()),
                    1 => Property::Int(a.value.parse()?),
                    2 => Property::Float(a.value.parse()?),
                    3 => Property::Bool(a.value == "true"),
                    4 => Property::Color(parse_color(a.value.as_str())?),
                    5 => Property::File(a.value.clone()),
                    _ => unreachable!(),
                }
            }
            _ => (), // skip
        }
    }

    parse_empty(reader)?;

    Ok((key, value))
}

fn parse_animation<R: Read + Send>(reader: &mut EventReader<R>) -> Result<Vec<Frame>> {
    let mut result = Vec::new();

    while match reader.next()? {
        XmlEvent::StartElement {
            name, attributes, ..
        } => {
            match name.local_name.as_ref() {
                "frame" => result.push(parse_frame(attributes, reader)?),
                _ => parse_empty(reader)?, // skip
            }

            true
        }
        XmlEvent::EndElement { .. } => false,
        _ => true,
    } {
        continue;
    }

    Ok(result)
}

fn parse_frame<R: Read + Send>(
    attributes: Vec<OwnedAttribute>,
    reader: &mut EventReader<R>,
) -> Result<Frame> {
    let mut frame = Frame {
        tile: 0,
        duration: 0,
    };

    for a in attributes {
        match a.name.local_name.as_ref() {
            "tileid" => frame.tile = a.value.parse()?,
            "duration" => frame.duration = a.value.parse()?,
            _ => (), // skip
        }
    }

    parse_empty(reader)?;

    Ok(frame)
}

fn parse_empty<R: Read + Send>(reader: &mut EventReader<R>) -> Result<()> {
    while match reader.next()? {
        XmlEvent::StartElement { .. } => {
            parse_empty(reader)?;
            true
        }
        XmlEvent::EndElement { .. } => false,
        _ => true,
    } {
        continue;
    }
    Ok(())
}

fn parse_color(text: &str) -> Result<[u8; 4]> {
    let lowercase: Vec<char> = text
        .chars()
        .filter(|&c| c != '#')
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let mut result = [255u8; 4];

    let nibble = |c| match c {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        '5' => 5,
        '6' => 6,
        '7' => 7,
        '8' => 8,
        '9' => 9,
        'a' => 10,
        'b' => 11,
        'c' => 12,
        'd' => 13,
        'e' => 14,
        'f' => 15,
        _ => 0,
    };

    match lowercase.len() {
        6 => {
            for (i, e) in lowercase.chunks_exact(2).enumerate() {
                result[i + 1] = nibble(e[0]) << 4 | nibble(e[1]);
            }
            Ok(result)
        }

        8 => {
            for (i, e) in lowercase.chunks_exact(2).enumerate() {
                result[i] = nibble(e[0]) << 4 | nibble(e[1]);
            }
            Ok(result)
        }

        _ => bail!("invalid color"),
    }
}

fn parse_color_vec4(text: &str) -> Result<Vec4> {
    let [a, r, g, b] = parse_color(text)?;
    Ok(Vec4::new(r as f32, g as f32, b as f32, a as f32) * (1.0 / 255.0))
}
