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

    // Setup the swapchain
    let screen_size = window.inner_size();
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
                swap_chain_descriptor.width = new_size.width;
                swap_chain_descriptor.height = new_size.height;
                swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);
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
                                attachment: display_texture,
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
