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
    let freq_index = i32(floor(uv.y * num_bins));

    let history_len_i = i32(u.history_len);
    let history_index_i = i32(u.history_index);
    let time_offset = i32(uv.x * u.history_len);
    let time_index = (history_index_i + time_offset) % history_len_i;

    let eps = 1e-6;
    let raw = u.u_value[time_index][freq_index] * (pow(uv.x, 2.5) + eps);
    let magnitude = sqrt(raw*100.0);

    let note_index = freq_index % 12;
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


    let yellow_color = vec3<f32>(0.99876, 0.98098, 0.8998698);
    let yellow_mix = uv.y; 
    let mix_factor = pow(yellow_mix, 3.0); 
    let final_color_base = mix(base_color, yellow_color, mix_factor);
    let color = final_color_base * max(magnitude, 0.001);

    return vec4<f32>(color, 1.0);

}

