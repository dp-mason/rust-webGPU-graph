
// Received a ton of help from: https://sotrh.github.io/learn-wgpu/beginner/tutorial2-surface/#first-some-housekeeping-state
use wgpu::{Instance, util::DeviceExt};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position:[f32;3],
    color:[f32;3]
}

// define triangles that fill the screen
// to accomodate this setup in the render pipeline config settings, the topology is set to "strip"
const VERTICES:&[Vertex] = &[
    Vertex{position: [ 1.0,  1.0, 0.0], color: [1.0, 0.0, 0.0]},
    Vertex{position: [-1.0,  1.0, 0.0], color: [0.0, 0.0, 0.0]},
    Vertex{position: [ 1.0, -1.0, 0.0], color: [1.0, 1.0, 0.0]},
    Vertex{position: [-1.0, -1.0, 0.0], color: [0.0, 1.0, 0.0]},
];

// data passed to the GPU about the state of the cursor
pub struct CursorState {
    pressed:bool,
    position:[f64;2],
}
impl CursorState {
    fn new() -> Self {
        let pressed = false;
        // pixel position of cursor in the window, must be normalized for window size to get 0..1 position 
        let position = [0.0, 0.0]; 

        Self {
            pressed,
            position,
        }
    }
    fn get_pos_f32 (&self) -> [f32; 4] { // vec4 is the min size of a uniform buff
        [self.position[0] as f32, self.position[1] as f32, 0.0, 0.0]
    }
}

pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    load_color: wgpu::Color, // for the challenge section, delete later
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Window,
    render_pipeline: wgpu::RenderPipeline,
    cursor_pos_buffer: wgpu::Buffer,
    cursor_pos_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
}
impl State {
    pub async fn new(window: Window) -> Self {
        let size = window.inner_size();

        // creates (handle?) to the GPU, Backends refers to Vulkan, DXD12, Metal and (BrowserWebGPU?)
        let instance = wgpu::Instance::new(wgpu::Backends::all());

        // Apparently this should be safe, because "State" owns the window
        // this ensures that the State will live as long as the window?
        // "surface" is the section of the window we draw to
        let surface = unsafe { instance.create_surface(&window) };

        // "adapter" houses the info about our GPU
        // if you have multiple graphics cards you will be able to interate over returned adapters
        // used to create Device and Queue
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false, // do not default to software rendering
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                },
                // Some(&std::path::Path::new("trace")), // Trace path
                None,
            )
            .await
            .unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
        };
        surface.configure(&device, &config);

        let load_color = wgpu::Color{r:0.0, g:0.0, b:0.0, a:1.0};

        // "include_str!" imports the contents of a file as a static string, which can be useful
        // I instead use include_wgsl! which creates a ShaderModuleDescriptor from the file that you supply
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        
        // !! BUFFER STUFF !!

        // Vertex Buffer
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        // Create Vertex Buffer Layout
        // From: https://sotrh.github.io/learn-wgpu/beginner/tutorial4-buffer/#so-what-do-i-do-with-it
        //      We need to tell the render_pipeline to use this buffer when we are drawing, but first, we need to 
        //  tell the render_pipeline how to read the buffer. We do this using VertexBufferLayouts and the 
        //  vertex_buffers field that I promised we'd talk about when we created the render_pipeline.
        //      A VertexBufferLayout defines how a buffer is represented in memory. Without this, the 
        //  render_pipeline has no idea how to map the buffer in the shader. Here's what the descriptor for a 
        //  buffer full of Vertex would look like.
        // If we wanna get sophistacated with it, we can returnb this layout description in a implementation
        //  of the Vertex struct. Not too worried about it right now though
        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            // byte length of vertex data, so array can be stepped through in linear memory
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress, 
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // put the position data in the first location of the vert buffer
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3, // matches format defined in Vertex struct
                },
                // put the color data in the second location in the vert buffer
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress, // step past postion data to get color
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                }
            ]
        };

        // placeholder for mouse position
        // TODO: look into making this a struct with x and y components instead? good challenge!
        let cursor_state:CursorState = CursorState::new();

        // create a uniform buffer for the mouse location
        let cursor_pos_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor{
                label: Some("Cursor Position Buffer"),
                contents: bytemuck::cast_slice(&[cursor_state.get_pos_f32()]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // TODO: move stuff into diff functions so this is easier to read
        
        // create bind group LAYOUT with this buffer
        let cursor_pos_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("cursor_pos_bind_group_layout"),
        });

        // create ACTUAL bind group FROM LAYOUT and BUFFER that we just made
        let cursor_pos_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &cursor_pos_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: cursor_pos_buffer.as_entire_binding(),
                }
            ],
            label: Some("cursor_pos_bind_group"),
        });

        // create proper pipeline layout (declares buffers and such, look into this more)
        // use the LAYOUT we created
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Basic Pipeline Layout"),
                bind_group_layouts: &[&cursor_pos_bind_group_layout],
                push_constant_ranges: &[], // ???
            });
        
        // create a render pipeline
        let render_pipeline = 
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
                label: Some("Basic Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vert_main",
                    buffers: &[vertex_buffer_layout],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "frag_main",
                    targets: &[Some(wgpu::ColorTargetState{
                        format:config.format,                   // matches the color config of the SurfaceTexture
                        blend: Some(wgpu::BlendState::REPLACE), // replace the old pixel data with new data
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip, // every three verts forms a triangle
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw, // counter-clockwise is the direction tris are drawn
                    cull_mode: Some(wgpu::Face::Back), // if the triangle is facing away, do not draw it
                    // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                },
                depth_stencil: None, // is stencil like a silhoutte of object?
                multisample: wgpu::MultisampleState {
                    count: 1, // ?? look into the topic of "multisampling", like raytrace samples?
                    mask: !0, // all samples should be active ?? how is "!0" diff than "1"
                    alpha_to_coverage_enabled: false, // ?? has to do with anti-aliasing
                },
                multiview: None, // ?? look into what "array textures" are, more than one surface texture?
            });

        Self {
            surface,
            device,
            load_color,
            queue,
            config,
            size,
            window,
            render_pipeline,
            cursor_pos_buffer,
            cursor_pos_bind_group,
            vertex_buffer
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }
    // this is where more user input for rendering can be added
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        // match to some input events you want to handle
        // if this event is not in your list of input events, return false so it may be consumed
        // by another process
        match event {
            WindowEvent::CursorMoved{ position,.. } => {
                // change load color (default background color)
                let redval = ((position.x + f64::MIN_POSITIVE) / self.size.width as f64) % 1.0;
                let greenval = ((position.y + f64::MIN_POSITIVE) / self.size.height as f64) % 1.0;
                self.load_color = wgpu::Color { r: redval, g:greenval, b:1.0, a:1.0 };

                // write the new mouse position to buffer
                self.queue.write_buffer(
                    &self.cursor_pos_buffer, 
                    0, 
                    bytemuck::cast_slice(&[[
                        position.x as f32 / self.size.width as f32, 
                        position.y as f32 / self.size.height as f32,
                    ]]));

                true
            },
            _ => false
        }
    }

    pub fn update(&mut self) {

    }

    pub async fn init_rendering() -> (EventLoop<()>, State) {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                std::panic::set_hook(Box::new(console_error_panic_hook::hook));
                console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
            } else {
                env_logger::init();
            }
        }
    
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new().build(&event_loop).unwrap();
    
        #[cfg(target_arch = "wasm32")]
        {
            // Winit prevents sizing with CSS, so we have to set
            // the size manually when on web.
            use winit::dpi::PhysicalSize;
            window.set_inner_size(PhysicalSize::new(1024, 1024));
    
            use winit::platform::web::WindowExtWebSys;
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    let dst = doc.get_element_by_id("wasm-example")?;
                    let canvas = web_sys::Element::from(window.canvas());
                    dst.append_child(&canvas).ok()?;
                    Some(())
                })
                .expect("Couldn't append canvas to document body.");
        }

        let render_state = Self::new(window).await;

        (event_loop, render_state)
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output_texture = self.surface.get_current_texture()?;
        let view = output_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // create the GPU command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // from: https://sotrh.github.io/learn-wgpu/beginner/tutorial2-surface/#render
        //  begin_render_pass() borrows encoder mutably (aka &mut self). We can't call encoder.finish() until
        //  we release that mutable borrow. The block tells rust to drop any variables within it when the code
        //  leaves that scope thus releasing the mutable borrow on encoder and allowing us to finish() it.
        //  If you don't like the {}, you can also use drop(render_pass) to achieve the same effect.
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    // This is what @location(0) in the fragment shader targets in the frag shader wgsl code
                    Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            //load: wgpu::LoadOp::Clear(color),
                            load: wgpu::LoadOp::Clear(self.load_color),
                            store: true,
                        },
                    })
                ],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);

            // Designate a vertex buffer
            // The reason "slice" is used is because we can store many objects in a single vertex buffer
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            render_pass.set_bind_group(0, &self.cursor_pos_bind_group, &[]);

            //draw something with 4 vertices, and 1 instance. This is where @builtin(vertex_index) comes from in the vert shader wgsl code
            render_pass.draw(0..VERTICES.len() as u32, 0..1); // remember range is not max inclusive
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output_texture.present();

        Ok(())
    }

    pub fn handle_rendering_events(&mut self, event:Event<()>, control_flow:&mut ControlFlow) {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == self.window().id() => {
                if !self.input(event) {
                    // UPDATED!
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            self.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &&mut so w have to dereference it twice
                            self.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(window_id) if window_id == self.window().id() => {
                self.update();
                match self.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        self.resize(self.size)
                    }
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,

                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            Event::RedrawEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                self.window().request_redraw();
            }
            _ => {}
        }
    }
}


