use crate::app_state::AppState;
use crate::app_state::canvas::layer::{Layer, TILE_SIZE};
use crate::utils::dimensions::DimensionsPxNonzero;
use egui_wgpu::RenderState;
use std::collections::HashMap;

pub struct Layers {
    render_state: RenderState,
    sampler: wgpu::Sampler,
    composition_bind_group_layout: wgpu::BindGroupLayout,
    layer_map: HashMap<i32, LayerResources>,
    // Whether there have been any updates to layer contents, including layer resizing. Does not
    // account for layers being created or deleted, as that's managed by the layer visibility code.
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
                layer.clear_dirty();
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
        self.write_texture(
            new_layer.texture.as_image_copy(),
            canvas_dimensions,
            initial_pixels,
        );

        self.layer_map.insert(layer_id, new_layer);
    }

    pub fn delete_layer(&mut self, layer_id: i32) {
        self.layer_map.remove(&layer_id);
    }

    pub fn resize_layer(
        &mut self,
        layer_id: i32,
        new_dimensions: DimensionsPxNonzero,
        new_buffer: &[u8],
    ) {
        // WGPU textures cannot be resized, only destroyed and re-created.
        // Potential optimizations:
        // - Keep the existing texture for small size reductions
        // - Copy from the old layer to the new one to avoid re-computing the entire layer buffer
        self.delete_layer(layer_id);
        self.add_layer(new_dimensions, layer_id, new_buffer);
        self.dirty = true;
    }

    /// Upload the dirty tiles of a CPU layer to its raw GPU texture.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
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

        let canvas_w = canvas_dimensions.width().cast_signed();
        let canvas_h = canvas_dimensions.height().cast_signed();
        let tile_size = TILE_SIZE.cast_signed();

        for coord in cpu_layer.dirty_tiles() {
            let tile_origin_x = coord.x * tile_size;
            let tile_origin_y = coord.y * tile_size;

            // Check if the tile is within the canvas boundary.
            if tile_origin_x >= canvas_w
                || tile_origin_y >= canvas_h
                || tile_origin_x + tile_size - 1 < 0
                || tile_origin_y + tile_size - 1 < 0
            {
                continue;
            }

            let Some(tile_pixmap) = cpu_layer.tiles().get(coord) else {
                continue;
            };

            // Clamp tile upload rectangle to canvas bounds.
            let x_offset = 0i32.max(tile_origin_x) as u32;
            let y_offset = 0i32.max(tile_origin_y) as u32;

            let width = ((tile_origin_x + tile_size).min(canvas_w) - x_offset as i32) as u32;
            let height = ((tile_origin_y + tile_size).min(canvas_h) - y_offset as i32) as u32;

            if width == 0 || height == 0 {
                continue;
            }

            // Offset within the tile pixmap for if the tile originates outside the canvas.
            let tile_local_offset_x = (x_offset as i32 - tile_origin_x) as usize;
            let tile_local_offset_y = (y_offset as i32 - tile_origin_y) as usize;

            let texture_copy_info = wgpu::TexelCopyTextureInfo {
                texture: &gpu_layer.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: x_offset,
                    y: y_offset,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            };

            if width == TILE_SIZE
                && height == TILE_SIZE
                && tile_local_offset_x == 0
                && tile_local_offset_y == 0
            {
                // Upload the full tile directly.
                self.write_texture(
                    texture_copy_info,
                    DimensionsPxNonzero::new(width, height),
                    tile_pixmap.data(),
                );
            } else {
                // Sub-rectangle upload (the edge tiles are clipped by the canvas boundaries).
                let mut clipped_bytes = Vec::with_capacity((width * height * 4) as usize);
                for row in 0..height as usize {
                    let src_y = tile_local_offset_y + row;
                    let src_start = src_y * (TILE_SIZE as usize * 4) + tile_local_offset_x * 4;
                    let src_end = src_start + width as usize * 4;
                    clipped_bytes.extend_from_slice(&tile_pixmap.data()[src_start..src_end]);
                }

                self.write_texture(
                    texture_copy_info,
                    DimensionsPxNonzero::new(width, height),
                    &clipped_bytes,
                );
            }
        }

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
        texture_copy_info: wgpu::TexelCopyTextureInfo,
        write_dimensions: DimensionsPxNonzero,
        bytes: &[u8],
    ) {
        let write_width = write_dimensions.width();
        let write_height = write_dimensions.height();

        self.render_state.queue.write_texture(
            texture_copy_info,
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(write_width * 4),
                rows_per_image: Some(write_height),
            },
            wgpu::Extent3d {
                width: write_width,
                height: write_height,
                depth_or_array_layers: 1,
            },
        );
    }
}
