struct Uniforms {
    freqs: array<vec4<f32>, 128>,
    time: f32,
    freq: f32,
    height: f32,
    width: f32,
};


const PI: f32 = 3.14159265;

@group(0) @binding(0) var<uniform> u: Uniforms;

@fragment
fn main(@builtin(position) in: vec4<f32>) -> @location(0) vec4<f32> {

    let resolution: vec2<f32> = vec2<f32>(u.width, u.height);
    // calulate normalized coordinates to 0 - 2 range, Y from bottom to top
    // For some reason the viewport is doubled in size
    var uv: vec2<f32> =
        vec2<f32>(
            (in.x / 2.0) / resolution.x ,
            (resolution.y - (in.y / 2.0)) / resolution.y
        );


    // get the center based on the sin and cos in range 0 - 1
    var s = (sin(u.time*0.3) + 1.0) / 2.0;
    var c = (cos(u.time*0.1) + 1.0) / 2.0;
    let radius: f32 = 0.9;
    s = s * radius ;
    c = c * radius ;

    let center = vec2<f32>(c, s);
    let d1 = distance(center, uv );
    let distance = distance(center, uv ) * fract(sin(uv.y) * cos(uv.x));
    let idx = (distance * 255.0) ;
    let slot = u32(idx) / 4u ;
    let offset = u32(2.0 * idx ) % 4u ;
    let f = u.freqs[slot][offset];

    let c2 = vec2<f32>(s, c);
    let d2 = distance(c2, uv );
    let distance_y = distance(c2, uv ) * fract(sin(uv.x) * cos(uv.y));
    let idx_y = (distance_y * 255.0) ;
    let slot_y = u32(idx_y) / 4u ;
    let offset_y = u32(2.0 *  idx_y ) % 4u ;
    let fy = u.freqs[slot_y][offset_y];

    let color = vec3(abs(sin(f + d2  - fy * d1)), (fy + f * d1) / 2.0, abs(fract((sin(fy + f + d1 + d2)))));
    //let color = vec3(abs(sin(f * (distance_y) )), abs(sin(fy  ( distance) )), abs((sin(distance_y - distance))));



    return vec4(color, 0.8);
}
