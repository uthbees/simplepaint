use crate::app_state::AppState;
use crate::app_state::canvas::layer::Layer;
use crate::utils::dimensions::DimensionsPxNonzero;
use egui_wgpu::RenderState;
use std::collections::HashMap;

pub struct Layers {
    render_state: RenderState,
    sampler: wgpu::Sampler,
    composition_bind_group_layout: wgpu::BindGroupLayout,
    layer_map: HashMap<i32, LayerResources>,
    // Whether there have been any updates to layer contents. Does not account for layers being
    // created or deleted, as that's managed by the layer visibility code.
    pub dirty: bool,
}

pub struct LayerResources {
    pub texture: wgpu::Texture,
    pub texture_id: egui::TextureId,
    pub composition_bind_group: wgpu::BindGroup,
}

impl Layers {
    pub fn new(render_state: RenderState) -> Self {
        let device = &render_state.device;

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
            render_state,
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

    /// Upload any changes to the CPU canvas to the GPU, including layer additions and deletions.
    pub fn sync_cpu_canvas_updates(&mut self, app_state: &mut AppState) {
        let canvas_dimensions = app_state.canvas.dimensions();
        app_state.canvas.layers_iter_mut().for_each(|layer| {
            if layer.dirty {
                self.upload_layer(layer, canvas_dimensions);
                layer.dirty = false;
            }
        });
    }

    pub fn add_layer(
        &mut self,
        canvas_dimensions: DimensionsPxNonzero,
        layer_id: i32,
        initial_pixels: &[u8],
    ) {
        let new_layer = self.create_layer_resources(canvas_dimensions);
        self.write_texture(&new_layer.texture, canvas_dimensions, initial_pixels);

        self.layer_map.insert(layer_id, new_layer);
    }

    pub fn delete_layer(&mut self, layer_id: i32) {
        self.layer_map.remove(&layer_id);
    }

    /// Upload a layer's full CPU buffer to its raw GPU texture.
    fn upload_layer(&mut self, cpu_layer: &Layer, canvas_dimensions: DimensionsPxNonzero) {
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

        self.write_texture(
            &gpu_layer.texture,
            canvas_dimensions,
            cpu_layer.buffer_as_bytes(),
        );
        self.dirty = true;
    }

    fn create_layer_resources(&self, canvas_dimensions: DimensionsPxNonzero) -> LayerResources {
        let device = &self.render_state.device;

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

        let texture_id = self.render_state.renderer.write().register_native_texture(
            device,
            &texture_view,
            wgpu::FilterMode::Nearest,
        );

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
            texture_id,
            composition_bind_group,
        }
    }

    /// Wrapper function for `queue.write_texture`.
    fn write_texture(
        &self,
        texture: &wgpu::Texture,
        texture_dimensions: DimensionsPxNonzero,
        bytes: &[u8],
    ) {
        let width = texture_dimensions.width();
        let height = texture_dimensions.height();

        self.render_state.queue.write_texture(
            texture.as_image_copy(),
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }
}
