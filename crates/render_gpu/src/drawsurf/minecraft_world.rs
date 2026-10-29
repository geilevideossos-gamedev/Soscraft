//! The Minecraft map's terrain: MinecraftOSS section meshes drawn before the
//! exact material passes, with the same camera and scene depth range, so the
//! players, weapons and effects that follow sort against it. The sky, clouds,
//! lightmap and fog are MinecraftOSS's shaders driven by its environment
//! uniform, so the world has Minecraft's day and night.
use std::sync::Arc;

use bevy::core_pipeline::{Core3d, Core3dSystems};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::binding_types::{
    sampler, texture_2d, uniform_buffer_sized,
};
use bevy::render::render_resource::{
    AddressMode, BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindingResource, BlendState,
    Buffer, BufferBinding, BufferDescriptor, BufferInitDescriptor, BufferUsages, ColorTargetState,
    ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, Extent3d, FilterMode, IndexFormat,
    MipmapFilterMode, MultisampleState, Origin3d, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, RawFragmentState, RawRenderPipelineDescriptor,
    RawVertexBufferLayout, RawVertexState, RenderPipeline, Sampler, SamplerBindingType,
    SamplerDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, StoreOp,
    TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect, TextureDescriptor,
    Texture, TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
    TextureViewDescriptor, VertexAttribute, VertexFormat, VertexStepMode,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy::render::view::{ExtractedView, Msaa, ViewTarget};
use bevy::render::{Render, RenderApp, RenderSystems};

use super::depth_range::{GFX_DEPTH_RANGE_SCENE, reverse_z_viewport_depth};
use super::exact_pipeline::ExactPipelineRegistry;
use super::scene_depth::{SCENE_DEPTH_FORMAT, SceneDepthTexture};

/// Bytes of one MinecraftOSS `SectionVertex`: position, atlas uv, colour,
/// sky and block light (times 16), padding.
pub const MINECRAFT_VERTEX_BYTES: u64 = 28;

pub struct MinecraftSectionUpload {
    pub pos: [i32; 3],
    pub vertices: Vec<u8>,
    pub indices: Vec<u32>,
    pub transparent_start: Option<u32>,
}

pub struct MinecraftAtlasImage {
    pub width: u32,
    pub height: u32,
    /// RGBA8 levels, largest first.
    pub levels: Vec<Vec<u8>>,
}

pub struct MinecraftAtlasPatch {
    pub x: u32,
    pub y: u32,
    pub levels: Vec<(u32, u32, Vec<u8>)>,
}

/// Cloud geometry in blocks: position then colour per vertex.
pub struct MinecraftClouds {
    pub vertices: Vec<[f32; 7]>,
    pub indices: Vec<u32>,
}

/// One frame of the Minecraft world as the render world sees it.
#[derive(Resource, Default)]
pub struct MinecraftWorldFrame {
    pub active: bool,
    pub origin: [f64; 3],
    pub view_distance: f32,
    pub atlas: Option<Arc<MinecraftAtlasImage>>,
    pub atlas_patches: Vec<MinecraftAtlasPatch>,
    pub uploads: Vec<MinecraftSectionUpload>,
    pub removed: Vec<[i32; 3]>,
    pub visible: Vec<[i32; 3]>,
    pub generation: u64,
    /// MinecraftOSS's environment uniform, in blocks.
    pub environment: [[f32; 4]; 16],
    /// Sun and moon phases side by side.
    pub celestial: Option<Arc<MinecraftAtlasImage>>,
    pub clouds: Option<Arc<MinecraftClouds>>,
    /// First block, then sky and block light as unorm pairs.
    pub light_volume: Option<Arc<([i32; 3], Vec<u8>)>>,
    pub eye_light: [f32; 2],
    /// Break particles: section vertex bytes and indices.
    pub particles: (Vec<u8>, Vec<u32>),
    pub cracks: (Vec<[f32; 6]>, Vec<u32>),
    /// The ten destroy stages side by side.
    pub crack_texture: Option<Arc<MinecraftAtlasImage>>,
    pub glint_texture: Option<Arc<MinecraftAtlasImage>>,
    pub armor_glint_texture: Option<Arc<MinecraftAtlasImage>>,
    /// Mob models (cut out, back-face culled, translucent) and entity
    /// shadows: MinecraftOSS `Vertex` bytes (44 each) and indices.
    pub entity_meshes: [(Vec<u8>, Vec<u32>); 6],
    /// A black card to draw, in blocks: behind the inventory's character.
    pub backdrop: Option<[[f32; 3]; 4]>,
    pub puppet_gear: (Vec<u8>, Vec<u32>),
    /// The first-person hand or held item in view space, and its projection.
    pub hand: (Vec<u8>, Vec<u32>),
    pub hand_clip: [f32; 16],
    pub end_sky_texture: Option<Arc<MinecraftAtlasImage>>,
    pub end_portal_texture: Option<Arc<MinecraftAtlasImage>>,
    pub end_portal_colors: [[f32; 4]; 16],
    pub end_portal_time: f32,
    pub end_flash: [f32; 3],
    pub weather: (Vec<[f32; 9]>, Vec<u32>),
    pub weather_texture: Option<Arc<MinecraftAtlasImage>>,
}

#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct TerrainView {
    clip_from_rel: [f32; 16],
    rel_from_clip: [f32; 16],
    hand_clip: [f32; 16],
    /// View origin in map units.
    view: [f32; 4],
    /// The block at map origin.
    origin: [f32; 4],
    environment: [[f32; 4]; 16],
    end_portal_colors: [[f32; 4]; 16],
    end_portal: [f32; 4],
    glint: [f32; 4],
}

const VIEW_SIZE: u64 = std::mem::size_of::<TerrainView>() as u64;
const CLOUD_VERTEX_BYTES: u64 = 28;
const CRACK_VERTEX_BYTES: u64 = 24;
const WEATHER_VERTEX_BYTES: u64 = 36;
/// MinecraftOSS's full `Vertex`: position, uv, colour, sky and block light.
const ENTITY_VERTEX_BYTES: u64 = 44;

struct SectionGpu {
    vertices: Buffer,
    indices: Buffer,
    count: u32,
    transparent_start: u32,
}

#[derive(Resource, Default)]
struct TerrainGpu {
    generation: u64,
    view: Option<Buffer>,
    atlas: Option<(Arc<MinecraftAtlasImage>, TextureView)>,
    atlas_texture: Option<Texture>,
    celestial: Option<(Arc<MinecraftAtlasImage>, TextureView)>,
    clouds: Option<(Arc<MinecraftClouds>, Buffer, Buffer, u32)>,
    cloud_top: f32,
    camera_above_clouds: bool,
    clouds_shown: bool,
    crack_texture: Option<(Arc<MinecraftAtlasImage>, TextureView)>,
    glint: Option<(Arc<MinecraftAtlasImage>, TextureView)>,
    armor_glint: Option<(Arc<MinecraftAtlasImage>, TextureView)>,
    glint_sampler: Option<Sampler>,
    end_sky: Option<(Arc<MinecraftAtlasImage>, TextureView)>,
    end_portal: Option<(Arc<MinecraftAtlasImage>, TextureView)>,
    repeat_sampler: Option<Sampler>,
    /// This frame's particles and cracks: vertices, indices, index count.
    particles: Option<(Buffer, Buffer, u32)>,
    cracks: Option<(Buffer, Buffer, u32)>,
    /// This frame's entity meshes, in `MinecraftWorldFrame::entity_meshes` order.
    entities: [Option<(Buffer, Buffer, u32)>; 6],
    backdrop: Option<Buffer>,
    puppet_gear: Option<(Buffer, Buffer, u32)>,
    hand: Option<(Buffer, Buffer, u32)>,
    weather: Option<(Buffer, Buffer, u32)>,
    weather_texture: Option<(Arc<MinecraftAtlasImage>, TextureView)>,
    sampler: Option<Sampler>,
    bind: Option<BindGroup>,
    sections: HashMap<[i32; 3], SectionGpu>,
    visible: Vec<[i32; 3]>,
    pipelines: HashMap<(TextureFormat, u32), [RenderPipeline; 14]>,
}

pub(super) fn register(app: &mut App) {
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app
        .init_resource::<MinecraftWorldFrame>()
        .init_resource::<TerrainGpu>()
        .add_systems(Render, prepare_terrain.in_set(RenderSystems::PrepareResources))
        .add_systems(
            Core3d,
            draw_terrain
                .in_set(Core3dSystems::MainPass)
                .before(super::draw::ExactColourDrawSet),
        );
}

fn layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "iw4l_minecraft_terrain",
        &[
            uniform_buffer_sized(false, std::num::NonZeroU64::new(VIEW_SIZE))
                .visibility(ShaderStages::VERTEX_FRAGMENT)
                .build(0, ShaderStages::VERTEX_FRAGMENT),
            texture_2d(TextureSampleType::Float { filterable: true })
                .visibility(ShaderStages::FRAGMENT)
                .build(1, ShaderStages::FRAGMENT),
            sampler(SamplerBindingType::Filtering)
                .visibility(ShaderStages::FRAGMENT)
                .build(2, ShaderStages::FRAGMENT),
            texture_2d(TextureSampleType::Float { filterable: true })
                .visibility(ShaderStages::FRAGMENT)
                .build(3, ShaderStages::FRAGMENT),
            texture_2d(TextureSampleType::Float { filterable: true })
                .visibility(ShaderStages::FRAGMENT)
                .build(4, ShaderStages::FRAGMENT),
            texture_2d(TextureSampleType::Float { filterable: true })
                .visibility(ShaderStages::FRAGMENT)
                .build(5, ShaderStages::FRAGMENT),
            sampler(SamplerBindingType::Filtering)
                .visibility(ShaderStages::FRAGMENT)
                .build(6, ShaderStages::FRAGMENT),
            texture_2d(TextureSampleType::Float { filterable: true })
                .visibility(ShaderStages::FRAGMENT)
                .build(7, ShaderStages::FRAGMENT),
            texture_2d(TextureSampleType::Float { filterable: true })
                .visibility(ShaderStages::FRAGMENT)
                .build(8, ShaderStages::FRAGMENT),
            texture_2d(TextureSampleType::Float { filterable: true })
                .visibility(ShaderStages::FRAGMENT)
                .build(9, ShaderStages::FRAGMENT),
            sampler(SamplerBindingType::Filtering)
                .visibility(ShaderStages::FRAGMENT)
                .build(10, ShaderStages::FRAGMENT),
            texture_2d(TextureSampleType::Float { filterable: true })
                .visibility(ShaderStages::FRAGMENT)
                .build(11, ShaderStages::FRAGMENT),
        ],
    )
}

