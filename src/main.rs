use std::io::Cursor;
use wgpu::*;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

fn main() {
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    futures::executor::block_on(run(event_loop, window));
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    // Setup WGPU
    let surface = Surface::create(&window);
    let adapter = Adapter::request(
        &RequestAdapterOptions {
            power_preference: PowerPreference::Default,
            compatible_surface: Some(&surface),
        },
        BackendBit::PRIMARY,
    )
    .await
    .unwrap();
    let (device, queue) = adapter
        .request_device(&DeviceDescriptor {
            extensions: Extensions {
                anisotropic_filtering: false,
            },
            limits: Limits::default(),
        })
        .await;
    let mut screen_size = window.inner_size();

    // Setup textures
    let mut texture_descriptor = TextureDescriptor {
        label: None,
        size: Extent3d {
            width: screen_size.width,
            height: screen_size.height,
            depth: 1,
        },
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Bgra8UnormSrgb,
        usage: TextureUsage::STORAGE
            | TextureUsage::COPY_DST
            | TextureUsage::COPY_SRC
            | TextureUsage::OUTPUT_ATTACHMENT
            | TextureUsage::SAMPLED, // TODO: Unsure what exactly is needed
    };
    let mut default_texture = device
        .create_texture(&texture_descriptor)
        .create_default_view();
    let mut pink_simple_texture = device
        .create_texture(&texture_descriptor)
        .create_default_view();

    // Setup the swapchain
    let mut swap_chain_descriptor = SwapChainDescriptor {
        usage: TextureUsage::OUTPUT_ATTACHMENT,
        format: TextureFormat::Bgra8UnormSrgb,
        width: screen_size.width,
        height: screen_size.height,
        present_mode: PresentMode::Mailbox,
    };
    let mut swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);

    // Setup the default renderer
    let default_vertex_shader = include_bytes!("../default_renderer_vert.spv");
    let default_vertex_shader = read_spirv(Cursor::new(&default_vertex_shader[..])).unwrap();
    let default_vertex_shader = device.create_shader_module(&default_vertex_shader);

    let default_fragment_shader = include_bytes!("../default_renderer_frag.spv");
    let default_fragment_shader = read_spirv(Cursor::new(&default_fragment_shader[..])).unwrap();
    let default_fragment_shader = device.create_shader_module(&default_fragment_shader);

    let default_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        bind_group_layouts: &[],
    });
    let default_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        layout: &default_pipeline_layout,
        vertex_stage: ProgrammableStageDescriptor {
            module: &default_vertex_shader,
            entry_point: "main",
        },
        fragment_stage: Some(ProgrammableStageDescriptor {
            module: &default_fragment_shader,
            entry_point: "main",
        }),
        rasterization_state: Some(RasterizationStateDescriptor {
            front_face: FrontFace::Ccw,
            cull_mode: CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
        }),
        primitive_topology: PrimitiveTopology::TriangleList,
        color_states: &[ColorStateDescriptor {
            format: TextureFormat::Bgra8UnormSrgb,
            color_blend: BlendDescriptor::REPLACE,
            alpha_blend: BlendDescriptor::REPLACE,
            write_mask: ColorWrite::ALL,
        }],
        depth_stencil_state: None,
        vertex_state: VertexStateDescriptor {
            index_format: IndexFormat::Uint16,
            vertex_buffers: &[],
        },
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    });

    // Setup simple_render_passes
    // In this example, there is only 1 simple_render_pass added
    // It takes the texture drawn, and makes the left half of it pink
    let pink_simple_shader = include_bytes!("../pink_simple_renderer.spv");
    let pink_simple_shader = read_spirv(Cursor::new(&pink_simple_shader[..])).unwrap();
    let pink_simple_shader = device.create_shader_module(&pink_simple_shader);

    let pink_simple_bind_group_layout =
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            bindings: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::StorageTexture {
                        dimension: TextureViewDimension::D2,
                        component_type: TextureComponentType::Uint,
                        format: TextureFormat::Bgra8UnormSrgb,
                        readonly: true,
                    },
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::StorageTexture {
                        dimension: TextureViewDimension::D2,
                        component_type: TextureComponentType::Uint,
                        format: TextureFormat::Bgra8UnormSrgb,
                        readonly: true,
                    },
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::StorageTexture {
                        dimension: TextureViewDimension::D2,
                        component_type: TextureComponentType::Uint,
                        format: TextureFormat::Bgra8UnormSrgb,
                        readonly: false,
                    },
                },
            ],
            label: None,
        });
    let pink_simple_bind_group = device.create_bind_group(&BindGroupDescriptor {
        layout: &pink_simple_bind_group_layout,
        bindings: &[
            Binding {
                binding: 0,
                resource: BindingResource::TextureView(&default_texture),
            },
            Binding {
                binding: 1,
                resource: BindingResource::TextureView(&default_texture),
            },
            Binding {
                binding: 2,
                resource: BindingResource::TextureView(&pink_simple_texture),
            },
        ],
        label: None,
    });

    let pink_simple_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        bind_group_layouts: &[&pink_simple_bind_group_layout],
    });
    let pink_simple_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        layout: &pink_simple_pipeline_layout,
        compute_stage: ProgrammableStageDescriptor {
            module: &pink_simple_shader,
            entry_point: "main",
        },
    });

    // Setup the copy renderer
    let copy_sampler = device.create_sampler(&SamplerDescriptor {
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Nearest,
        min_filter: FilterMode::Nearest,
        mipmap_filter: FilterMode::Nearest,
        lod_min_clamp: 0.0,
        lod_max_clamp: 100.0,
        compare: CompareFunction::Never,
    });

    let copy_vertex_shader = include_bytes!("../copy_vert.spv");
    let copy_vertex_shader = read_spirv(Cursor::new(&copy_vertex_shader[..])).unwrap();
    let copy_vertex_shader = device.create_shader_module(&copy_vertex_shader);

    let copy_fragment_shader = include_bytes!("../copy_frag.spv");
    let copy_fragment_shader = read_spirv(Cursor::new(&copy_fragment_shader[..])).unwrap();
    let copy_fragment_shader = device.create_shader_module(&copy_fragment_shader);

    let copy_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        bindings: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStage::FRAGMENT,
                ty: BindingType::SampledTexture {
                    dimension: TextureViewDimension::D2,
                    component_type: TextureComponentType::Uint,
                    multisampled: false,
                },
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStage::FRAGMENT,
                ty: BindingType::Sampler { comparison: false },
            },
        ],
        label: None,
    });
    let copy_bind_group = device.create_bind_group(&BindGroupDescriptor {
        layout: &copy_bind_group_layout,
        bindings: &[
            Binding {
                binding: 0,
                resource: BindingResource::TextureView(&pink_simple_texture),
            },
            Binding {
                binding: 1,
                resource: BindingResource::Sampler(&copy_sampler),
            },
        ],
        label: None,
    });

    let copy_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        bind_group_layouts: &[&copy_bind_group_layout],
    });
    let copy_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        layout: &copy_pipeline_layout,
        vertex_stage: ProgrammableStageDescriptor {
            module: &copy_vertex_shader,
            entry_point: "main",
        },
        fragment_stage: Some(ProgrammableStageDescriptor {
            module: &copy_fragment_shader,
            entry_point: "main",
        }),
        rasterization_state: Some(RasterizationStateDescriptor {
            front_face: FrontFace::Ccw,
            cull_mode: CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
        }),
        primitive_topology: PrimitiveTopology::TriangleList,
        color_states: &[ColorStateDescriptor {
            format: TextureFormat::Bgra8UnormSrgb,
            color_blend: BlendDescriptor::REPLACE,
            alpha_blend: BlendDescriptor::REPLACE,
            write_mask: ColorWrite::ALL,
        }],
        depth_stencil_state: None,
        vertex_state: VertexStateDescriptor {
            index_format: IndexFormat::Uint16,
            vertex_buffers: &[],
        },
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    });

    // Event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => window.request_redraw(),

            // Resize
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                screen_size = new_size;

                swap_chain_descriptor.width = screen_size.width;
                swap_chain_descriptor.height = screen_size.height;
                swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);

                texture_descriptor.size.width = screen_size.width;
                texture_descriptor.size.height = screen_size.height;
                default_texture = device
                    .create_texture(&texture_descriptor)
                    .create_default_view();
                pink_simple_texture = device
                    .create_texture(&texture_descriptor)
                    .create_default_view();
            }

            // Rendering
            Event::RedrawRequested(_) => {
                let display_texture = &swap_chain.get_next_texture().unwrap().view;
                let mut encoder =
                    device.create_command_encoder(&CommandEncoderDescriptor { label: None });

                // Default renderer
                {
                    let mut default_render_pass =
                        encoder.begin_render_pass(&RenderPassDescriptor {
                            color_attachments: &[RenderPassColorAttachmentDescriptor {
                                attachment: &default_texture,
                                resolve_target: None,
                                load_op: LoadOp::Clear,
                                store_op: StoreOp::Store,
                                clear_color: Color::BLACK,
                            }],
                            depth_stencil_attachment: None,
                        });
                    default_render_pass.set_pipeline(&default_pipeline);
                    default_render_pass.draw(0..6, 0..1);
                }

                // Pink simple renderer
                {
                    let mut pink_simple_pass = encoder.begin_compute_pass();
                    pink_simple_pass.set_bind_group(0, &pink_simple_bind_group, &[]);
                    pink_simple_pass.set_pipeline(&pink_simple_pipeline);
                    pink_simple_pass.dispatch(
                        (screen_size.width / 7) + 8,
                        (screen_size.height / 7) + 8,
                        1,
                    );
                }

                // Copy renderer
                {
                    let mut copy_render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        color_attachments: &[RenderPassColorAttachmentDescriptor {
                            attachment: display_texture,
                            resolve_target: None,
                            load_op: LoadOp::Clear,
                            store_op: StoreOp::Store,
                            clear_color: Color::BLACK,
                        }],
                        depth_stencil_attachment: None,
                    });
                    copy_render_pass.set_bind_group(0, &copy_bind_group, &[]);
                    copy_render_pass.set_pipeline(&copy_pipeline);
                    copy_render_pass.draw(0..6, 0..1);
                }

                queue.submit(&[encoder.finish()]);
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,

            _ => {}
        }
    });
}
