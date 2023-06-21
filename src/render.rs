use wgpu::util::DeviceExt;

use crate::texture::Texture;
// use image::{ImageBuffer, Rgba};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    tex_id: u32,
}
impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Uint32,
    ];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

const VERTICES: &[Vertex] = &[
    // bg
    Vertex { tex_id: 0, position: [1.0, 0.0, 0.0], tex_coords: [1.0, 1.0], },
    Vertex { tex_id: 0, position: [1.0, 1.0, 0.0], tex_coords: [1.0, 0.0], },
    Vertex { tex_id: 0, position: [0.0, 0.0, 0.0], tex_coords: [0.0, 1.0], },
    Vertex { tex_id: 0, position: [0.0, 1.0, 0.0], tex_coords: [0.0, 0.0], },

    // fg
    Vertex { tex_id: 1, position: [1.0, 0.0, 0.0], tex_coords: [1.0, 1.0], },
    Vertex { tex_id: 1, position: [1.0, 1.0, 0.0], tex_coords: [1.0, 0.0], },
    Vertex { tex_id: 1, position: [0.0, 0.0, 0.0], tex_coords: [0.0, 1.0], },
    Vertex { tex_id: 1, position: [0.0, 1.0, 0.0], tex_coords: [0.0, 0.0], },
];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    position: [f32; 2], // bottom left
    size: [f32; 2],
}

impl Instance {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        3 => Float32x2,
        4 => Float32x2,
    ];
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Instance>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}


pub struct Renderer<'a> {
    device: wgpu::Device,
    queue: wgpu::Queue,

    texture: wgpu::Texture,
    texture_desc: wgpu::TextureDescriptor<'a>,
    texture_view: wgpu::TextureView,

    vertex_buffer: wgpu::Buffer,

    // output_buffer_desc: wgpu::BufferDescriptor<'a>,
    output_buffer: wgpu::Buffer,

    render_pipeline: wgpu::RenderPipeline,

    diffuse0_bind_group: wgpu::BindGroup,
    diffuse0_texture: Texture,
    diffuse1_bind_group: wgpu::BindGroup,
    diffuse1_texture: Texture,

    instance_buffer: wgpu::Buffer,
    instance_len: usize,
}

impl<'a> Renderer<'a> {
    pub async fn new(
        size: [u16; 2],
        monitors: &[[u16; 4]],
    ) -> Renderer<'a> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        }).await.unwrap();


        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default(), None).await.unwrap();

        let texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: size[0].into(),
                height: size[1].into(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: Some("texture"),
            view_formats: &[],
        };

        let texture = device.create_texture(&texture_desc);
        let texture_view = texture.create_view(&Default::default());

        // we need to store this for later
        let u32_size = std::mem::size_of::<u32>() as u32;

        let output_buffer_size = (u32_size * size[0] as u32 * size[1] as u32) as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: Some("output_buffer"),
            mapped_at_creation: false,
        };
        let output_buffer = device.create_buffer(&output_buffer_desc);

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: std::mem::size_of::<Vertex>() as wgpu::BufferAddress * 4 * 2,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let diffuse0_bytes = include_bytes!("happy-tree.png");
        let diffuse0_texture = Texture::from_bytes(&device, &queue, diffuse0_bytes, "happy-tree.png").unwrap();

        let diffuse0_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse0_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse0_texture.sampler),
                    },
                ],
                label: Some("diffuse_bind_group"),
            }
        );
        let diffuse1_bytes = include_bytes!("favicon.png");
        let diffuse1_texture = Texture::from_bytes(&device, &queue, diffuse1_bytes, "favicon.png").unwrap();

        let diffuse1_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse1_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse1_texture.sampler),
                    },
                ],
                label: Some("diffuse_bind_group"),
            }
        );

        let instances = monitors.iter().map(|m| {
            Instance {
                position: [
                    m[0] as f32 / size[0] as f32 * 2.0 - 1.0,
                    1.0 - (m[1] + m[3]) as f32 / size[1] as f32 * 2.0,
                ],
                size: [
                    m[2] as f32 / size[0] as f32 * 2.0,
                    m[3] as f32 / size[1] as f32 * 2.0,
                ],
            }
        }).collect::<Vec<_>>();
        println!("{:?}", instances);

        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );


        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &texture_bind_group_layout,
                &texture_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), Instance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_desc.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,

                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        println!("render pipeline created");

        Self {
            device,
            queue,
            texture,
            texture_desc,
            texture_view,
            vertex_buffer,
            // output_buffer_desc,
            output_buffer,

            render_pipeline,

            diffuse0_bind_group,
            diffuse0_texture,
            diffuse1_bind_group,
            diffuse1_texture,

            instance_buffer,
            instance_len: instances.len(),
        }
    }

    pub async fn render<T>( &mut self,
        t: std::time::Duration,
        callback: impl FnOnce(wgpu::BufferView) -> T
    ) -> Result<T, wgpu::SurfaceError> {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("encoder"),
        });

        let u32_size = std::mem::size_of::<u32>() as u32;
        {
            let render_pass_desc = wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: true,
                        },
                    })
                ],
                depth_stencil_attachment: None,
            };

            {
                let mut render_pass = encoder.begin_render_pass(&render_pass_desc);

                let mut vbuf: [Vertex; 8] = unsafe { std::mem::uninitialized() };
                vbuf.copy_from_slice(&VERTICES);
                for i in 4..8 {
                    vbuf[i].position[1] += t.as_secs_f32().cos() * 0.1;
                }

                self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vbuf));


                // self.queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&[
                //     Instance { position: [-1.0, -1.0], size: [1.0, 2.0], },
                //     Instance { position: [-0.0, -1.0], size: [1.0, 2.0], },
                // ]));

                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.diffuse0_bind_group, &[]);
                render_pass.set_bind_group(1, &self.diffuse1_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
                render_pass.draw(0..4, 0..self.instance_len as u32);
                render_pass.draw(4..8, 0..self.instance_len as u32);
            }

            encoder.copy_texture_to_buffer(
                wgpu::ImageCopyTexture {
                    aspect: wgpu::TextureAspect::All,
                    texture: &self.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                wgpu::ImageCopyBuffer {
                    buffer: &self.output_buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(u32_size * self.get_width()),
                        rows_per_image: Some(self.get_height()),
                    },
                },
                self.texture_desc.size,
            );


            self.queue.submit(Some(encoder.finish()));
        }

        let buffer_slice = self.output_buffer.slice(..);

        // NOTE: We have to create the mapping THEN device.poll() before await
        // the future. Otherwise the application will freeze.
        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        self.device.poll(wgpu::Maintain::Wait);
        rx.receive().await.unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let ret = callback(data);

        self.output_buffer.unmap();

        Ok(ret)
    }

    pub fn get_width(&self) -> u32 {
        self.texture_desc.size.width
    }
    pub fn get_height(&self) -> u32 {
        self.texture_desc.size.height
    }
}
