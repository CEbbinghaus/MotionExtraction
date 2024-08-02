use bytemuck::{Pod, Zeroable};
use nokhwa::{
    pixel_format::RgbAFormat,
    utils::*,
    Camera,
};
use std::{
    borrow::Cow, sync::{Arc, Mutex}, thread, vec
};
use wgpu::{util::DeviceExt, ImageDataLayout};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    _pos: [f32; 4],
}

fn vertex(x: f32, y: f32) -> Vertex {
    Vertex {
        _pos: [x, y, 0.0, 1.0],
    }
}

fn padded_bytes_per_row(width: u32) -> u32 {
    let bytes_per_row = width as u32 * 4;
    let padding = (256 - bytes_per_row % 256) % 256;
    bytes_per_row + padding
}

fn read_webcam_frames(
    refresh_window: impl Fn(),
    queue: Arc<Mutex<wgpu::Queue>>,
    cur_texture: &wgpu::Texture,
    prev_texture: &wgpu::Texture,
    image_data_layout: ImageDataLayout,
    texture_extent: wgpu::Extent3d,
    buffer_size: usize,
) {
    let mut camera = Camera::new(CameraIndex::Index(0), RequestedFormat::new::<RgbAFormat>(RequestedFormatType::AbsoluteHighestFrameRate)).unwrap();
    camera.open_stream().unwrap();
    
    let mut cur_buffer: Vec<u8> = vec![0; buffer_size];
    let mut prev_buffer: Vec<u8> = vec![0; buffer_size];

    loop {
        let frame = camera.frame().unwrap();
        // frame
        //     .decode_image_to_buffer::<RgbAFormat>(cur_buffer.as_mut_slice())
        //     .unwrap();

        let decompress = mozjpeg::Decompress::new_mem(frame.buffer()).unwrap();

        let mut decompress = decompress.rgba().unwrap();

        decompress.read_scanlines_into(&mut cur_buffer).unwrap();

        {
            let queue = queue.lock().unwrap();
            
            queue.write_texture(
                cur_texture.as_image_copy(),
                &cur_buffer,
                image_data_layout,
                texture_extent,
            );
    
            queue.write_texture(
                prev_texture.as_image_copy(),
                &prev_buffer,
                image_data_layout,
                texture_extent,
            );
        }
        refresh_window();
        prev_buffer.copy_from_slice(&cur_buffer);
        // thread::sleep(Duration::from_millis(16));
    }
}