static HIDES_MAP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether the loaded IW4 map is only standing in for a Minecraft world, so
/// its own world surfaces are not drawn.
pub(super) fn hides_map() -> bool {
    HIDES_MAP.load(std::sync::atomic::Ordering::Relaxed)
}

fn prepare_terrain(
    mut frame: ResMut<MinecraftWorldFrame>,
    published: Option<Res<super::PublishedRenderFrame>>,
    registry: Res<ExactPipelineRegistry>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut gpu: ResMut<TerrainGpu>,
    mut started: Local<Option<std::time::Instant>>,
) {
    HIDES_MAP.store(frame.active, std::sync::atomic::Ordering::Relaxed);
    if !frame.active {
        frame.atlas_patches.clear();
        if !gpu.sections.is_empty() || gpu.atlas.is_some() {
            *gpu = TerrainGpu {
                pipelines: std::mem::take(&mut gpu.pipelines),
                ..TerrainGpu::default()
            };
        }
        return;
    }
    if gpu.generation != frame.generation {
        gpu.sections.clear();
        gpu.generation = frame.generation;
    }
    if let Some(atlas) = frame.atlas.clone()
        && gpu.atlas.as_ref().is_none_or(|(held, _)| !Arc::ptr_eq(held, &atlas))
    {
        let (view, texture) = upload_atlas(&device, &queue, &atlas);
        gpu.atlas = Some((atlas, view));
        gpu.atlas_texture = Some(texture);
        gpu.bind = None;
    }
    let patches = std::mem::take(&mut frame.atlas_patches);
    if let (Some((atlas, _)), Some(texture)) = (gpu.atlas.as_ref(), gpu.atlas_texture.as_ref()) {
        for patch in &patches {
            for (level, origin, size, texels) in patch_writes(patch, atlas) {
                queue.write_texture(
                    TexelCopyTextureInfo {
                        texture,
                        mip_level: level,
                        origin,
                        aspect: TextureAspect::All,
                    },
                    texels,
                    TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(size.width * 4),
                        rows_per_image: Some(size.height),
                    },
                    size,
                );
            }
        }
    }
    if let Some(celestial) = frame.celestial.clone()
        && gpu
            .celestial
            .as_ref()
            .is_none_or(|(held, _)| !Arc::ptr_eq(held, &celestial))
    {
        let (view, _) = upload_atlas(&device, &queue, &celestial);
        gpu.celestial = Some((celestial, view));
        gpu.bind = None;
    }
    if let Some(cracks) = frame.crack_texture.clone()
        && gpu
            .crack_texture
            .as_ref()
            .is_none_or(|(held, _)| !Arc::ptr_eq(held, &cracks))
    {
        let (view, _) = upload_atlas(&device, &queue, &cracks);
        gpu.crack_texture = Some((cracks, view));
        gpu.bind = None;
    }
    match frame.glint_texture.clone() {
        Some(glint) if gpu.glint.as_ref().is_none_or(|(held, _)| !Arc::ptr_eq(held, &glint)) => {
            let (view, _) = upload_atlas(&device, &queue, &glint);
            gpu.glint = Some((glint, view));
            gpu.bind = None;
        }
        None if gpu.glint.is_none() => {
            let blank = Arc::new(no_glint());
            let (view, _) = upload_atlas(&device, &queue, &blank);
            gpu.glint = Some((blank, view));
            gpu.bind = None;
        }
        _ => {}
    }
    if let Some(armor) = frame.armor_glint_texture.clone()
        && gpu
            .armor_glint
            .as_ref()
            .is_none_or(|(held, _)| !Arc::ptr_eq(held, &armor))
    {
        let (view, _) = upload_atlas(&device, &queue, &armor);
        gpu.armor_glint = Some((armor, view));
        gpu.bind = None;
    }
    let end_sky = frame.end_sky_texture.clone();
    let end_portal = frame.end_portal_texture.clone();
    let sky_changed = upload_or_blank(&device, &queue, &mut gpu.end_sky, end_sky);
    let portal_changed = upload_or_blank(&device, &queue, &mut gpu.end_portal, end_portal);
    let weather_texture = frame.weather_texture.clone();
    let weather_changed = upload_or_blank(&device, &queue, &mut gpu.weather_texture, weather_texture);
    if sky_changed || portal_changed || weather_changed {
        gpu.bind = None;
    }
    let (weather_vertices, weather_indices) = std::mem::take(&mut frame.weather);
    gpu.weather = (!weather_indices.is_empty()).then(|| {
        (
            device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("iw4l_minecraft_weather_vertices"),
                contents: bytemuck::cast_slice(&weather_vertices),
                usage: BufferUsages::VERTEX,
            }),
            device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("iw4l_minecraft_weather_indices"),
                contents: bytemuck::cast_slice(&weather_indices),
                usage: BufferUsages::INDEX,
            }),
            weather_indices.len() as u32,
        )
    });
    let (particle_vertices, particle_indices) = std::mem::take(&mut frame.particles);
    gpu.particles = (!particle_indices.is_empty()).then(|| {
        (
            device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("iw4l_minecraft_particle_vertices"),
                contents: &particle_vertices,
                usage: BufferUsages::VERTEX,
            }),
            device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("iw4l_minecraft_particle_indices"),
                contents: bytemuck::cast_slice(&particle_indices),
                usage: BufferUsages::INDEX,
            }),
            particle_indices.len() as u32,
        )
    });
    gpu.backdrop = frame.backdrop.take().map(|[a, b, c, d]| {
        let corners: [[f32; 3]; 6] = [a, b, c, a, c, d];
        device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("iw4l_minecraft_backdrop"),
            contents: bytemuck::cast_slice(&corners),
            usage: BufferUsages::VERTEX,
        })
    });
    let (gear_vertices, gear_indices) = std::mem::take(&mut frame.puppet_gear);
    gpu.puppet_gear = (!gear_indices.is_empty()).then(|| {
        (
            device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("iw4l_minecraft_puppet_gear_vertices"),
                contents: &gear_vertices,
                usage: BufferUsages::VERTEX,
            }),
            device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("iw4l_minecraft_puppet_gear_indices"),
                contents: bytemuck::cast_slice(&gear_indices),
                usage: BufferUsages::INDEX,
            }),
            gear_indices.len() as u32,
        )
    });
    let (hand_vertices, hand_indices) = std::mem::take(&mut frame.hand);
    gpu.hand = (!hand_indices.is_empty()).then(|| {
        (
            device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("iw4l_minecraft_hand_vertices"),
                contents: &hand_vertices,
                usage: BufferUsages::VERTEX,
            }),
            device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("iw4l_minecraft_hand_indices"),
                contents: bytemuck::cast_slice(&hand_indices),
                usage: BufferUsages::INDEX,
            }),
            hand_indices.len() as u32,
        )
    });
    let meshes = std::mem::take(&mut frame.entity_meshes);
    for (slot, (vertices, indices)) in gpu.entities.iter_mut().zip(meshes) {
        *slot = (!indices.is_empty()).then(|| {
            (
                device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("iw4l_minecraft_entity_vertices"),
                    contents: &vertices,
                    usage: BufferUsages::VERTEX,
                }),
                device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("iw4l_minecraft_entity_indices"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: BufferUsages::INDEX,
                }),
                indices.len() as u32,
            )
        });
    }
    let (crack_vertices, crack_indices) = std::mem::take(&mut frame.cracks);
    gpu.cracks = (!crack_indices.is_empty()).then(|| {
        (
            device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("iw4l_minecraft_crack_vertices"),
                contents: bytemuck::cast_slice(&crack_vertices),
                usage: BufferUsages::VERTEX,
            }),
            device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("iw4l_minecraft_crack_indices"),
                contents: bytemuck::cast_slice(&crack_indices),
                usage: BufferUsages::INDEX,
            }),
            crack_indices.len() as u32,
        )
    });
    match frame.clouds.clone() {
        Some(clouds) if gpu.clouds.as_ref().is_none_or(|(held, ..)| !Arc::ptr_eq(held, &clouds)) => {
            gpu.cloud_top = clouds.vertices.iter().map(|vertex| vertex[1]).fold(f32::NEG_INFINITY, f32::max);
            gpu.clouds = (!clouds.indices.is_empty()).then(|| {
                let vertices = device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("iw4l_minecraft_cloud_vertices"),
                    contents: bytemuck::cast_slice(&clouds.vertices),
                    usage: BufferUsages::VERTEX,
                });
                let indices = device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("iw4l_minecraft_cloud_indices"),
                    contents: bytemuck::cast_slice(&clouds.indices),
                    usage: BufferUsages::INDEX,
                });
                let count = clouds.indices.len() as u32;
                (clouds, vertices, indices, count)
            });
        }
        Some(_) => {}
        None => gpu.clouds = None,
    }
    gpu.camera_above_clouds = frame.environment[3][1] > gpu.cloud_top;
    gpu.clouds_shown = clouds_shown(&frame.environment);
    if gpu.view.is_none() {
        gpu.view = Some(device.create_buffer(&BufferDescriptor {
            label: Some("iw4l_minecraft_terrain_view"),
            size: VIEW_SIZE,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        gpu.sampler = Some(device.create_sampler(&SamplerDescriptor {
            label: Some("iw4l_minecraft_terrain"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Linear,
            ..default()
        }));
        gpu.glint_sampler = Some(device.create_sampler(&SamplerDescriptor {
            label: Some("iw4l_minecraft_glint"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..default()
        }));
        gpu.repeat_sampler = Some(device.create_sampler(&SamplerDescriptor {
            label: Some("iw4l_minecraft_repeat"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..default()
        }));
    }
    if gpu.bind.is_none()
        && let (Some(view), Some((_, atlas)), Some((_, celestial)), Some((_, cracks)), Some(sampler), Some((_, glint)), Some(glint_sampler)) = (
            gpu.view.as_ref(),
            gpu.atlas.as_ref(),
            gpu.celestial.as_ref(),
            gpu.crack_texture.as_ref(),
            gpu.sampler.as_ref(),
            gpu.glint.as_ref(),
            gpu.glint_sampler.as_ref(),
        )
        && let (Some((_, end_sky)), Some((_, end_portal)), Some(repeat_sampler)) = (gpu.end_sky.as_ref(), gpu.end_portal.as_ref(), gpu.repeat_sampler.as_ref())
        && let Some((_, weather_texture)) = gpu.weather_texture.as_ref()
    {
        let armor_glint = gpu.armor_glint.as_ref().map_or(glint, |(_, view)| view);
        let layout = registry.bind_group_layout(&device, &layout());
        let bind = device.create_bind_group(
            "iw4l_minecraft_terrain",
            &layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: view,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(atlas),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(celestial),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(cracks),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(glint),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::Sampler(glint_sampler),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::TextureView(armor_glint),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::TextureView(end_sky),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: BindingResource::TextureView(end_portal),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: BindingResource::Sampler(repeat_sampler),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: BindingResource::TextureView(weather_texture),
                },
            ],
        );
        gpu.bind = Some(bind);
    }
    for pos in std::mem::take(&mut frame.removed) {
        gpu.sections.remove(&pos);
    }
    for upload in std::mem::take(&mut frame.uploads) {
        if upload.vertices.is_empty() && !upload.indices.is_empty() {
            if let (Some(section), Some(start)) = (gpu.sections.get(&upload.pos), upload.transparent_start)
                && start == section.transparent_start
                && start as usize + upload.indices.len() == section.count as usize
            {
                queue.write_buffer(&section.indices, u64::from(start) * 4, bytemuck::cast_slice(&upload.indices));
            }
            continue;
        }
        if upload.indices.is_empty() || upload.vertices.is_empty() {
            gpu.sections.remove(&upload.pos);
            continue;
        }
        let vertices = device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("iw4l_minecraft_section_vertices"),
            contents: &upload.vertices,
            usage: BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("iw4l_minecraft_section_indices"),
            contents: bytemuck::cast_slice(&upload.indices),
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
        });
        let count = upload.indices.len() as u32;
        gpu.sections.insert(
            upload.pos,
            SectionGpu {
                vertices,
                indices,
                count,
                transparent_start: upload.transparent_start.unwrap_or(count).min(count),
            },
        );
    }
    gpu.visible = frame.visible.clone();

    let shift = frame::minecraft_ui::glint_shift(started.get_or_insert_with(std::time::Instant::now).elapsed().as_millis() as u64);
    let mut view = TerrainView {
        environment: frame.environment,
        hand_clip: frame.hand_clip,
        glint: [shift[0], shift[1], frame::minecraft_ui::GLINT_STRENGTH, 0.0],
        end_portal_colors: frame.end_portal_colors,
        end_portal: [frame.end_portal_time, frame.end_flash[0], frame.end_flash[1], frame.end_flash[2]],
        ..TerrainView::default()
    };
    if let Some(exec) = published.as_ref().map(|p| &p.exec_frame)
        && let Some(clip_from_world) = exec.clip_from_world
    {
        let o = exec.view_origin;
        let clip_from_rel = clip_from_world * Mat4::from_translation(o);
        view.clip_from_rel = clip_from_rel.to_cols_array();
        view.rel_from_clip = clip_from_rel.inverse().to_cols_array();
        view.view = [o.x, o.y, o.z, 1.0];
    }
    view.origin = [
        frame.origin[0] as f32,
        frame.origin[1] as f32,
        frame.origin[2] as f32,
        frame.view_distance * 16.0,
    ];
    if let Some(buffer) = gpu.view.as_ref() {
        queue.write_buffer(buffer, 0, bytemuck::bytes_of(&view));
    }
}

fn upload_or_blank(
    device: &RenderDevice,
    queue: &RenderQueue,
    held: &mut Option<(Arc<MinecraftAtlasImage>, TextureView)>,
    image: Option<Arc<MinecraftAtlasImage>>,
) -> bool {
    let image = match image {
        Some(image) if held.as_ref().is_none_or(|(old, _)| !Arc::ptr_eq(old, &image)) => image,
        None if held.is_none() => Arc::new(no_glint()),
        _ => return false,
    };
    let (view, _) = upload_atlas(device, queue, &image);
    *held = Some((image, view));
    true
}

fn clouds_shown(environment: &[[f32; 4]; 16]) -> bool {
    environment[11][1] > 0.0
}

fn no_glint() -> MinecraftAtlasImage {
    MinecraftAtlasImage {
        width: 1,
        height: 1,
        levels: vec![vec![0, 0, 0, 255]],
    }
}

fn patch_writes<'a>(patch: &'a MinecraftAtlasPatch, atlas: &MinecraftAtlasImage) -> Vec<(u32, Origin3d, Extent3d, &'a [u8])> {
    patch
        .levels
        .iter()
        .zip(&atlas.levels)
        .enumerate()
        .filter_map(|(level, ((width, height, texels), _))| {
            let (x, y) = (patch.x >> level, patch.y >> level);
            let (atlas_width, atlas_height) = ((atlas.width >> level).max(1), (atlas.height >> level).max(1));
            let fits = *width > 0
                && *height > 0
                && x + width <= atlas_width
                && y + height <= atlas_height
                && texels.len() >= (width * height * 4) as usize;
            fits.then(|| {
                (
                    level as u32,
                    Origin3d { x, y, z: 0 },
                    Extent3d { width: *width, height: *height, depth_or_array_layers: 1 },
                    &texels[..(width * height * 4) as usize],
                )
            })
        })
        .collect()
}

