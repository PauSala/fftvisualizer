use std::time::Duration;

use audio_streams::{AudioConsumerF32, AudioProducerF32, FftConsumer, InputModel};
use nannou::prelude::*;
use nannou_audio::{self as audio, Buffer};
use rand::Rng;
use ringbuf::{traits::*, HeapRb}; // Add rand crate to your dependencies

pub fn generate_random_noise(length: usize) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    let mut samples = Vec::with_capacity(length);

    for _ in 0..length {
        samples.push(rng.gen_range(-1.0..1.0));
    }

    samples
}

/// Input buffer
const IB_LEN: usize = 1024;
/// Frequencies buffer
const FB_LEN: usize = IB_LEN / 2;
/// Display buffer
const DB_LEN: usize = FB_LEN / 1;
/// Dellta factor for smoothing
pub const DELTA: usize = 2;
///
const WIDTH: usize = 512;
const HEIGHT: usize = 512;

fn main() {
    nannou::app(model).update(update).run();
}

pub struct Model {
    pub audio_in: audio::Stream<AudioProducerF32>,
    pub fft_analizer: AudioConsumerF32<IB_LEN, FB_LEN, DELTA>,
    pub elapsed: Duration,

    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
}

impl Model {
    pub fn update(&mut self, milis: Duration) {
        self.fft_analizer.update(milis);
    }
}

fn update(_app: &App, model: &mut Model, update: Update) {
    let milis = update.since_last;
    // This is due to precission issues if the elapsed time is too short
    model.update(milis);
}

fn view(app: &App, model: &Model, frame: Frame) {
    let uvalue = mutate_uniforms(&model.fft_analizer.smoothed);
    let uniforms = Uniforms {
        u_value: uvalue,
        time: app.time,
        freq: 0.0,
        width: WIDTH as f32,
        height: HEIGHT as f32,
    };
    let uniforms_size = std::mem::size_of::<Uniforms>() as wgpu::BufferAddress;
    let uniforms_bytes = uniforms_as_bytes(&uniforms);
    let usage = wgpu::BufferUsages::COPY_SRC;
    let device = frame.device_queue_pair().device();
    let new_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: uniforms_bytes,
        usage,
    });
    // Using this we will encode commands that will be submitted to the GPU.

    let mut encoder = frame.command_encoder();
    encoder.copy_buffer_to_buffer(
        &new_uniform_buffer,
        0,
        &model.uniform_buffer,
        0,
        uniforms_size,
    );

    // The render pass can be thought of a single large command consisting of sub commands. Here we
    // begin a render pass that outputs to the frame's texture. Then we add sub-commands for
    // setting the bind group, render pipeline, vertex buffers and then finally drawing.
    let mut render_pass = wgpu::RenderPassBuilder::new()
        .color_attachment(frame.texture_view(), |color| color)
        .begin(&mut encoder);
    render_pass.set_bind_group(0, &model.bind_group, &[]);
    // render_pass.set_bind_group(1, &model.camera_bind_group, &[]);
    render_pass.set_pipeline(&model.render_pipeline);
    render_pass.set_vertex_buffer(0, model.vertex_buffer.slice(..));

    // We want to draw the whole range of vertices, and we're only drawing one instance of them.

    let vertex_range = 0..VERTICES.len() as u32;
    let instance_range = 0..1;
    render_pass.draw(vertex_range, instance_range);

    // Now we're done! The commands we added will be submitted after `view` completes.
}