async fn run(size: (u32, u32), event_loop: EventLoop<()>, window: Window) {
    let (width, height) = size;
    let vertex_size = std::mem::size_of::<Vertex>();
    let vertices = [
        vertex(-1.0, -1.0),
        vertex(1.0, -1.0),
        vertex(-1.0, 1.0),
        vertex(1.0, -1.0),
        vertex(-1.0, 1.0),
        vertex(1.0, 1.0),
    ];

    let buffer_width = padded_bytes_per_row(width);

    let texture_extent = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let tmp_buffer: Vec<u8> = vec![0; (buffer_width * height) as usize];

    let instance = wgpu::Instance::default();

   let surface = instance.create_surface(&window).unwrap();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            // Request an adapter which can render to our surface
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");

    // Create the logical device and command queue
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
            },
            None,
        )
        .await
        .expect("Failed to create device");

    // Load the shaders from disk
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/shader.wgsl"))),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(8),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    sample_type: wgpu::TextureSampleType::Uint,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    sample_type: wgpu::TextureSampleType::Uint,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Outer Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let vertex_buffers = [wgpu::VertexBufferLayout {
        array_stride: vertex_size as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 0,
            shader_location: 0,
        }],
    }];

    let viewport: &[u32; 2] = &[width, height];
    let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::cast_slice(viewport),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let image_data_layout = wgpu::ImageDataLayout {
        offset: 0,
        bytes_per_row: Some(width * 4),
        rows_per_image: None,
    };

    let cur_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: texture_extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Uint,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let cur_texture_view = cur_texture.create_view(&wgpu::TextureViewDescriptor::default());
    queue.write_texture(
        cur_texture.as_image_copy(),
        &tmp_buffer,
        image_data_layout,
        texture_extent,
    );
    let prev_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: texture_extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Uint,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let prev_texture_view = prev_texture.create_view(&wgpu::TextureViewDescriptor::default());
    queue.write_texture(
        prev_texture.as_image_copy(),
        &tmp_buffer,
        image_data_layout,
        texture_extent,
    );

    drop(tmp_buffer);

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&cur_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&prev_texture_view),
            },
        ],
        label: None,
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &vertex_buffers,
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            compilation_options: Default::default(),
            targets: &[Some(swapchain_format.into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    let mut config = surface
        .get_default_config(&adapter, width, height)
        .unwrap();
    surface.configure(&device, &config);

    let queue = Arc::new(Mutex::new(queue));
    let window = &window;

    let thread_queue = queue.clone();
    let event_loop_proxy = event_loop.create_proxy();
    thread::spawn(move || {
        read_webcam_frames(
            || {
                event_loop_proxy.send_event(()).unwrap();
            },
            thread_queue,
            &cur_texture,
            &prev_texture,
            image_data_layout,
            texture_extent,
            buffer_width as usize * height as usize,
        );
    });
    event_loop
        .run( |event, target| {
            // Have the closure take ownership of the resources.
            // `event_loop.run` never returns, therefore we must do this to ensure
            // the resources are properly cleaned up.
            let _ = (&instance, &adapter, &shader, &pipeline_layout);

            //println!("Recieved event {event:?}");
            if let Event::UserEvent(_) = event {
                window.request_redraw();
            }
            if let Event::WindowEvent {
                window_id: _,
                event,
            } = event
            {
                match event {
                    WindowEvent::Resized(new_size) => {
                        // Reconfigure the surface with the new size
                        config.width = new_size.width.max(1);
                        config.height = new_size.height.max(1);
                        surface.configure(&device, &config);
                        // On macos the window needs to be redrawn manually after resizing
                        window.request_redraw();
                    }
                    WindowEvent::RedrawRequested => {
                        let frame = surface
                            .get_current_texture()
                            .expect("Failed to acquire next swap chain texture");
                        let view = frame
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());
                        let mut encoder =
                            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: None,
                            });
                        {
                            let mut rpass =
                                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: None,
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                        view: &view,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                                            store: wgpu::StoreOp::Store,
                                        },
                                    })],
                                    depth_stencil_attachment: None,
                                    timestamp_writes: None,
                                    occlusion_query_set: None,
                                });
                            rpass.set_bind_group(0, &bind_group, &[]);
                            rpass.set_pipeline(&render_pipeline);
                            rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
                            rpass.draw(0..(vertices.len() as u32), 0..1);
                        }

                        queue.lock().unwrap().submit(Some(encoder.finish()));
                        frame.present();
                    }
                    WindowEvent::CloseRequested => target.exit(),
                    _ => {
                    }
                };
            }
        })
        .unwrap();
}

pub fn main() {
    // get a frame. We use this to scale the window
    let frame = {
        let mut camera = Camera::new(CameraIndex::Index(0), RequestedFormat::new::<RgbAFormat>(RequestedFormatType::AbsoluteHighestFrameRate)).unwrap();
        camera.open_stream().unwrap();
        camera.frame().unwrap()
    };

    let (width, height) = {
        let resolution = frame.resolution();
        (resolution.width_x, resolution.height_y)
    };


    let event_loop = EventLoop::new().unwrap();
    #[allow(unused_mut)]
    let mut builder =
        winit::window::WindowBuilder::new().with_inner_size(PhysicalSize::new(width, height));
    let window = builder.build(&event_loop).unwrap();
    window.set_resizable(false);

    pollster::block_on(run((width, height), event_loop, window));
}