fn upload_atlas(device: &RenderDevice, queue: &RenderQueue, atlas: &MinecraftAtlasImage) -> (TextureView, Texture) {
    let levels = atlas.levels.len().max(1) as u32;
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("iw4l_minecraft_atlas"),
        size: Extent3d {
            width: atlas.width,
            height: atlas.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: levels,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, texels) in atlas.levels.iter().enumerate() {
        let (w, h) = ((atlas.width >> level).max(1), (atlas.height >> level).max(1));
        if texels.len() < (w * h * 4) as usize {
            break;
        }
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            texels,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
    }
    (texture.create_view(&TextureViewDescriptor::default()), texture)
}

fn draw_terrain(
    view: ViewQuery<(&ViewTarget, &SceneDepthTexture, &ExtractedView, Option<&Msaa>)>,
    registry: Res<ExactPipelineRegistry>,
    device: Res<RenderDevice>,
    mut gpu: ResMut<TerrainGpu>,
    mut context: RenderContext,
) {
    let Some(bind) = gpu.bind.clone() else {
        return;
    };
    let (target, depth, extracted_view, msaa) = view.into_inner();
    let format = target.main_texture_format();
    let samples = msaa.map_or(1, Msaa::samples);
    let [sky, opaque, translucent, clouds, crack, entity, entity_culled, entity_translucent, shadow, backdrop, hand, sections, weather, glow] = gpu
        .pipelines
        .entry((format, samples))
        .or_insert_with(|| pipelines(&device, &registry, format, samples))
        .clone();
    let attachments = [Some(target.get_color_attachment())];
    let mut pass =
        context.begin_tracked_render_pass(bevy::render::render_resource::RenderPassDescriptor {
            label: Some("iw4l_minecraft_terrain"),
            color_attachments: &attachments,
            depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    let vp = extracted_view.viewport;
    let (depth_min, depth_max) = reverse_z_viewport_depth(GFX_DEPTH_RANGE_SCENE);
    pass.set_viewport(vp.x as f32, vp.y as f32, vp.z as f32, vp.w as f32, depth_min, depth_max);
    pass.set_bind_group(0, &bind, &[]);
    pass.set_render_pipeline(&sections);
    for pos in &gpu.visible {
        let Some(section) = gpu.sections.get(pos) else {
            continue;
        };
        if section.transparent_start == 0 {
            continue;
        }
        pass.set_vertex_buffer(0, section.vertices.slice(..));
        pass.set_index_buffer(section.indices.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..section.transparent_start, 0, 0..1);
    }
    pass.set_render_pipeline(&opaque);
    if let Some((vertices, indices, count)) = gpu.particles.as_ref() {
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_index_buffer(indices.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..*count, 0, 0..1);
    }
    if let Some(card) = gpu.backdrop.as_ref() {
        let (band_min, band_max) = reverse_z_viewport_depth(super::depth_range::GFX_DEPTH_RANGE_VIEWMODEL);
        pass.set_viewport(vp.x as f32, vp.y as f32, vp.z as f32, vp.w as f32, band_min, band_max);
        pass.set_render_pipeline(&backdrop);
        pass.set_vertex_buffer(0, card.slice(..));
        pass.draw(0..6, 0..1);
        pass.set_viewport(vp.x as f32, vp.y as f32, vp.z as f32, vp.w as f32, depth_min, depth_max);
        pass.set_render_pipeline(&opaque);
    }
    if let Some((vertices, indices, count)) = gpu.puppet_gear.as_ref() {
        let (band_min, band_max) = reverse_z_viewport_depth(super::depth_range::GFX_DEPTH_RANGE_VIEWMODEL);
        pass.set_viewport(vp.x as f32, vp.y as f32, vp.z as f32, vp.w as f32, band_min, band_max);
        pass.set_render_pipeline(&opaque);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_index_buffer(indices.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..*count, 0, 0..1);
        pass.set_viewport(vp.x as f32, vp.y as f32, vp.z as f32, vp.w as f32, depth_min, depth_max);
    }
    // Mobs, then their shadows on what is drawn so far.
    for (pipeline, mesh) in [(&entity, &gpu.entities[0]), (&entity_culled, &gpu.entities[1]), (&shadow, &gpu.entities[3])] {
        if let Some((vertices, indices, count)) = mesh.as_ref() {
            pass.set_render_pipeline(pipeline);
            pass.set_vertex_buffer(0, vertices.slice(..));
            pass.set_index_buffer(indices.slice(..), IndexFormat::Uint32);
            pass.draw_indexed(0..*count, 0, 0..1);
        }
    }
    // Sky wherever nothing has been drawn yet: a full-screen triangle at the
    // cleared depth.
    pass.set_viewport(vp.x as f32, vp.y as f32, vp.z as f32, vp.w as f32, 0.0, 0.0);
    pass.set_render_pipeline(&sky);
    pass.draw(0..3, 0..1);
    pass.set_viewport(vp.x as f32, vp.y as f32, vp.z as f32, vp.w as f32, depth_min, depth_max);
    if let Some((vertices, indices, count)) = gpu.entities[2].as_ref() {
        pass.set_render_pipeline(&entity_translucent);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_index_buffer(indices.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..*count, 0, 0..1);
    }
    if !gpu.camera_above_clouds
        && gpu.clouds_shown
        && let Some((_, vertices, indices, count)) = gpu.clouds.as_ref()
    {
        pass.set_render_pipeline(&clouds);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_index_buffer(indices.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..*count, 0, 0..1);
    }
    pass.set_render_pipeline(&translucent);
    for pos in gpu.visible.iter().rev() {
        let Some(section) = gpu.sections.get(pos) else {
            continue;
        };
        if section.transparent_start >= section.count {
            continue;
        }
        pass.set_vertex_buffer(0, section.vertices.slice(..));
        pass.set_index_buffer(section.indices.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(section.transparent_start..section.count, 0, 0..1);
    }
    if let Some((vertices, indices, count)) = gpu.cracks.as_ref() {
        pass.set_render_pipeline(&crack);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_index_buffer(indices.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..*count, 0, 0..1);
    }
    if let Some((vertices, indices, count)) = gpu.weather.as_ref() {
        pass.set_render_pipeline(&weather);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_index_buffer(indices.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..*count, 0, 0..1);
    }
    for (pipeline, mesh) in [(&glow, &gpu.entities[4]), (&entity, &gpu.entities[5])] {
        if let Some((vertices, indices, count)) = mesh.as_ref() {
            pass.set_render_pipeline(pipeline);
            pass.set_vertex_buffer(0, vertices.slice(..));
            pass.set_index_buffer(indices.slice(..), IndexFormat::Uint32);
            pass.draw_indexed(0..*count, 0, 0..1);
        }
    }
    // The hand in front of everything, as the view model is.
    if let Some((vertices, indices, count)) = gpu.hand.as_ref() {
        let (band_min, band_max) = reverse_z_viewport_depth(super::depth_range::GFX_DEPTH_RANGE_VIEWMODEL);
        pass.set_viewport(vp.x as f32, vp.y as f32, vp.z as f32, vp.w as f32, band_min, band_max);
        pass.set_render_pipeline(&hand);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_index_buffer(indices.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..*count, 0, 0..1);
        pass.set_viewport(vp.x as f32, vp.y as f32, vp.z as f32, vp.w as f32, depth_min, depth_max);
    }
    if gpu.camera_above_clouds
        && gpu.clouds_shown
        && let Some((_, vertices, indices, count)) = gpu.clouds.as_ref()
    {
        pass.set_render_pipeline(&clouds);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_index_buffer(indices.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..*count, 0, 0..1);
    }
}

const TERRAIN_WGSL: &str = r#"
// MinecraftOSS's environment uniform (viewer/src/sky.wgsl), in blocks.
struct Environment {
    forward: vec4<f32>, right: vec4<f32>, up: vec4<f32>, camera_pos: vec4<f32>,
    sky: vec4<f32>, fog: vec4<f32>, light: vec4<f32>, sunset: vec4<f32>,
    sun_dir: vec4<f32>, moon_dir: vec4<f32>, cloud: vec4<f32>, params: vec4<f32>,
    extra: vec4<f32>,
    fog_distances: vec4<f32>,
    ambient: vec4<f32>,
    block_tint: vec4<f32>,
}
struct TerrainView {
    clip_from_rel: mat4x4<f32>,
    rel_from_clip: mat4x4<f32>,
    hand_clip: mat4x4<f32>,
    view: vec4<f32>,
    origin: vec4<f32>,
    environment: Environment,
    end_portal_colors: array<vec4<f32>, 16>,
    end_portal: vec4<f32>,
    glint: vec4<f32>,
}
@group(0) @binding(0) var<uniform> view: TerrainView;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;
@group(0) @binding(3) var celestials: texture_2d<f32>;
@group(0) @binding(4) var cracks: texture_2d<f32>;
@group(0) @binding(5) var glint_texture: texture_2d<f32>;
@group(0) @binding(6) var glint_sampler: sampler;
@group(0) @binding(7) var armor_glint_texture: texture_2d<f32>;
@group(0) @binding(8) var end_sky_texture: texture_2d<f32>;
@group(0) @binding(9) var end_portal_texture: texture_2d<f32>;
@group(0) @binding(10) var repeat_sampler: sampler;
@group(0) @binding(11) var weather_texture: texture_2d<f32>;

// Map units per block, as `sim::voxel::BLOCK`.
const BLOCK: f32 = 36.0;

// Block space to map space relative to the view.
fn rel_from_block(position: vec3<f32>) -> vec3<f32> {
    let rel = position - view.origin.xyz;
    return vec3<f32>(rel.x, -rel.z, rel.y) * BLOCK - view.view.xyz;
}

fn light_brightness(level: f32) -> f32 {
    return level / (4.0 - 3.0 * level);
}

// Minecraft 26.3 lightmap.fsh as MinecraftOSS ports it: ambient, sky light
// scaled by the sky factor, and tinted block light, then the brightness
// option's notGamma blend.
fn lightmap(sky_level: f32, block_level: f32) -> vec3<f32> {
    let environment = view.environment;
    let sky = sky_level / 15.0;
    let block = block_level / 15.0;
    var color = environment.ambient.rgb;
    color += environment.light.rgb * (light_brightness(sky) * environment.light.w);
    let parabolic = (2.0 * block - 1.0) * (2.0 * block - 1.0);
    let block_color = mix(environment.block_tint.rgb, vec3<f32>(1.0), 0.9 * parabolic);
    color += block_color * (light_brightness(block) * environment.block_tint.w);
    color = color - vec3<f32>(environment.forward.w);
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
    let greatest = max(color.r, max(color.g, color.b));
    let inverted = 1.0 - greatest;
    let gamma = color * ((1.0 - inverted * inverted * inverted * inverted) / max(greatest, 0.00001));
    return mix(color, gamma, environment.cloud.w);
}

// Minecraft 26.3 fog.glsl: the larger of spherical environmental fog and
// cylindrical render-distance fog, each linear between its start and end.
fn linear_fog_value(vertex_distance: f32, fog_start: f32, fog_end: f32) -> f32 {
    if vertex_distance <= fog_start { return 0.0; }
    if vertex_distance >= fog_end { return 1.0; }
    return (vertex_distance - fog_start) / (fog_end - fog_start);
}

fn fog_value(world_pos: vec3<f32>) -> f32 {
    let environment = view.environment;
    let pos = world_pos - environment.camera_pos.xyz;
    let spherical = length(pos);
    let cylindrical = max(length(pos.xz), abs(pos.y));
    return max(
        linear_fog_value(spherical, environment.fog_distances.x, environment.fog_distances.y),
        linear_fog_value(cylindrical, environment.fog_distances.z, environment.fog_distances.w),
    );
}

struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colour: vec4<f32>,
    @location(2) world_pos: vec3<f32>,
    @location(3) glint: f32,
    @location(4) screen: vec4<f32>,
}

@vertex
fn vertex(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) colour: vec4<f32>,
    @location(3) light: vec2<f32>,
    @location(4) flags: vec2<f32>,
) -> Out {
    var out: Out;
    out.clip = view.clip_from_rel * vec4<f32>(rel_from_block(position), 1.0);
    out.uv = uv;
    // Unorm of level * 16 back to a level in 0..15.
    let level = light * (255.0 / 16.0);
    out.colour = vec4<f32>(colour.rgb * lightmap(level.x, level.y), colour.a);
    out.world_pos = position;
    out.glint = flags.x;
    out.screen = vec4<f32>((out.clip.xy + out.clip.w) * 0.5, out.clip.zw);
    return out;
}

const END_PORTAL_ALPHA_BELOW: f32 = 0.375;
const END_GATEWAY_ALPHA_BELOW: f32 = 0.1875;

fn end_portal_uv(layer: f32, screen: vec2<f32>) -> vec2<f32> {
    let scale = (4.5 - layer / 4.0) * 2.0;
    let angle = radians((layer * layer * 4321.0 + layer * 9.0) * 2.0);
    let c = cos(angle);
    let s = sin(angle);
    let turned = scale * vec2<f32>(screen.x * c - screen.y * s, screen.x * s + screen.y * c);
    let moved = turned + vec2<f32>(17.0 / layer, (2.0 + layer / 1.5) * (view.end_portal.x * 1.5));
    return moved * 0.5 + 0.25;
}

fn end_portal_colour(projected: vec4<f32>, gateway: bool) -> vec3<f32> {
    let screen = projected.xy / projected.w;
    var color = textureSampleLevel(end_sky_texture, repeat_sampler, screen, 0.0).rgb * view.end_portal_colors[0].rgb;
    let layers = select(15, 16, gateway);
    for (var i = 0; i < layers; i++) {
        let texel = textureSampleLevel(end_portal_texture, repeat_sampler, end_portal_uv(f32(i + 1), screen), 0.0);
        color += texel.rgb * view.end_portal_colors[i].rgb;
    }
    return color;
}

// Minecraft's values are display values, as the target's are.
fn shade(in: Out, texel: vec4<f32>) -> vec4<f32> {
    let lit = texel.rgb * in.colour.rgb;
    return vec4<f32>(mix(lit, view.environment.fog.rgb, fog_value(in.world_pos)), texel.a * in.colour.a);
}

fn glint_light(uv: vec2<f32>, flag: f32) -> vec3<f32> {
    if flag <= 0.0 {
        return vec3<f32>(0.0);
    }
    let armour = flag < 0.75;
    let p = uv * vec2<f32>(textureDimensions(atlas, 0)) * select(8.0 / 512.0, 0.16 / 64.0, armour);
    let c = cos(0.17453292);
    let s = sin(0.17453292);
    let g = view.glint.xy + vec2<f32>(c * p.x - s * p.y, s * p.x + c * p.y);
    let item = textureSampleLevel(glint_texture, glint_sampler, g, 0.0).rgb;
    let worn = textureSampleLevel(armor_glint_texture, glint_sampler, g, 0.0).rgb;
    let t = select(item, worn, armour) * view.glint.z;
    return t * t;
}

@fragment
fn opaque(in: Out) -> @location(0) vec4<f32> {
    let entity_sheet = in.colour.a < 0.75;
    let level_zero = vec2<f32>(0.0);
    let texel = textureSampleGrad(atlas, atlas_sampler, in.uv, select(dpdx(in.uv), level_zero, entity_sheet), select(dpdy(in.uv), level_zero, entity_sheet));
    if in.colour.a < END_PORTAL_ALPHA_BELOW {
        let portal = end_portal_colour(in.screen, in.colour.a < END_GATEWAY_ALPHA_BELOW);
        return vec4<f32>(mix(portal, view.environment.fog.rgb, fog_value(in.world_pos)), 1.0);
    }
    if texel.a < select(0.5, 0.1, entity_sheet) {
        discard;
    }
    let lit = texel.rgb * in.colour.rgb + glint_light(in.uv, in.glint);
    return vec4<f32>(mix(lit, view.environment.fog.rgb, fog_value(in.world_pos)), 1.0);
}

@fragment
fn translucent(in: Out) -> @location(0) vec4<f32> {
    let texel = textureSample(atlas, atlas_sampler, in.uv);
    return shade(in, texel);
}

// viewer/src/sky.wgsl, with the view ray taken from the MW2 camera.
struct SkyOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) ndc: vec2<f32>,
}

