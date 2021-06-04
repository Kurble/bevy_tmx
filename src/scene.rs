use std::collections::HashMap;

use anyhow::*;
use bevy::asset::{LoadContext, LoadedAsset};
use bevy::ecs::world::EntityMut;
use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::mesh::Indices;
use bevy::render::pipeline::{PrimitiveTopology, RenderPipeline};
use bevy::render::render_graph::base::MainPass;
use bevy::render::texture::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::{QUAD_HANDLE, SPRITE_PIPELINE_HANDLE};

use crate::parallax::Parallax;
use crate::tmx::{Layer, Map, Object, Texture as TmxTexture, TexturePtr, Tile};

pub type ObjectVisitor = dyn for<'w> Fn(&Object, &mut EntityMut<'w>) + Send + Sync;
pub type ImageVisitor = dyn for<'w> Fn(&mut EntityMut<'w>) + Send + Sync;
pub type MapVisitor = dyn for<'w> Fn(&Map, &mut World) + Send + Sync;

pub struct SceneBuilder<'a, 'b> {
    world: World,
    context: &'a mut LoadContext<'b>,
    map: &'a Map,
    texture_handles: HashMap<TexturePtr, Handle<Texture>>,
    material_handles: HashMap<(Handle<Texture>, [u8; 4]), Handle<ColorMaterial>>,
    object_sprites: HashMap<u32, ProtoSpriteBundle>,
    label_counter: usize,
    offset_z: f32,
    scale: Vec3,
    visit_object: Option<&'a ObjectVisitor>,
    visit_image: Option<&'a ImageVisitor>,
    visit_map: Option<&'a MapVisitor>,
}

#[derive(Debug, Default, Clone, TypeUuid, Reflect)]
#[reflect(Component)]
#[uuid = "39eb4ed0-d44e-4ed5-8676-2e0c148f96c4"]
pub struct ProtoSprite(Vec2);

