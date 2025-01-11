use std::mem;

use cgmath::{Matrix4, Ortho, SquareMatrix};
use crevice::std140::AsStd140;
use glyphon::Color;
use wgpu::util::DeviceExt;

use crate::srgb;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[derive(Debug, Clone, Copy)]
pub struct Quad {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
struct Vertex {
    x: f32,
    y: f32,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(crevice::std140::AsStd140, Debug, Copy, Clone)]
struct Uniform {
    matrix: Matrix4<f32>,
}

impl Default for Uniform {
    fn default() -> Self {
        Self {
            matrix: Matrix4::identity(),
        }
    }
}

impl Uniform {
    fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Quad fragment bind group"),
        })
    }

    pub fn from_size(width: f32, height: f32) -> Self {
        let ortho = Ortho {
            left: 0.0,
            right: width,
            top: 0.0,
            bottom: height,
            near: 0.0,
            far: 1.0,
        };
        Self {
            matrix: OPENGL_TO_WGPU_MATRIX * Matrix4::from(ortho),
        }
    }
}

pub struct QuadRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_buffer_len: u64,
    index_buffer: wgpu::Buffer,
    index_buffer_len: u64,
    num_indices: u32,
    uniform: Uniform,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl QuadRenderer {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Quad render pipeline layout"),
                bind_group_layouts: &[&Uniform::get_bind_group_layout(device)],
                push_constant_ranges: &[],
            });

        let vertex = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Quad vertex"),
            source: wgpu::ShaderSource::Glsl {
                shader: include_str!("../../../../shaders/quad.vert").into(),
                stage: wgpu::naga::ShaderStage::Vertex,
                defines: Default::default(),
            },
        });

        let fragment = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Quad fragment"),
            source: wgpu::ShaderSource::Glsl {
                shader: include_str!("../../../../shaders/quad.frag").into(),
                stage: wgpu::naga::ShaderStage::Fragment,
                defines: Default::default(),
            },
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Quad Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: Some("main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: Some("main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let vertex_buffer_len = 128;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Quad Vertex Buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            size: vertex_buffer_len * mem::size_of::<Vertex>() as u64,
            mapped_at_creation: false,
        });

        let index_buffer_len = 128;
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Quad Index Buffer"),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            size: index_buffer_len * mem::size_of::<u32>() as u64,
            mapped_at_creation: false,
        });

        let uniform = Uniform::from_size(config.width as f32, config.height as f32);

        let value_std140 = Uniform::default().as_std140();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex uniform buffer"),
            contents: value_std140.as_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group_layout = Uniform::get_bind_group_layout(device);
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Vertex uniform bind group"),
        });

        Self {
            pipeline,
            vertex_buffer,
            vertex_buffer_len,
            index_buffer,
            index_buffer_len,
            num_indices: 0,
            uniform,
            uniform_buffer,
            uniform_bind_group,
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    pub fn push_quad(&mut self, quad: Quad, color: Color) {
        self.indices.push(self.vertices.len() as u32);
        self.indices.push(self.vertices.len() as u32 + 1);
        self.indices.push(self.vertices.len() as u32 + 2);
        self.indices.push(self.vertices.len() as u32 + 2);
        self.indices.push(self.vertices.len() as u32 + 1);
        self.indices.push(self.vertices.len() as u32 + 3);
        let r = srgb::srgb_to_linear(color.r() as f32 / 255.0);
        let b = srgb::srgb_to_linear(color.b() as f32 / 255.0);
        let g = srgb::srgb_to_linear(color.g() as f32 / 255.0);
        let a = color.a() as f32 / 255.0;
        self.vertices.push(Vertex {
            x: quad.x,
            y: quad.y,
            r,
            g,
            b,
            a,
        });
        self.vertices.push(Vertex {
            x: quad.x + quad.width,
            y: quad.y,
            r,
            g,
            b,
            a,
        });
        self.vertices.push(Vertex {
            x: quad.x,
            y: quad.y + quad.height,
            r,
            g,
            b,
            a,
        });
        self.vertices.push(Vertex {
            x: quad.x + quad.width,
            y: quad.y + quad.height,
            r,
            g,
            b,
            a,
        });
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.uniform = Uniform::from_size(width, height);
    }

    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let value_std140 = self.uniform.as_std140();
        queue.write_buffer(&self.uniform_buffer, 0, value_std140.as_bytes());

        if self.vertex_buffer_len < self.vertices.len() as u64 {
            while self.vertex_buffer_len < self.vertices.len() as u64 {
                self.vertex_buffer_len *= 2;
            }
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Quad Vertex Buffer"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                size: self.vertex_buffer_len * mem::size_of::<Vertex>() as u64,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));

        if self.index_buffer_len < self.indices.len() as u64 {
            while self.index_buffer_len < self.indices.len() as u64 {
                self.index_buffer_len *= 2;
            }
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Quad Index Buffer"),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                size: self.index_buffer_len * mem::size_of::<u32>() as u64,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));

        self.num_indices = self.indices.len() as u32;
    }

    pub fn render<'rpass>(&'rpass mut self, rpass: &mut wgpu::RenderPass<'rpass>) {
        if self.num_indices > 0 {
            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &self.uniform_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..self.num_indices, 0, 0..1);
        }
    }
}