const SKYBOX_NONE_ABOVE: f32 = 1.5;
const SKYBOX_END_ABOVE: f32 = 0.5;
const END_SKY_SHADE: f32 = 40.0 / 255.0;
const CELESTIAL_SLOTS: f32 = 10.0;
const END_FLASH_SLOT: f32 = 9.0;
const END_FLASH_HALF_SIZE: f32 = 0.6;
const END_FLASH_SHOWN_ABOVE: f32 = 0.00001;

fn end_sky_uv(ray: vec3<f32>) -> vec2<f32> {
    let size = abs(ray);
    var local: vec3<f32>;
    if size.y >= size.x && size.y >= size.z {
        local = select(ray, vec3<f32>(ray.x, -ray.y, -ray.z), ray.y > 0.0);
    } else if size.z >= size.x {
        local = select(vec3<f32>(ray.x, ray.z, -ray.y), vec3<f32>(ray.x, -ray.z, ray.y), ray.z > 0.0);
    } else {
        local = select(vec3<f32>(-ray.y, ray.x, ray.z), vec3<f32>(ray.y, -ray.x, ray.z), ray.x > 0.0);
    }
    return (local.xz / -local.y + 1.0) * 8.0;
}

fn end_sky(ray: vec3<f32>) -> vec3<f32> {
    let texel = textureSampleLevel(end_sky_texture, repeat_sampler, end_sky_uv(ray), 0.0);
    return mix(view.environment.fog.rgb, texel.rgb * END_SKY_SHADE, texel.a);
}

