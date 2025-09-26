struct Uniforms {
    u_value: array<array<f32, 88>, 128>,
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

    let num_bins = 88.0;
    let freq_index = i32(floor(uv.y * num_bins));

    // Map to time index for scrolling
    let history_len_i = i32(u.history_len);
    let history_index_i = i32(u.history_index);
    let time_offset = i32(uv.x * u.history_len);
    let time_index = (history_index_i + time_offset) % history_len_i;

    // Fetch magnitude
    let eps = 1e-6;
    let raw = u.u_value[time_index][freq_index] * (pow(uv.x, 1.5) + eps);
    let magnitude = raw*1000.0; //sqrt(raw) * 5.0;

    // --- Color by note (repeating every 12 bins) ---
    let note_index = freq_index % 12; // 0..11
    var base_color: vec3<f32>;

switch(note_index) {
    case 0:  { base_color = vec3<f32>(1.0, 0.4897923, 0.4897923); }   // C   - red
    case 1:  { base_color = vec3<f32>(0.4897923, 1.0, 0.4897923); }   // C#  - green
    case 2:  { base_color = vec3<f32>(0.4, 0.4897923, 1.0); }   // D   - blue
    case 3:  { base_color = vec3<f32>(1.0, 1.0, 0.4897923); }   // D#  - yellow
    case 4:  { base_color = vec3<f32>(1.0, 0.4897923, 1.0); }   // E   - magenta
    case 5:  { base_color = vec3<f32>(0.4897923, 1.0, 1.0); }   // F   - cyan
    case 6:  { base_color = vec3<f32>(0.8, 0.4, 0.0); }   // F#  - orange
    case 7:  { base_color = vec3<f32>(0.4, 0.0, 0.8); }   // G   - purple
    case 8:  { base_color = vec3<f32>(0.0, 0.8, 0.4); }   // G#  - teal
    case 9:  { base_color = vec3<f32>(1.0, 0.6, 0.0); }   // A   - amber
    case 10: { base_color = vec3<f32>(0.6, 0.0, 1.0); }   // A#  - violet
    case 11: { base_color = vec3<f32>(0.0, 1.0, 0.6); }   // B   - spring green
    default: { base_color = vec3<f32>(1.0, 1.0, 1.0); }   // fallback
}


    // Modulate color by magnitude
    let color = base_color * max(magnitude, 0.001);

    return vec4<f32>(color, 1.0);
}

