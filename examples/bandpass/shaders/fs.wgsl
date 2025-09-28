struct Uniforms {
    u_value: array<array<f32, 88>, 256>,
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
    let freq_index = i32(floor(uv.x * num_bins));

    if freq_index < 0 || freq_index >= i32(num_bins) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let time_index = i32(u.history_index);
    let raw = u.u_value[time_index][freq_index];
    let visual_magnitude = sqrt(raw)*5.0;
    let magnitude = clamp(visual_magnitude, 0.0, 1.0);
    let bar_y = uv.y;

    if bar_y < magnitude {
        let white_color = vec3<f32>(1.0, 1.0, 1.0);
        let bar_brightness = 1.0 - pow(bar_y / magnitude, 4.0);
        let final_color = white_color * bar_brightness;
        return vec4<f32>(final_color, 1.0);
        
    } else {
        return vec4<f32>(vec3<f32>(0.05, 0.05, 0.05) * 0.1, 1.0); 
    }
}

