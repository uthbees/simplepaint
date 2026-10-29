use crate::app_state::canvas::layer::Layer;
use crate::gpu::CanvasGpu;
use crate::utils::dimensions::DimensionsPxNonzero;
use std::collections::HashMap;

pub struct Layers {
    sampler: wgpu::Sampler,
    composition_bind_group_layout: wgpu::BindGroupLayout,
    layer_map: HashMap<i32, LayerResources>,
    pub dirty: bool,
}

pub struct LayerResources {
    pub texture: wgpu::Texture,
    pub composition_bind_group: wgpu::BindGroup,
}

impl Layers {
    pub fn new(device: &wgpu::Device) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("layer_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let composition_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("layer_composition_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        Self {
            sampler,
            composition_bind_group_layout,
            layer_map: HashMap::new(),
            dirty: false,
        }
    }

    pub fn composition_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.composition_bind_group_layout
    }

    pub fn layer_map(&self) -> &HashMap<i32, LayerResources> {
        &self.layer_map
    }

    pub fn add_layer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        canvas_dimensions: DimensionsPxNonzero,
        layer_id: i32,
        initial_pixels: &[u8],
    ) {
        let new_layer = self.create_layer_resources(device, canvas_dimensions);
        CanvasGpu::write_texture(queue, &new_layer.texture, canvas_dimensions, initial_pixels);

        self.layer_map.insert(layer_id, new_layer);
        self.dirty = true;
    }

    /// Upload a layer's full CPU buffer to its raw GPU texture.
    pub fn upload_layer(
        &mut self,
        queue: &wgpu::Queue,
        cpu_layer: &Layer,
        canvas_dimensions: DimensionsPxNonzero,
    ) {
        if !self.layer_map.contains_key(cpu_layer.id()) {
            eprintln!(
                "Error: Tried to upload data to an unregistered layer ({}).",
                cpu_layer.id()
            );
            return;
        }
        let gpu_layer = &self
            .layer_map
            .get(cpu_layer.id())
            .expect("Should have validated that map contains layer");

        CanvasGpu::write_texture(
            queue,
            &gpu_layer.texture,
            canvas_dimensions,
            cpu_layer.buffer_as_bytes(),
        );
        self.dirty = true;
    }

    fn create_layer_resources(
        &self,
        device: &wgpu::Device,
        canvas_dimensions: DimensionsPxNonzero,
    ) -> LayerResources {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("canvas_layer_texture"),
            size: wgpu::Extent3d {
                width: canvas_dimensions.width(),
                height: canvas_dimensions.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let composition_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("layer_composition_bind_group"),
            layout: &self.composition_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        LayerResources {
            texture,
            composition_bind_group,
        }
    }
}
