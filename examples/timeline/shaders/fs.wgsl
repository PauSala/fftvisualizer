struct Uniforms {
    u_value: array<array<f32, 512>, 128>,
    time: f32,
    history_len: f32,
    height: f32,
    width: f32,
    history_index: f32,
};

@group(0) @binding(0) var<storage, read> u: Uniforms;

@fragment
fn main(@builtin(position) in_pos: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = vec2<f32>(
        in_pos.x / u.width,
        1.0 - (in_pos.y / u.height)
    );

    // Map uv.x to a time index in the history, making it scroll
    let history_len_i = i32(u.history_len);
    let history_index_i = i32(u.history_index);
    let time_offset = i32(uv.x * u.history_len);
    let time_index = (history_index_i + time_offset) % history_len_i;

    // Map uv.y to a frequency bin
    let freq_index = i32(uv.y * 512.0);

    // Get the FFT magnitude for the given time and frequency
    let magnitude = sqrt(u.u_value[time_index][freq_index] * (uv.x+0.0000001));

    // Visualize the magnitude (e.g., as a color)
    let color = vec3<f32>(magnitude, magnitude*magnitude, 0.018704897);

    return vec4<f32>(color, 1.0);
}