use std::time::Duration;

use audio_streams::{
    bandpass::{AudioConsumerFilterBankF32, FilterBankConsumer},
    AudioProducerF32, InputModel,
};
use nannou::prelude::*;
use nannou_audio::{self as audio, Buffer};
use ringbuf::{traits::*, HeapRb}; // Add rand crate to your dependencies

/// Input buffer
const IB_LEN: usize = 2048;
/// Frequencies buffer
const FB_LEN: usize = 88;
/// Display buffer
const DB_LEN: usize = FB_LEN / 1;
/// Number of FFT frames to store in history
const HISTORY_LEN: usize = 128;
/// Dellta factor for smoothing
pub const DELTA: usize = 4;
///
const WIDTH: usize = 512;
const HEIGHT: usize = 512;

fn main() {
    nannou::app(model).update(update).run();
}

pub struct Model {
    pub audio_in: audio::Stream<AudioProducerF32>,
    pub filter_bank: AudioConsumerFilterBankF32<IB_LEN, FB_LEN, DELTA>,
    pub elapsed: Duration,
    fft_history: [[f32; DB_LEN]; HISTORY_LEN],
    history_index: usize,

    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    storage_buffer: wgpu::Buffer,
    pub time: Duration,
}

impl Model {
    pub fn update(&mut self, milis: Duration) {
        self.filter_bank.update(milis);
    }
}

fn update(_app: &App, model: &mut Model, update: Update) {
    let milis = update.since_last;
    // This is due to precission issues if the elapsed time is too short
    model.update(milis);

    model.time += milis;
    // Store the latest FFT data in our history buffer
    if model.time.as_millis() > 0 {
        let new_fft_data = mutate_uniforms(&model.filter_bank.smoothed);
        model.fft_history[model.history_index] = new_fft_data;
        model.history_index = (model.history_index + 1) % HISTORY_LEN;
        model.time = Duration::from_millis(0);
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let uniforms = Uniforms {
        u_value: model.fft_history,
        time: app.time,
        history_len: HISTORY_LEN as f32,
        width: app.main_window().rect().w(),
        height: app.main_window().rect().h(),
        history_index: model.history_index as f32,
        _padding: [0.0; 3],
    };
    let uniforms_size = std::mem::size_of::<Uniforms>() as wgpu::BufferAddress;
    let uniforms_bytes = uniforms_as_bytes(&uniforms);
    let usage = wgpu::BufferUsages::COPY_SRC;
    let device = frame.device_queue_pair().device();
    let temp_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("temp-storage-buffer"),
        contents: uniforms_bytes,
        usage,
    });
    // Using this we will encode commands that will be submitted to the GPU.

    let mut encoder = frame.command_encoder();
    encoder.copy_buffer_to_buffer(&temp_buffer, 0, &model.storage_buffer, 0, uniforms_size);

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

    let output_model = FilterBankConsumer::new(
        cons,
        in_stream.cpal_config().sample_rate.0 as f32,
        27.5,
        4186.0,
    );

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
        u_value: [[0.0; DB_LEN]; HISTORY_LEN],
        time: 0.0,
        history_len: HISTORY_LEN as f32,
        width: WIDTH as f32,
        height: HEIGHT as f32,
        history_index: 0.0,
        _padding: [0.0; 3],
    };
    let uniforms_bytes = uniforms_as_bytes(&uniforms);
    let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
    let storage_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("storage-buffer"),
        contents: uniforms_bytes,
        usage,
    });
    // Create the bind group layout specifying the binding for uniforms.
    let bind_group_layout = create_bind_group_layout(device);
    // Create the bind group using the layout and uniform buffer.
    let bind_group = create_bind_group(device, &bind_group_layout, &storage_buffer);

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
        filter_bank: output_model,
        elapsed: Duration::from_secs(0),
        fft_history: [[0.0; DB_LEN]; HISTORY_LEN],
        history_index: 0,
        render_pipeline,
        bind_group,
        vertex_buffer,
        storage_buffer,
        time: Duration::from_secs(0),
    }
}

fn mutate_uniforms(u: &[f32; FB_LEN]) -> [f32; DB_LEN] {
    let mut uniforms = [0.0; DB_LEN];
    for i in 0..DB_LEN {
        uniforms[i] = u[i];
    }
    uniforms
}

fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    wgpu::BindGroupLayoutBuilder::new()
        .storage_buffer(wgpu::ShaderStages::FRAGMENT, false, true)
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
    u_value: [[f32; DB_LEN]; HISTORY_LEN],
    time: f32,
    history_len: f32,
    height: f32,
    width: f32,
    history_index: f32,
    _padding: [f32; 3], // Padding to align to 16 bytes
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