fn model(app: &App) -> Model {
    // Initialise the audio host so we can spawn an audio stream.
    let audio_host = audio::Host::new();

    // Create a ring buffer and split it into producer and consumer
    let rb = HeapRb::<f32>::new(IB_LEN);
    let (prod, cons) = rb.split();

    // Input stream
    let in_model = InputModel { producer: prod };
    let in_stream = audio_host
        .new_input_stream(in_model)
        .capture(pass_in)
        .build()
        .unwrap();

    // FftConsumer:  recieves from input stream
    let channels = in_stream.cpal_config().channels;
    let output_model = FftConsumer::new(cons, channels);

    // Start input stream
    in_stream.play().unwrap();

    let w_id = app
        .new_window()
        .size(WIDTH.try_into().unwrap(), HEIGHT.try_into().unwrap())
        .view(view)
        .build()
        .unwrap();

    // The gpu device associated with the window's swapchain
    let window = app.window(w_id).unwrap();
    let device = window.device();
    let format = Frame::TEXTURE_FORMAT;
    let sample_count = window.msaa_samples();

    // Load shader modules.
    let vs_desc = wgpu::include_wgsl!("shaders/vs.wgsl");
    let fs_desc = wgpu::include_wgsl!("shaders/fs.wgsl");
    let vs_mod = device.create_shader_module(vs_desc);
    let fs_mod = device.create_shader_module(fs_desc);

    // Create the vertex buffer.
    let vertices_bytes = vertices_as_bytes(&VERTICES[..]);
    let usage = wgpu::BufferUsages::VERTEX;
    let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: vertices_bytes,
        usage,
    });

    //uniforms
    // Create the buffer that will store time.
    let uniforms = Uniforms {
        u_value: create_uniforms(),
        time: 0.0,
        freq: 0.0,
        width: WIDTH as f32,
        height: HEIGHT as f32,
    };
    let uniforms_bytes = uniforms_as_bytes(&uniforms);
    let usage = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
    let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("uniform-buffer"),
        contents: uniforms_bytes,
        usage,
    });
    // Create the bind group layout specifying the binding for uniforms.
    let bind_group_layout = create_mixed_bind_group_layout(device);
    // Create the bind group using the layout and uniform buffer.
    let bind_group = create_bind_group(device, &bind_group_layout, &uniform_buffer);

    // Create the pipeline layout using the bind group layout.
    let pipeline_layout = create_pipeline_layout(&device, &bind_group_layout);

    // Create the render pipeline using the vertex and fragment shaders, bind group, and pipeline layout.
    let render_pipeline = wgpu::RenderPipelineBuilder::from_layout(&pipeline_layout, &vs_mod)
        .fragment_shader(&fs_mod)
        .color_format(format)
        .add_vertex_buffer_layout(Vertex::desc())
        .sample_count(sample_count)
        .build(device);

    Model {
        audio_in: in_stream,
        fft_analizer: output_model,
        elapsed: Duration::from_secs(0),
        render_pipeline,
        bind_group,
        vertex_buffer,
        uniform_buffer,
    }
}

fn create_uniforms() -> [f32; DB_LEN] {
    [0.0; DB_LEN]
}

fn mutate_uniforms(u: &[f32; FB_LEN]) -> [f32; DB_LEN] {
    let mut uniforms = [0.0; DB_LEN];
    for i in 0..DB_LEN {
        uniforms[i] = u[i];
    }
    uniforms
}

fn create_mixed_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    wgpu::BindGroupLayoutBuilder::new()
        .uniform_buffer(wgpu::ShaderStages::VERTEX_FRAGMENT, false)
        .build(device)
}

fn create_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    wgpu::BindGroupBuilder::new()
        .buffer::<Uniforms>(uniform_buffer, 0..1)
        .build(device, layout)
}

fn create_pipeline_layout(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::PipelineLayout {
    let desc = wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    };
    device.create_pipeline_layout(&desc)
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Uniforms {
    u_value: [f32; DB_LEN],
    time: f32,
    freq: f32,
    height: f32,
    width: f32,
}

// The vertex type that we will use to represent a point on our triangle.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex {
    position: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// See the `nannou::wgpu::bytes` documentation for why this is necessary.
fn vertices_as_bytes(data: &[Vertex]) -> &[u8] {
    unsafe { wgpu::bytes::from_slice(data) }
}

fn uniforms_as_bytes(uniforms: &Uniforms) -> &[u8] {
    unsafe { wgpu::bytes::from(uniforms) }
}

// The vertices that make up our layout.
// The vertices that make up our hexagon layout.
const VERTICES: [Vertex; 6] = [
    Vertex {
        position: [-1.0, -1.0],
    },
    Vertex {
        position: [1.0, -1.0],
    },
    Vertex {
        position: [-1.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0],
    },
    Vertex {
        position: [1.0, 1.0],
    },
    Vertex {
        position: [-1.0, 1.0],
    },
];

pub fn pass_in<T: Producer<Item = f32>>(model: &mut InputModel<T>, buffer: &Buffer) {
    for frame in buffer.frames() {
        for sample in frame {
            model.producer.try_push(*sample).ok();
        }
    }
}