#[derive(Bundle, Clone)]
struct ProtoSpriteBundle {
    pub sprite: ProtoSprite,
    pub mesh: Handle<Mesh>,
    pub material: Handle<ColorMaterial>,
    pub main_pass: MainPass,
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl<'a, 'b> SceneBuilder<'a, 'b> {
    pub fn new(
        load_context: &'a mut LoadContext<'b>,
        map: &'a Map,
        visit_object: Option<&'a ObjectVisitor>,
        visit_image: Option<&'a ImageVisitor>,
        visit_map: Option<&'a MapVisitor>,
        scale: Vec3,
    ) -> Self {
        Self {
            world: World::default(),
            context: load_context,
            map,
            texture_handles: HashMap::default(),
            material_handles: HashMap::default(),
            object_sprites: HashMap::default(),
            label_counter: 0,
            offset_z: 0.0,
            visit_object,
            visit_image,
            visit_map,
            scale,
        }
    }

    pub fn build(mut self) -> Result<Scene> {
        let map = self.map;
        for (i, layer) in map.layers.iter().enumerate() {
            self.spawn_layer_entities(&layer)
                .with_context(|| format!("layer #{} of tmx asset: \n", i))?;
        }
        if let Some(visit_map) = self.visit_map {
            (*visit_map)(&self.map, &mut self.world);
        }
        Ok(Scene::new(self.world))
    }

    fn spawn_layer_entities(&mut self, layer: &Layer) -> Result<()> {
        match layer {
            Layer::TileLayer {
                position,
                size,
                color,
                visible: _,
                offset,
                parallax,
                data,
            } => {
                let mut images_to_meshes =
                    HashMap::<TexturePtr, (Handle<ColorMaterial>, Vec<_>)>::new();

                for (i, &gid) in data.iter().enumerate() {
                    if let Some(&Tile {
                        image: Some(ref image),
                        top_left,
                        bottom_right,
                        width: tile_width,
                        height: tile_height,
                        ..
                    }) = self.map.get_tile(gid)
                    {
                        let (x, y) = self.map.tile_type.coord_to_pos(
                            size.y as i32,
                            (i as i32 % size.x as i32) + position.x,
                            (i as i32 / size.x as i32) + position.y,
                        );
                        images_to_meshes
                            .entry(TexturePtr::from(image))
                            .or_insert_with(|| {
                                let texture = self.texture_handle(image);
                                let material = self.texture_material_handle(texture, color);
                                (material, Vec::new())
                            })
                            .1
                            .push((x, y, tile_width, tile_height, top_left, bottom_right));
                    }
                }

                for (_, (material, tiles)) in images_to_meshes.into_iter() {
                    let mut vertices = Vec::with_capacity(tiles.len() * 4);
                    let mut normals = Vec::with_capacity(tiles.len() * 4);
                    let mut uvs = Vec::with_capacity(tiles.len() * 4);
                    let mut indices = Vec::with_capacity(tiles.len() * 6);

                    for (x, y, w, h, top_left, bottom_right) in tiles {
                        let i = vertices.len() as u16;
                        indices.extend_from_slice(&[i, i + 1, i + 2, i + 2, i + 1, i + 3]);

                        vertices.push([x as f32, y as f32, 0.0]);
                        vertices.push([(x + w) as f32, y as f32, 0.0]);
                        vertices.push([x as f32, (y + h) as f32, 0.0]);
                        vertices.push([(x + w) as f32, (y + h) as f32, 0.0]);

                        normals.push([0.0, 0.0, 1.0]);
                        normals.push([0.0, 0.0, 1.0]);
                        normals.push([0.0, 0.0, 1.0]);
                        normals.push([0.0, 0.0, 1.0]);

                        uvs.push([top_left.x, top_left.y]);
                        uvs.push([bottom_right.x, top_left.y]);
                        uvs.push([top_left.x, bottom_right.y]);
                        uvs.push([bottom_right.x, bottom_right.y]);
                    }

                    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                    mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
                    mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                    mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                    mesh.set_indices(Some(Indices::U16(indices)));
                    self.label_counter += 1;
                    let mesh = self.context.set_labeled_asset(
                        format!("mesh#{}", self.label_counter).as_str(),
                        LoadedAsset::new(mesh),
                    );

                    let mut entity = self.world.spawn();
                    let transform = Transform::from_xyz(
                        offset.x as f32 * self.scale.x,
                        offset.y as f32 * self.scale.y,
                        self.offset_z,
                    );
                    entity.insert_bundle(ProtoSpriteBundle {
                        sprite: ProtoSprite(self.scale.xy()),
                        mesh,
                        material,
                        transform,
                        ..ProtoSpriteBundle::default()
                    });
                    if parallax != &Vec2::new(1.0, 1.0) {
                        entity.insert(Parallax::new(*parallax, transform));
                    }
                }
            }

            Layer::ObjectLayer {
                objects,
                offset,
                parallax,
                visible,
                color,
                ..
            } => {
                for (i, object) in objects.iter().enumerate() {
                    let object_sprite = object.tile.and_then(|gid| self.object_sprite(gid, color));

                    let mut entity = self.world.spawn();

                    let mut transform = Transform::from_xyz(
                        (offset.x as f32 + object.x) * self.scale.x,
                        (offset.y as f32 + object.y) * self.scale.y,
                        self.offset_z as f32 + (i as f32 / objects.len() as f32) * self.scale.z,
                    );
                    transform.rotation = Quat::from_rotation_z(-object.rotation.to_radians());

                    if let Some(object_sprite) = object_sprite {
                        entity.insert_bundle(ProtoSpriteBundle {
                            sprite: ProtoSprite(
                                Vec2::new(object.width, object.height) * self.scale.xy(),
                            ),
                            transform,
                            visible: Visible {
                                is_transparent: true,
                                is_visible: *visible && object.visible,
                            },
                            ..object_sprite
                        });
                    } else {
                        entity.insert_bundle((transform, GlobalTransform::default()));
                    }

                    if parallax != &Vec2::new(1.0, 1.0) {
                        entity.insert(Parallax::new(*parallax, transform));
                    }

                    if let Some(handler) = self.visit_object.as_ref() {
                        (*handler)(object, &mut entity);
                    }
                }
            }

            Layer::ImageLayer {
                color,
                visible: _,
                offset,
                parallax,
                image,
            } => {
                let texture = self.texture_handle(image);
                let material = self.texture_material_handle(texture, color);
                let transform = Transform::from_xyz(
                    offset.x as f32 * self.scale.x,
                    offset.y as f32 * self.scale.y,
                    self.offset_z,
                );

                let mut entity = self.world.spawn();
                entity.insert_bundle(ProtoSpriteBundle {
                    sprite: ProtoSprite(
                        Vec2::new(image.width() as f32, image.height() as f32) * self.scale.xy(),
                    ),
                    material,
                    transform,
                    ..ProtoSpriteBundle::default()
                });
                if parallax != &Vec2::new(1.0, 1.0) {
                    entity.insert(Parallax::new(*parallax, transform));
                }
                if let Some(handler) = self.visit_image.as_ref() {
                    (*handler)(&mut entity);
                }
            }

            Layer::Group { layers } => {
                for (i, layer) in layers.iter().enumerate() {
                    self.spawn_layer_entities(layer)
                        .with_context(|| format!("in layer #{} of group: \n", i))?;
                }
            }
        }

        self.offset_z += self.scale.z;

        Ok(())
    }

    fn texture_handle(&mut self, image: &TmxTexture) -> Handle<Texture> {
        let width = image.width();
        let height = image.height();

        let texture_handles = &mut self.texture_handles;
        let label_counter = &mut self.label_counter;
        let context = &mut *self.context;

        texture_handles
            .entry(TexturePtr::from(image))
            .or_insert_with(|| {
                let texture = Texture::new(
                    Extent3d::new(width, height, 1),
                    TextureDimension::D2,
                    image.to_vec(),
                    TextureFormat::Rgba8Unorm,
                );
                *label_counter += 1;
                context.set_labeled_asset(
                    format!("{}#{}", image.label(), *label_counter).as_str(),
                    LoadedAsset::new(texture),
                )
            })
            .clone()
    }

    fn texture_material_handle(
        &mut self,
        texture: Handle<Texture>,
        color: &Vec4,
    ) -> Handle<ColorMaterial> {
        let material_handles = &mut self.material_handles;
        let label_counter = &mut self.label_counter;
        let context = &mut *self.context;

        let color_u8 = [
            (color.x * 255.0) as u8,
            (color.y * 255.0) as u8,
            (color.z * 255.0) as u8,
            (color.w * 255.0) as u8,
        ];

        material_handles
            .entry((texture.clone(), color_u8))
            .or_insert_with(|| {
                *label_counter += 1;
                context.set_labeled_asset(
                    format!("material#{}", *label_counter).as_str(),
                    LoadedAsset::new(ColorMaterial::modulated_texture(
                        texture,
                        Color::from(*color),
                    )),
                )
            })
            .clone()
    }

    fn object_sprite(&mut self, gid: u32, color: &Vec4) -> Option<ProtoSpriteBundle> {
        if self.object_sprites.contains_key(&gid) {
            self.object_sprites.get(&gid).cloned()
        } else {
            let tile = self.map.get_tile(gid)?;
            let image = tile.image.as_ref()?;

            let texture = self.texture_handle(image);
            let material = self.texture_material_handle(texture, color);
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
            mesh.set_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![
                    [0.0, -1.0, 0.0],
                    [1.0, -1.0, 0.0],
                    [0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                ],
            );
            mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4]);
            mesh.set_attribute(
                Mesh::ATTRIBUTE_UV_0,
                vec![
                    [tile.top_left.x, tile.top_left.y],
                    [tile.bottom_right.x, tile.top_left.y],
                    [tile.top_left.x, tile.bottom_right.y],
                    [tile.bottom_right.x, tile.bottom_right.y],
                ],
            );
            mesh.set_indices(Some(Indices::U16(vec![0, 1, 2, 2, 1, 3])));
            self.label_counter += 1;
            let mesh = self.context.set_labeled_asset(
                format!("object#{}", self.label_counter).as_str(),
                LoadedAsset::new(mesh),
            );

            Some(
                self.object_sprites
                    .entry(gid)
                    .or_insert(ProtoSpriteBundle {
                        sprite: ProtoSprite(self.scale.xy()),
                        mesh,
                        material,
                        ..ProtoSpriteBundle::default()
                    })
                    .clone(),
            )
        }
    }
}

impl Default for ProtoSpriteBundle {
    fn default() -> Self {
        ProtoSpriteBundle {
            mesh: QUAD_HANDLE.typed(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                SPRITE_PIPELINE_HANDLE.typed(),
            )]),
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            main_pass: MainPass,
            draw: Default::default(),
            sprite: Default::default(),
            material: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

pub fn proto_sprite_upgrade_system(mut commands: Commands, sprites: Query<(Entity, &ProtoSprite)>) {
    for (e, s) in sprites.iter() {
        commands
            .entity(e)
            .insert(Sprite::new(s.0))
            .remove::<ProtoSprite>();
    }
}