fn end_flash(ray: vec3<f32>, color: vec3<f32>) -> vec3<f32> {
    let intensity = view.end_portal.y;
    if intensity <= END_FLASH_SHOWN_ABOVE { return color; }
    let x = radians(view.end_portal.z);
    let y = radians(view.end_portal.w);
    let direction = vec3<f32>(-sin(y) * cos(x), -sin(x), cos(y) * cos(x));
    let facing = dot(ray, direction);
    if facing <= 0.0 { return color; }
    let axis_x = vec3<f32>(-cos(y), 0.0, -sin(y));
    let axis_z = vec3<f32>(-sin(x) * sin(y), cos(x), sin(x) * cos(y));
    let projected = ray / facing;
    let quad = vec2<f32>(dot(projected, axis_x), dot(projected, axis_z)) / END_FLASH_HALF_SIZE;
    if any(abs(quad) > vec2<f32>(1.0)) { return color; }
    let uv = (quad + 1.0) * 0.5;
    let texel = textureSampleLevel(celestials, atlas_sampler, vec2<f32>((END_FLASH_SLOT + uv.x) / CELESTIAL_SLOTS, uv.y), 0.0);
    if texel.a <= 0.0 { return color; }
    return min(color + texel.rgb * texel.a * intensity * intensity, vec3<f32>(1.0));
}

@vertex
fn sky_vertex(@builtin(vertex_index) index: u32) -> SkyOut {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: SkyOut;
    out.ndc = uv * 2.0 - 1.0;
    out.clip = vec4<f32>(out.ndc, 0.0, 1.0);
    return out;
}

fn celestial(ray: vec3<f32>, direction: vec3<f32>, half_size: f32, slot: f32, moon: bool) -> vec4<f32> {
    let facing = dot(ray, direction);
    if facing <= 0.0 { return vec4<f32>(0.0); }
    // SkyRenderer rotates the XZ quad by Y=-90 degrees and then by the
    // celestial X angle: local +X becomes world +Z, local +Z becomes
    // (-direction.y, direction.x, 0). Its quads sit 100 units from the eye.
    let tangent_u = vec3<f32>(0.0, 0.0, 1.0);
    let tangent_v = vec3<f32>(-direction.y, direction.x, 0.0);
    let projected = ray / facing;
    var uv = vec2<f32>(dot(projected, tangent_u), dot(projected, tangent_v)) / (2.0 * half_size) + 0.5;
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) { return vec4<f32>(0.0); }
    // The moon phase quad reverses both texture axes in buildMoonPhases.
    if moon { uv = vec2<f32>(1.0) - uv; }
    let atlas_uv = vec2<f32>((slot + uv.x) / CELESTIAL_SLOTS, uv.y);
    return textureSampleLevel(celestials, atlas_sampler, atlas_uv, 0.0);
}

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn stars(ray: vec3<f32>) -> f32 {
    let angle = view.environment.params.w;
    let ca = cos(angle);
    let sa = sin(angle);
    let turned = vec3<f32>(ray.x, ray.y * ca - ray.z * sa, ray.y * sa + ray.z * ca);
    let spherical = vec2<f32>(atan2(turned.z, turned.x) / 6.2831853 + 0.5, asin(clamp(turned.y, -1.0, 1.0)) / 3.14159265 + 0.5);
    let cell_uv = spherical * vec2<f32>(320.0, 160.0);
    let cell = floor(cell_uv);
    let seed = hash2(cell);
    if seed < 0.982 { return 0.0; }
    let center = vec2<f32>(hash2(cell + 9.0), hash2(cell + 31.0));
    let size = 0.045 + hash2(cell + 51.0) * 0.035;
    let offset = abs(fract(cell_uv) - center);
    return select(0.0, view.environment.params.z, all(offset < vec2<f32>(size)));
}

@fragment
fn sky_fragment(in: SkyOut) -> @location(0) vec4<f32> {
    let environment = view.environment;
    if environment.ambient.w > SKYBOX_NONE_ABOVE {
        return vec4<f32>(environment.fog.rgb, 1.0);
    }
    let far = view.rel_from_clip * vec4<f32>(in.ndc, 0.5, 1.0);
    let map_ray = far.xyz / far.w;
    let ray = normalize(vec3<f32>(map_ray.x, map_ray.z, -map_ray.y));
    if environment.ambient.w > SKYBOX_END_ABOVE {
        return vec4<f32>(end_flash(ray, end_sky(ray)), 1.0);
    }
    // SkyRenderer's 16-block-high fan has a 512-block radius. Its fog value
    // interpolates between the center and rim vertex distances.
    var color = environment.fog.rgb;
    if ray.y > 0.0 {
        let radius = 16.0 * length(ray.xz) / ray.y;
        if radius < 512.0 {
            let vertex_distance = mix(16.0, length(vec2<f32>(512.0, 16.0)), radius / 512.0);
            color = mix(environment.sky.rgb, environment.fog.rgb, clamp(vertex_distance / environment.fog.w, 0.0, 1.0));
        }
    }
    if ray.y > -0.01 {
        let star = stars(ray) * smoothstep(-0.01, 0.07, ray.y);
        color = mix(color, vec3<f32>(1.0), star);
    }
    let sun_horizontal = normalize(vec3<f32>(environment.sun_dir.x, 0.0, environment.sun_dir.z + 0.0001));
    let view_horizontal = normalize(vec3<f32>(ray.x, 0.0, ray.z + 0.0001));
    let sunset = environment.sunset.a * pow(max(dot(sun_horizontal, view_horizontal), 0.0), 8.0)
        * (1.0 - smoothstep(0.0, 0.35, abs(ray.y)));
    color = mix(color, environment.sunset.rgb, sunset);
    let sun = celestial(ray, environment.sun_dir.xyz, 0.3, 0.0, false);
    // Minecraft's celestial pipeline uses OVERLAY (source alpha, destination one).
    color = min(color + sun.rgb * sun.a * environment.sun_dir.w, vec3<f32>(1.0));
    let moon = celestial(ray, environment.moon_dir.xyz, 0.2, environment.extra.x + 1.0, true);
    color = min(color + moon.rgb * moon.a * environment.moon_dir.w, vec3<f32>(1.0));
    return vec4<f32>(color, 1.0);
}

// viewer/src/clouds.wgsl.
struct CloudOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) colour: vec4<f32>,
    @location(1) distance: f32,
}

@vertex
fn cloud_vertex(@location(0) position: vec3<f32>, @location(1) colour: vec4<f32>) -> CloudOut {
    // CloudRenderer moves the texture at 0.03 blocks per game tick, with a
    // fixed 3.96-block Z phase. Geometry is rebuilt only on cell boundaries.
    let world = position - vec3<f32>(view.environment.extra.y, 0.0, 3.96);
    var out: CloudOut;
    out.clip = view.clip_from_rel * vec4<f32>(rel_from_block(world), 1.0);
    out.colour = colour;
    out.distance = distance(world, view.environment.camera_pos.xyz);
    return out;
}

@fragment
fn cloud_fragment(in: CloudOut) -> @location(0) vec4<f32> {
    let alpha = 0.8 * (1.0 - clamp(in.distance / 1024.0, 0.0, 1.0));
    return vec4<f32>(view.environment.cloud.rgb * in.colour.rgb, alpha);
}

// Mobs: shader.wgsl `vs_main` with `fs_entity` (the entity overlay rides in
// vertex alpha: negative for the hurt flash's red, else the white's alpha)
// and `fs_entity_translucent`; entity textures have no mipmaps.
struct EntityOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colour: vec4<f32>,
    @location(2) world_pos: vec3<f32>,
    @location(3) light: vec3<f32>,
}

@vertex
fn entity_vertex(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) colour: vec4<f32>,
    @location(3) sky_light: f32,
    @location(4) block_light: f32,
) -> EntityOut {
    var out: EntityOut;
    out.clip = view.clip_from_rel * vec4<f32>(rel_from_block(position), 1.0);
    out.uv = uv;
    let light = lightmap(sky_light, block_light);
    out.colour = vec4<f32>(colour.rgb * light, colour.a);
    out.world_pos = position;
    out.light = light;
    return out;
}

@fragment
fn entity_fragment(in: EntityOut) -> @location(0) vec4<f32> {
    let texel = textureSampleLevel(atlas, atlas_sampler, in.uv, 0.0);
    if texel.a < 0.1 {
        discard;
    }
    let keep = abs(in.colour.a);
    let overlay = select(vec3<f32>(1.0), vec3<f32>(1.0, 0.0, 0.0), in.colour.a < 0.0);
    let lit = texel.rgb * in.colour.rgb * keep + overlay * (1.0 - keep) * in.light;
    return vec4<f32>(mix(lit, view.environment.fog.rgb, fog_value(in.world_pos)), 1.0);
}

