use std::ops::Range;

use geometry_renderer::{Geometry, GeometryRenderer};
use glyphon::{
    Cache, FontSystem, Resolution, SwashCache, TextArea, TextAtlas, TextRenderer, Viewport,
};

pub mod geometry_renderer;

pub struct Layer<'a> {
    pub bundles: Vec<Bundle<'a>>,
}

pub struct Bundle<'a> {
    pub top_geometry: Geometry,
    pub bottom_geometry: Geometry,
    pub text_area: TextArea<'a>,
}

pub struct LayerRenderer {
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    top_geometry_index_range: Range<u32>,
    bottom_geometry_index_range: Range<u32>,
}

pub struct Renderer {
    pub font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    geometry_renderer: GeometryRenderer,
    layer_renderers: Vec<LayerRenderer>,
    cache: Cache,
    width: f32,
    height: f32,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        width: f32,
        height: f32,
    ) -> Self {
        let mut font_system = FontSystem::new();
        font_system
            .db_mut()
            .load_font_data(include_bytes!("../../../fonts/FiraCode-Regular.ttf").to_vec());
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);

        let viewport = Viewport::new(device, &cache);
        let geometry_renderer = GeometryRenderer::new(device, config);

        Self {
            font_system,
            swash_cache,
            viewport,
            geometry_renderer,
            cache,
            layer_renderers: Vec::new(),
            width,
            height,
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
        layers: Vec<Layer>,
    ) {
        self.geometry_renderer.clear();
        self.viewport.update(
            queue,
            Resolution {
                width: self.width as u32,
                height: self.height as u32,
            },
        );

        while self.layer_renderers.len() < layers.len() {
            let mut atlas = TextAtlas::new(device, queue, &self.cache, config.format);
            let text_renderer = TextRenderer::new(
                &mut atlas,
                device,
                wgpu::MultisampleState {
                    count: 1,
                    ..Default::default()
                },
                None,
            );
            self.layer_renderers.push(LayerRenderer {
                text_renderer,
                atlas,
                top_geometry_index_range: 0..0,
                bottom_geometry_index_range: 0..0,
            });
        }

        for (renderer, layer) in self.layer_renderers.iter_mut().zip(layers) {
            {
                let start = self.geometry_renderer.num_indices();
                for bundle in &layer.bundles {
                    bundle
                        .bottom_geometry
                        .tessellate(&mut self.geometry_renderer);
                }
                let end = self.geometry_renderer.num_indices();
                renderer.bottom_geometry_index_range = start..end;
            }
            {
                let start = self.geometry_renderer.num_indices();
                for bundle in &layer.bundles {
                    bundle.top_geometry.tessellate(&mut self.geometry_renderer);
                }
                let end = self.geometry_renderer.num_indices();
                renderer.top_geometry_index_range = start..end;
            }
            let text_areas: Vec<_> = layer
                .bundles
                .into_iter()
                .map(|bundle| bundle.text_area)
                .collect();
            renderer
                .text_renderer
                .prepare(
                    device,
                    queue,
                    &mut self.font_system,
                    &mut renderer.atlas,
                    &self.viewport,
                    text_areas,
                    &mut self.swash_cache,
                )
                .unwrap();
        }
        self.geometry_renderer.prepare(device, queue);
    }

    pub fn render<'rpass>(&'rpass self, rpass: &mut wgpu::RenderPass<'rpass>) {
        let Self {
            geometry_renderer,
            layer_renderers,
            ..
        } = self;
        for layer in layer_renderers {
            geometry_renderer.render(rpass, layer.bottom_geometry_index_range.clone());
            layer
                .text_renderer
                .render(&layer.atlas, &self.viewport, rpass)
                .unwrap();
            geometry_renderer.render(rpass, layer.top_geometry_index_range.clone());
        }
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.geometry_renderer.resize(width, height);
    }
}
