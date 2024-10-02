use glyphon::{
    cosmic_text::Scroll, Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution,
    Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use wgpu::RenderPass;

pub struct GuiRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    width: f32,
    height: f32,
    // buffer
    buffer: Buffer,
    text: String,
}

impl GuiRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        width: f32,
        height: f32,
    ) -> Self {
        let mut font_system = FontSystem::new();
        font_system.db_mut().set_monospace_family("9x15");
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let mut atlas = TextAtlas::new(device, queue, &cache, surface_format);
        let text_renderer = TextRenderer::new(
            &mut atlas,
            device,
            wgpu::MultisampleState {
                count: 1,
                ..Default::default()
            },
            None,
        );

        let viewport = Viewport::new(device, &cache);

        let mut buffer = Buffer::new(&mut font_system, Metrics::new(15.0, 19.0));
        buffer.set_monospace_width(&mut font_system, Some(2.0));
        buffer.set_wrap(&mut font_system, glyphon::Wrap::None);

        Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            width,
            height,
            buffer,
            text: String::new(),
        }
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        text: String,
        scroll: usize,
    ) {
        let mut text_areas = Vec::new();
        self.buffer
            .set_size(&mut self.font_system, Some(self.width), Some(self.height));
        let scroll = Scroll::new(scroll, 0.0, 0.0);
        self.buffer.set_scroll(scroll);
        self.buffer.shape_until_scroll(&mut self.font_system, true);

        if self.text != text {
            self.text = text;
            self.buffer.set_text(
                &mut self.font_system,
                &self.text,
                Attrs::new().family(Family::Monospace),
                Shaping::Advanced,
            );
        }

        self.viewport.update(
            queue,
            Resolution {
                width: self.width as u32,
                height: self.height as u32,
            },
        );

        text_areas.push(TextArea {
            buffer: &self.buffer,
            left: 0.0,
            top: 0.0,
            scale: 1.0,
            bounds: TextBounds {
                left: 0,
                top: 0,
                right: self.width as i32,
                bottom: self.height as i32,
            },
            default_color: Color::rgb(205, 214, 244),
            custom_glyphs: &[],
        });

        self.text_renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .unwrap();
    }

    pub fn render<'rpass>(&'rpass mut self, rpass: &mut RenderPass<'rpass>) {
        self.text_renderer
            .render(&self.atlas, &self.viewport, rpass)
            .unwrap();
    }
}