@fragment
fn glow_fragment(in: EntityOut) -> @location(0) vec4<f32> {
    let texel = textureSampleLevel(atlas, atlas_sampler, in.uv, 0.0);
    if texel.a < 0.1 {
        discard;
    }
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}

@fragment
fn entity_translucent_fragment(in: EntityOut) -> @location(0) vec4<f32> {
    let texel = textureSampleLevel(atlas, atlas_sampler, in.uv, 0.0);
    if texel.a < 0.1 {
        discard;
    }
    let lit = texel.rgb * in.colour.rgb;
    return vec4<f32>(mix(lit, view.environment.fog.rgb, fog_value(in.world_pos)), texel.a * in.colour.a);
}

// The first-person hand: view-space vertices under the hand projection,
// lit by the lightmap, cut out as items are (below 0.1).
@vertex
fn hand_vertex(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) colour: vec4<f32>,
    @location(3) light: vec2<f32>,
    @location(4) flags: vec2<f32>,
) -> Out {
    var out: Out;
    out.clip = view.hand_clip * vec4<f32>(position, 1.0);
    out.uv = uv;
    let level = light * (255.0 / 16.0);
    out.colour = vec4<f32>(colour.rgb * lightmap(level.x, level.y), colour.a);
    out.world_pos = view.environment.camera_pos.xyz;
    out.glint = flags.x;
    return out;
}

@fragment
fn hand_fragment(in: Out) -> @location(0) vec4<f32> {
    let texel = textureSampleLevel(atlas, atlas_sampler, in.uv, 0.0);
    if texel.a < 0.1 {
        discard;
    }
    return vec4<f32>(texel.rgb * in.colour.rgb + glint_light(in.uv, in.glint), 1.0);
}

// The card behind the inventory's character.
@vertex
fn backdrop_vertex(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    // The far end of its depth band, whatever its distance.
    let clip = view.clip_from_rel * vec4<f32>(rel_from_block(position), 1.0);
    return vec4<f32>(clip.xy, 0.0, clip.w);
}

@fragment
fn backdrop_fragment() -> @location(0) vec4<f32> {
    return vec4<f32>(0.012, 0.011, 0.010, 1.0);
}

// shadow.wgsl: black, as dark as the shadow sprite and the vertex alpha.
@fragment
fn shadow_fragment(in: EntityOut) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, textureSample(atlas, atlas_sampler, in.uv).a * in.colour.a);
}

// viewer/src/block_overlay.wgsl `fs_crack`, blended as the crumbling render
// type: source times destination, twice.
struct CrackOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) stage: f32,
}

@vertex
fn crack_vertex(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) stage: f32) -> CrackOut {
    var out: CrackOut;
    out.clip = view.clip_from_rel * vec4<f32>(rel_from_block(position), 1.0);
    out.uv = uv;
    out.stage = stage;
    return out;
}

@fragment
fn crack_fragment(in: CrackOut) -> @location(0) vec4<f32> {
    let tile = fract(in.uv);
    let colour = textureSampleLevel(cracks, atlas_sampler, vec2<f32>((in.stage + tile.x) / 10.0, tile.y), 0.0);
    if colour.a < 0.1 {
        discard;
    }
    return colour;
}

struct WeatherOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colour: vec4<f32>,
    @location(2) world_pos: vec3<f32>,
    @location(3) @interpolate(flat) snow: f32,
}

@vertex
fn weather_vertex(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) alpha: f32,
    @location(3) light: vec2<f32>,
    @location(4) snow: f32,
) -> WeatherOut {
    var out: WeatherOut;
    out.clip = view.clip_from_rel * vec4<f32>(rel_from_block(position), 1.0);
    out.uv = uv;
    out.colour = vec4<f32>(lightmap(light.x, light.y), alpha);
    out.world_pos = position;
    out.snow = snow;
    return out;
}

@fragment
fn weather_fragment(in: WeatherOut) -> @location(0) vec4<f32> {
    let cell = vec2<f32>((fract(in.uv.x) + in.snow) * 0.5, fract(in.uv.y));
    let colour = textureSampleLevel(weather_texture, atlas_sampler, cell, 0.0) * in.colour;
    if colour.a < 0.1 {
        discard;
    }
    return vec4<f32>(mix(colour.rgb, view.environment.fog.rgb, fog_value(in.world_pos)), colour.a);
}
"#;

