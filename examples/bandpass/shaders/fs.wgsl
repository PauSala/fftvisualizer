struct Uniforms {
    u_value: array<array<f32, 44>, 128>,
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

    let num_bins = 44.0;
    let freq_index = i32(floor(uv.y * num_bins));

    // Map to time index for scrolling
    let history_len_i = i32(u.history_len);
    let history_index_i = i32(u.history_index);
    let time_offset = i32(uv.x * u.history_len);
    let time_index = (history_index_i + time_offset) % history_len_i;

    // Fetch magnitude
    let eps = 1e-6;
    let raw = u.u_value[time_index][freq_index] * (pow(uv.x, 1.5) + eps);
    let magnitude = sqrt(raw*100.0);

    // --- Color by note (repeating every 12 bins) ---
    let note_index = freq_index % 12; // 0..11
    var base_color: vec3<f32>;

    switch(note_index) {
        case 0:  { base_color = vec3<f32>(0.5333333, 0.0666667, 0.4666667); }  // #817
        case 1:  { base_color = vec3<f32>(0.6666667, 0.2000000, 0.3333333); }  // #a35
        case 2:  { base_color = vec3<f32>(0.8000000, 0.4000000, 0.4000000); }  // #c66
        case 3:  { base_color = vec3<f32>(0.9333333, 0.6000000, 0.2666667); }  // #e94
        case 4:  { base_color = vec3<f32>(0.9333333, 0.8666667, 0.0000000); }  // #ed0
        case 5:  { base_color = vec3<f32>(0.6000000, 0.8666667, 0.3333333); }  // #9d5
        case 6:  { base_color = vec3<f32>(0.2666667, 0.8666667, 0.5333333); }  // #4d8
        case 7:  { base_color = vec3<f32>(0.1333333, 0.8000000, 0.7333333); }  // #2cb
        case 8:  { base_color = vec3<f32>(0.0000000, 0.7333333, 0.8000000); }  // #0bc
        case 9:  { base_color = vec3<f32>(0.0000000, 0.6000000, 0.8000000); }  // #09c
        case 10: { base_color = vec3<f32>(0.2000000, 0.4000000, 0.7333333); }  // #36b
        case 11: { base_color = vec3<f32>(0.4000000, 0.2000000, 0.6000000); }  // #639
        default: { base_color = vec3<f32>(1.0, 1.0, 1.0); }
    }


    // --- High-Frequency Yellow Filter (NEW LOGIC) ---
    let yellow_color = vec3<f32>(1.0, 1.0, 0.0); // Pure Yellow
    
    // Create a normalized factor based on the frequency index (0 to 1)
    // The uv.y value already gives a smooth 0.0 to 1.0 transition from bottom to top.
    let yellow_mix = uv.y; 
    
    // Increase the exponent (e.g., pow(uv.y, 2.0)) to delay the yellow tint 
    // until the highest frequencies, making the effect more pronounced at the top.
    let mix_factor = pow(yellow_mix, 2.0); 

    // Linearly interpolate (mix) between the note's base_color and the yellow_color.
    let final_color_base = mix(base_color, yellow_color, mix_factor);


    // Modulate color by magnitude
    let color = final_color_base * max(magnitude, 0.001);

    return vec4<f32>(color, 1.0);

}

