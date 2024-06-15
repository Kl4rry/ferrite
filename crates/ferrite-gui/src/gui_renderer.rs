use glyphon::{
    Attrs, Buffer, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea,
    TextAtlas, TextBounds, TextRenderer,
};
use wgpu::RenderPass;

pub struct GuiRenderer {
    font_system: FontSystem,
    cache: SwashCache,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    width: f32,
    height: f32,
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
        let cache = SwashCache::new();
        let mut atlas = TextAtlas::new(device, queue, surface_format);
        let text_renderer = TextRenderer::new(
            &mut atlas,
            device,
            wgpu::MultisampleState {
                count: 1,
                ..Default::default()
            },
            None,
        );

        Self {
            font_system,
            cache,
            atlas,
            text_renderer,
            width,
            height,
        }
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }

    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, text: String) {
        let mut text_areas = Vec::new();
        let mut buffer = Buffer::new(&mut self.font_system, Metrics::new(15.0, 19.0));
        buffer.set_size(&mut self.font_system, self.width, self.height);
        buffer.set_wrap(&mut self.font_system, glyphon::Wrap::None);
        buffer.set_text(
            &mut self.font_system,
            &text,
            Attrs::new().family(Family::Monospace),
            Shaping::Advanced,
        );
        buffer.shape_until(&mut self.font_system, 1000);

        text_areas.push(TextArea {
            buffer: &buffer,
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
        });

        self.text_renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                Resolution {
                    width: self.width as u32,
                    height: self.height as u32,
                },
                text_areas,
                &mut self.cache,
            )
            .unwrap();
    }

    pub fn render<'rpass>(&'rpass mut self, rpass: &mut RenderPass<'rpass>) {
        self.text_renderer.render(&self.atlas, rpass).unwrap();
    }
}