fn pipelines(
    device: &RenderDevice,
    registry: &ExactPipelineRegistry,
    format: TextureFormat,
    samples: u32,
) -> [RenderPipeline; 14] {
    let shader = unsafe {
        device.create_shader_module(ShaderModuleDescriptor {
            label: Some("iw4l_minecraft_terrain"),
            source: ShaderSource::Wgsl(TERRAIN_WGSL.into()),
        })
    };
    let layout = registry.bind_group_layout(device, &layout());
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("iw4l_minecraft_terrain"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let attributes = [
        VertexAttribute {
            format: VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        VertexAttribute {
            format: VertexFormat::Float32x2,
            offset: 12,
            shader_location: 1,
        },
        VertexAttribute {
            format: VertexFormat::Unorm8x4,
            offset: 20,
            shader_location: 2,
        },
        VertexAttribute {
            format: VertexFormat::Unorm8x2,
            offset: 24,
            shader_location: 3,
        },
        VertexAttribute {
            format: VertexFormat::Unorm8x2,
            offset: 26,
            shader_location: 4,
        },
    ];
    let options = PipelineCompilationOptions {
        constants: &[],
        zero_initialize_workgroup_memory: false,
    };
    let buffers = [RawVertexBufferLayout {
        array_stride: MINECRAFT_VERTEX_BYTES,
        step_mode: VertexStepMode::Vertex,
        attributes: &attributes,
    }];
    let cloud_attributes = [
        VertexAttribute {
            format: VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: 12,
            shader_location: 1,
        },
    ];
    let cloud_buffers = [RawVertexBufferLayout {
        array_stride: CLOUD_VERTEX_BYTES,
        step_mode: VertexStepMode::Vertex,
        attributes: &cloud_attributes,
    }];
    let crack_attributes = [
        VertexAttribute {
            format: VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        VertexAttribute {
            format: VertexFormat::Float32x2,
            offset: 12,
            shader_location: 1,
        },
        VertexAttribute {
            format: VertexFormat::Float32,
            offset: 20,
            shader_location: 2,
        },
    ];
    let crack_buffers = [RawVertexBufferLayout {
        array_stride: CRACK_VERTEX_BYTES,
        step_mode: VertexStepMode::Vertex,
        attributes: &crack_attributes,
    }];
    let entity_attributes = [
        VertexAttribute {
            format: VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        VertexAttribute {
            format: VertexFormat::Float32x2,
            offset: 12,
            shader_location: 1,
        },
        VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: 20,
            shader_location: 2,
        },
        VertexAttribute {
            format: VertexFormat::Float32,
            offset: 36,
            shader_location: 3,
        },
        VertexAttribute {
            format: VertexFormat::Float32,
            offset: 40,
            shader_location: 4,
        },
    ];
    let entity_buffers = [RawVertexBufferLayout {
        array_stride: ENTITY_VERTEX_BYTES,
        step_mode: VertexStepMode::Vertex,
        attributes: &entity_attributes,
    }];
    let backdrop_attributes = [VertexAttribute {
        format: VertexFormat::Float32x3,
        offset: 0,
        shader_location: 0,
    }];
    let backdrop_buffers = [RawVertexBufferLayout {
        array_stride: 12,
        step_mode: VertexStepMode::Vertex,
        attributes: &backdrop_attributes,
    }];
    let weather_attributes = [
        VertexAttribute {
            format: VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        VertexAttribute {
            format: VertexFormat::Float32x2,
            offset: 12,
            shader_location: 1,
        },
        VertexAttribute {
            format: VertexFormat::Float32,
            offset: 20,
            shader_location: 2,
        },
        VertexAttribute {
            format: VertexFormat::Float32x2,
            offset: 24,
            shader_location: 3,
        },
        VertexAttribute {
            format: VertexFormat::Float32,
            offset: 32,
            shader_location: 4,
        },
    ];
    let weather_buffers = [RawVertexBufferLayout {
        array_stride: WEATHER_VERTEX_BYTES,
        step_mode: VertexStepMode::Vertex,
        attributes: &weather_attributes,
    }];
    let crumbling = BlendState {
        color: bevy::render::render_resource::BlendComponent {
            src_factor: bevy::render::render_resource::BlendFactor::Dst,
            dst_factor: bevy::render::render_resource::BlendFactor::Src,
            operation: bevy::render::render_resource::BlendOperation::Add,
        },
        alpha: bevy::render::render_resource::BlendComponent {
            src_factor: bevy::render::render_resource::BlendFactor::One,
            dst_factor: bevy::render::render_resource::BlendFactor::Zero,
            operation: bevy::render::render_resource::BlendOperation::Add,
        },
    };
    let make_biased = |vertex: &str, fragment: &str, buffers: &[RawVertexBufferLayout], depth_write: bool, compare: CompareFunction, blend: Option<BlendState>, cull: Option<bevy::render::render_resource::Face>, bias: DepthBiasState| {
        device.create_render_pipeline(&RawRenderPipelineDescriptor {
            label: Some("iw4l_minecraft_terrain"),
            layout: Some(&pipeline_layout),
            vertex: RawVertexState {
                module: &shader,
                entry_point: Some(vertex),
                buffers,
                compilation_options: options.clone(),
            },
            fragment: Some(RawFragmentState {
                module: &shader,
                entry_point: Some(fragment),
                targets: &[Some(ColorTargetState {
                    format,
                    blend,
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: options.clone(),
            }),
            primitive: PrimitiveState {
                cull_mode: cull,
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: SCENE_DEPTH_FORMAT,
                depth_write_enabled: Some(depth_write),
                depth_compare: Some(compare),
                stencil: default(),
                bias,
            }),
            multisample: MultisampleState {
                count: samples,
                ..default()
            },
            multiview_mask: None,
            cache: None,
        })
    };
    let make = |vertex: &str, fragment: &str, buffers: &[RawVertexBufferLayout], depth_write: bool, compare: CompareFunction, blend: Option<BlendState>, cull: Option<bevy::render::render_resource::Face>| {
        make_biased(vertex, fragment, buffers, depth_write, compare, blend, cull, DepthBiasState::default())
    };
    [
        make("sky_vertex", "sky_fragment", &[], false, CompareFunction::Equal, None, None),
        make("vertex", "opaque", &buffers, true, CompareFunction::GreaterEqual, None, None),
        make(
            "vertex",
            "translucent",
            &buffers,
            false,
            CompareFunction::GreaterEqual,
            Some(BlendState::ALPHA_BLENDING),
            Some(bevy::render::render_resource::Face::Back),
        ),
        make(
            "cloud_vertex",
            "cloud_fragment",
            &cloud_buffers,
            false,
            CompareFunction::GreaterEqual,
            Some(BlendState::ALPHA_BLENDING),
            None,
        ),
        make_biased(
            "crack_vertex",
            "crack_fragment",
            &crack_buffers,
            false,
            CompareFunction::GreaterEqual,
            Some(crumbling),
            Some(bevy::render::render_resource::Face::Back),
            DepthBiasState {
                constant: 10,
                slope_scale: 1.0,
                clamp: 0.0,
            },
        ),
        make("entity_vertex", "entity_fragment", &entity_buffers, true, CompareFunction::GreaterEqual, None, None),
        make(
            "entity_vertex",
            "entity_fragment",
            &entity_buffers,
            true,
            CompareFunction::GreaterEqual,
            None,
            Some(bevy::render::render_resource::Face::Back),
        ),
        make(
            "entity_vertex",
            "entity_translucent_fragment",
            &entity_buffers,
            true,
            CompareFunction::GreaterEqual,
            Some(BlendState::ALPHA_BLENDING),
            None,
        ),
        make(
            "entity_vertex",
            "shadow_fragment",
            &entity_buffers,
            false,
            CompareFunction::GreaterEqual,
            Some(BlendState::ALPHA_BLENDING),
            None,
        ),
        make("backdrop_vertex", "backdrop_fragment", &backdrop_buffers, true, CompareFunction::GreaterEqual, None, None),
        make("hand_vertex", "hand_fragment", &buffers, true, CompareFunction::GreaterEqual, None, None),
        make("vertex", "opaque", &buffers, true, CompareFunction::GreaterEqual, None, Some(bevy::render::render_resource::Face::Back)),
        make(
            "weather_vertex",
            "weather_fragment",
            &weather_buffers,
            false,
            CompareFunction::GreaterEqual,
            Some(BlendState::ALPHA_BLENDING),
            None,
        ),
        make("entity_vertex", "glow_fragment", &entity_buffers, false, CompareFunction::Always, None, None),
    ]
}

#[cfg(test)]
mod tests {
    #[test]
    fn terrain_shader_validates() {
        let module = naga::front::wgsl::parse_str(super::TERRAIN_WGSL).unwrap();
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
            .validate(&module)
            .unwrap();
    }

    #[test]
    fn darkness_dims_the_terrain_lightmap_before_its_clamp() {
        let source = super::TERRAIN_WGSL;
        let lightmap = &source[source.find("fn lightmap(").unwrap()..];
        let lightmap = &lightmap[..lightmap.find("\n}\n").unwrap()];
        let pulse = lightmap.find("color = color - vec3<f32>(environment.forward.w);").unwrap();
        let clamp = lightmap.find("color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));").unwrap();
        assert!(pulse < clamp);
        assert!(lightmap.ends_with("return mix(color, gamma, environment.cloud.w);"));
    }

    #[test]
    fn glowing_mobs_draw_through_walls_in_white_with_their_cover_after() {
        let module = naga::front::wgsl::parse_str(super::TERRAIN_WGSL).unwrap();
        assert!(module.entry_points.iter().any(|entry| entry.name == "glow_fragment"));
        assert_eq!(std::mem::size_of::<[(Vec<u8>, Vec<u32>); 6]>(), std::mem::size_of_val(&super::MinecraftWorldFrame::default().entity_meshes));
    }

    #[test]
    fn the_glint_uniform_ends_the_terrain_view_in_rust_and_wgsl() {
        let size = std::mem::size_of::<super::TerrainView>();
        assert_eq!(size % 16, 0);
        assert_eq!(std::mem::offset_of!(super::TerrainView, glint), size - 16);
        let module = naga::front::wgsl::parse_str(super::TERRAIN_WGSL).unwrap();
        let (_, view) = module.types.iter().find(|(_, ty)| ty.name.as_deref() == Some("TerrainView")).unwrap();
        let naga::TypeInner::Struct { members, span } = &view.inner else {
            panic!("TerrainView is not a struct");
        };
        assert_eq!(*span as usize, size);
        let glint = members.iter().find(|member| member.name.as_deref() == Some("glint")).unwrap();
        assert_eq!(glint.offset as usize, size - 16);
    }

    #[test]
    fn the_glint_flags_the_meshes_write_pick_the_item_and_armour_glints() {
        let module = naga::front::wgsl::parse_str(super::TERRAIN_WGSL).unwrap();
        let bindings: Vec<(u32, String)> = module
            .global_variables
            .iter()
            .filter_map(|(_, var)| Some((var.binding.as_ref()?.binding, var.name.clone()?)))
            .collect();
        assert!(bindings.contains(&(7, "armor_glint_texture".to_owned())));
        let source = super::TERRAIN_WGSL;
        assert!(source.contains("let armour = flag < 0.75;"));
        let flag = |byte: u8| f32::from(byte) / 255.0;
        assert!(flag(255) >= 0.75, "minecraft_terrain::mesh::GLINT_VERTEX");
        assert!(flag(128) > 0.0 && flag(128) < 0.75, "minecraft_terrain::mesh::ARMOR_GLINT_VERTEX");
    }

    #[test]
    fn the_missing_glint_stand_in_is_one_black_texel() {
        let blank = super::no_glint();
        assert_eq!((blank.width, blank.height), (1, 1));
        assert_eq!(blank.levels, [vec![0, 0, 0, 255]]);
    }

    #[test]
    fn crack_vertex_stride_is_the_frame_crack_vertex() {
        fn vertex_bytes<T>(_: fn(&super::MinecraftWorldFrame) -> &(Vec<T>, Vec<u32>)) -> u64 {
            std::mem::size_of::<T>() as u64
        }
        assert_eq!(super::CRACK_VERTEX_BYTES, vertex_bytes(|frame| &frame.cracks));
        assert_eq!(super::CRACK_VERTEX_BYTES, std::mem::size_of::<[f32; 6]>() as u64);
    }

    #[test]
    fn weather_vertex_stride_matches_the_frame() {
        fn vertex_bytes<T>(_: fn(&super::MinecraftWorldFrame) -> &(Vec<T>, Vec<u32>)) -> u64 {
            std::mem::size_of::<T>() as u64
        }
        assert_eq!(super::WEATHER_VERTEX_BYTES, vertex_bytes(|frame| &frame.weather));
        let module = naga::front::wgsl::parse_str(super::TERRAIN_WGSL).unwrap();
        let bindings: Vec<(u32, String)> = module
            .global_variables
            .iter()
            .filter_map(|(_, var)| Some((var.binding.as_ref()?.binding, var.name.clone()?)))
            .collect();
        assert!(bindings.contains(&(11, "weather_texture".to_owned())));
        let entry = |name: &str| module.entry_points.iter().any(|entry| entry.name == name);
        assert!(entry("weather_vertex") && entry("weather_fragment"));
    }

    #[test]
    fn atlas_patch_writes_each_mip_level_at_its_scaled_cell() {
        let atlas = super::MinecraftAtlasImage {
            width: 256,
            height: 256,
            levels: (0..5).map(|level| vec![0; ((256 >> level) * (256 >> level) * 4) as usize]).collect(),
        };
        let patch = super::MinecraftAtlasPatch {
            x: 128,
            y: 64,
            levels: (0..5).map(|level| (64 >> level, 64 >> level, vec![level as u8; ((64 >> level) * (64 >> level) * 4) as usize])).collect(),
        };
        let writes = super::patch_writes(&patch, &atlas);
        assert_eq!(
            writes.iter().map(|(level, origin, size, texels)| (*level, origin.x, origin.y, size.width, size.height, texels.len(), texels[0])).collect::<Vec<_>>(),
            [
                (0, 128, 64, 64, 64, 64 * 64 * 4, 0),
                (1, 64, 32, 32, 32, 32 * 32 * 4, 1),
                (2, 32, 16, 16, 16, 16 * 16 * 4, 2),
                (3, 16, 8, 8, 8, 8 * 8 * 4, 3),
                (4, 8, 4, 4, 4, 4 * 4 * 4, 4),
            ]
        );
        let outside = super::MinecraftAtlasPatch { x: 224, y: 0, levels: patch.levels.clone() };
        assert!(super::patch_writes(&outside, &atlas).is_empty());
        let short = super::MinecraftAtlasImage { width: 256, height: 256, levels: atlas.levels[..2].to_vec() };
        assert_eq!(super::patch_writes(&patch, &short).len(), 2);
    }

    fn wgsl_float(name: &str) -> f32 {
        let module = naga::front::wgsl::parse_str(super::TERRAIN_WGSL).unwrap();
        let (_, constant) = module.constants.iter().find(|(_, constant)| constant.name.as_deref() == Some(name)).unwrap();
        match module.global_expressions[constant.init] {
            naga::Expression::Literal(naga::Literal::F32(value)) => value,
            ref other => panic!("{name} is {other:?}"),
        }
    }

    #[test]
    fn only_a_dimension_without_a_skybox_shows_the_fog_colour_as_its_sky() {
        let none_above = wgsl_float("SKYBOX_NONE_ABOVE");
        assert!(1.0 < none_above && none_above < 2.0, "render_anim sends 0 for the overworld, 1 for the End and 2 for none");
        assert!(super::TERRAIN_WGSL.contains("if environment.ambient.w > SKYBOX_NONE_ABOVE {
        return vec4<f32>(environment.fog.rgb, 1.0);"));
    }

    #[test]
    fn end_portal_faces_are_told_apart_by_their_vertex_alpha() {
        let portal_below = wgsl_float("END_PORTAL_ALPHA_BELOW");
        let gateway_below = wgsl_float("END_GATEWAY_ALPHA_BELOW");
        let byte = |value: u8| f32::from(value) / 255.0;
        assert!(byte(31) < gateway_below, "minecraft_terrain::end_portal_look::END_GATEWAY_ALPHA");
        assert!(gateway_below < byte(63) && byte(63) < portal_below, "minecraft_terrain::end_portal_look::END_PORTAL_ALPHA");
        assert!(portal_below < byte(127), "minecraft_terrain::special_models::ENTITY_SHEET_ALPHA");
        let source = super::TERRAIN_WGSL;
        assert!(source.contains("let layers = select(15, 16, gateway);"));
        assert!(source.contains("out.screen = vec4<f32>((out.clip.xy + out.clip.w) * 0.5, out.clip.zw);"));
    }

    #[test]
    fn the_end_textures_and_portal_uniforms_match_in_rust_and_wgsl() {
        let module = naga::front::wgsl::parse_str(super::TERRAIN_WGSL).unwrap();
        let bindings: Vec<(u32, String)> = module
            .global_variables
            .iter()
            .filter_map(|(_, var)| Some((var.binding.as_ref()?.binding, var.name.clone()?)))
            .collect();
        for (binding, name) in [(8, "end_sky_texture"), (9, "end_portal_texture"), (10, "repeat_sampler")] {
            assert!(bindings.contains(&(binding, name.to_owned())), "{name}");
        }
        let (_, view) = module.types.iter().find(|(_, ty)| ty.name.as_deref() == Some("TerrainView")).unwrap();
        let naga::TypeInner::Struct { members, .. } = &view.inner else {
            panic!("TerrainView is not a struct");
        };
        let offset = |name: &str| members.iter().find(|member| member.name.as_deref() == Some(name)).unwrap().offset as usize;
        assert_eq!(offset("end_portal_colors"), std::mem::offset_of!(super::TerrainView, end_portal_colors));
        assert_eq!(offset("end_portal"), std::mem::offset_of!(super::TerrainView, end_portal));
    }

    fn end_sky_uv(ray: bevy::math::Vec3) -> bevy::math::Vec2 {
        let size = ray.abs();
        let local = if size.y >= size.x && size.y >= size.z {
            if ray.y > 0.0 { bevy::math::Vec3::new(ray.x, -ray.y, -ray.z) } else { ray }
        } else if size.z >= size.x {
            if ray.z > 0.0 { bevy::math::Vec3::new(ray.x, -ray.z, ray.y) } else { bevy::math::Vec3::new(ray.x, ray.z, -ray.y) }
        } else if ray.x > 0.0 {
            bevy::math::Vec3::new(ray.y, -ray.x, ray.z)
        } else {
            bevy::math::Vec3::new(-ray.y, ray.x, ray.z)
        };
        (bevy::math::Vec2::new(local.x, local.z) / -local.y + 1.0) * 8.0
    }

    #[test]
    fn the_end_sky_tiles_its_texture_sixteen_times_on_each_face_of_vanillas_cube() {
        let turns = [
            bevy::math::Mat3::IDENTITY,
            bevy::math::Mat3::from_rotation_x(std::f32::consts::FRAC_PI_2),
            bevy::math::Mat3::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            bevy::math::Mat3::from_rotation_x(std::f32::consts::PI),
            bevy::math::Mat3::from_rotation_z(std::f32::consts::FRAC_PI_2),
            bevy::math::Mat3::from_rotation_z(-std::f32::consts::FRAC_PI_2),
        ];
        let corners = [([-100.0, -100.0, -100.0], [0.0, 0.0]), ([-100.0, -100.0, 100.0], [0.0, 16.0]), ([100.0, -100.0, 100.0], [16.0, 16.0]), ([100.0, -100.0, -100.0], [16.0, 0.0])]
            .map(|(position, uv)| (bevy::math::Vec3::from_array(position), bevy::math::Vec2::from_array(uv)));
        let mixed = |s: f32, t: f32| {
            let [(p0, u0), (p1, u1), (p2, u2), (p3, u3)] = corners;
            let along = |a: bevy::math::Vec3, b: bevy::math::Vec3, c: bevy::math::Vec3, d: bevy::math::Vec3| a.lerp(d, s).lerp(b.lerp(c, s), t);
            let along_uv = |a: bevy::math::Vec2, b: bevy::math::Vec2, c: bevy::math::Vec2, d: bevy::math::Vec2| a.lerp(d, s).lerp(b.lerp(c, s), t);
            (along(p0, p1, p2, p3), along_uv(u0, u1, u2, u3))
        };
        for turn in turns {
            for s in [0.05, 0.3, 0.5, 0.77, 0.95] {
                for t in [0.05, 0.4, 0.5, 0.62, 0.95] {
                    let (point, uv) = mixed(s, t);
                    let got = end_sky_uv((turn * point).normalize());
                    assert!((got - uv).abs().max_element() < 1e-3, "{turn:?} at {s},{t}: {got} against {uv}");
                }
            }
        }
        let source = super::TERRAIN_WGSL;
        for line in [
            "local = select(ray, vec3<f32>(ray.x, -ray.y, -ray.z), ray.y > 0.0);",
            "local = select(vec3<f32>(ray.x, ray.z, -ray.y), vec3<f32>(ray.x, -ray.z, ray.y), ray.z > 0.0);",
            "local = select(vec3<f32>(-ray.y, ray.x, ray.z), vec3<f32>(ray.y, -ray.x, ray.z), ray.x > 0.0);",
            "return (local.xz / -local.y + 1.0) * 8.0;",
        ] {
            assert!(source.contains(line), "{line}");
        }
    }

    fn end_flash_uv(ray: bevy::math::Vec3, x_angle: f32, y_angle: f32) -> Option<bevy::math::Vec2> {
        let (x, y) = (x_angle.to_radians(), y_angle.to_radians());
        let direction = bevy::math::Vec3::new(-y.sin() * x.cos(), -x.sin(), y.cos() * x.cos());
        let facing = ray.dot(direction);
        if facing <= 0.0 {
            return None;
        }
        let axis_x = bevy::math::Vec3::new(-y.cos(), 0.0, -y.sin());
        let axis_z = bevy::math::Vec3::new(-x.sin() * y.sin(), x.cos(), x.sin() * y.cos());
        let projected = ray / facing;
        let quad = bevy::math::Vec2::new(projected.dot(axis_x), projected.dot(axis_z)) / wgsl_float("END_FLASH_HALF_SIZE");
        (quad.abs().max_element() <= 1.0).then(|| (quad + 1.0) * 0.5)
    }

    #[test]
    fn the_end_flash_quad_centres_on_its_direction_and_spans_point_six() {
        assert_eq!(wgsl_float("END_FLASH_HALF_SIZE"), 0.6);
        for (x_angle, y_angle) in [(-90.0f32, 0.0f32), (-31.479216, -105.22267), (-59.709076, -1.1438904), (10.0, 170.0), (-5.0, -180.0)] {
            let pose = bevy::math::Mat3::from_rotation_y((180.0 - y_angle).to_radians()) * bevy::math::Mat3::from_rotation_x((-90.0 - x_angle).to_radians());
            let corner = |lx: f32, lz: f32| (pose * bevy::math::Vec3::new(lx * 60.0, 100.0, lz * 60.0)).normalize();
            let centre = end_flash_uv(corner(0.0, 0.0), x_angle, y_angle).unwrap();
            assert!((centre - bevy::math::Vec2::splat(0.5)).abs().max_element() < 1e-4, "{x_angle},{y_angle}: {centre}");
            for lx in [-0.99, -0.6, 0.0, 0.35, 0.99] {
                for lz in [-0.99, -0.2, 0.0, 0.7, 0.99] {
                    let uv = end_flash_uv(corner(lx, lz), x_angle, y_angle).unwrap();
                    let expected = bevy::math::Vec2::new((lx + 1.0) / 2.0, (lz + 1.0) / 2.0);
                    assert!((uv - expected).abs().max_element() < 1e-4, "{x_angle},{y_angle} at {lx},{lz}: {uv} against {expected}");
                }
            }
            for (lx, lz) in [(1.02, 0.0), (0.0, -1.02), (-1.5, 1.5)] {
                assert_eq!(end_flash_uv(corner(lx, lz), x_angle, y_angle), None);
            }
            assert_eq!(end_flash_uv(-corner(0.0, 0.0), x_angle, y_angle), None);
        }
        let source = super::TERRAIN_WGSL;
        for line in [
            "let direction = vec3<f32>(-sin(y) * cos(x), -sin(x), cos(y) * cos(x));",
            "let axis_x = vec3<f32>(-cos(y), 0.0, -sin(y));",
            "let axis_z = vec3<f32>(-sin(x) * sin(y), cos(x), sin(x) * cos(y));",
            "let quad = vec2<f32>(dot(projected, axis_x), dot(projected, axis_z)) / END_FLASH_HALF_SIZE;",
            "return min(color + texel.rgb * texel.a * intensity * intensity, vec3<f32>(1.0));",
            "return vec4<f32>(end_flash(ray, end_sky(ray)), 1.0);",
        ] {
            assert!(source.contains(line), "{line}");
        }
    }

    #[test]
    fn the_celestial_atlas_has_ten_slots() {
        assert_eq!(wgsl_float("CELESTIAL_SLOTS"), 10.0);
        assert_eq!(wgsl_float("END_FLASH_SLOT"), 9.0);
        let source = super::TERRAIN_WGSL;
        assert!(source.contains("let atlas_uv = vec2<f32>((slot + uv.x) / CELESTIAL_SLOTS, uv.y);"));
        assert!(source.contains("vec2<f32>((END_FLASH_SLOT + uv.x) / CELESTIAL_SLOTS, uv.y)"));
    }

    #[test]
    fn the_end_sky_is_drawn_for_the_end_skybox_only_and_shaded_like_vanilla() {
        let end_above = wgsl_float("SKYBOX_END_ABOVE");
        assert!(0.0 < end_above && end_above < 1.0, "render_anim sends 0 for the overworld and 1 for the End");
        assert!((wgsl_float("END_SKY_SHADE") - 0x28 as f32 / 255.0).abs() < 1e-6, "SkyRenderer.buildEndSky colour -14145496");
        assert!(super::TERRAIN_WGSL.contains("return mix(view.environment.fog.rgb, texel.rgb * END_SKY_SHADE, texel.a);"));
    }

    #[test]
    fn clouds_show_only_while_the_cloud_colour_has_alpha() {
        let mut environment = [[0.0; 4]; 16];
        assert!(!super::clouds_shown(&environment));
        environment[11][1] = 1.0 / 255.0;
        assert!(super::clouds_shown(&environment));
        environment[11][1] = 0.8;
        assert!(super::clouds_shown(&environment));
    }
}
