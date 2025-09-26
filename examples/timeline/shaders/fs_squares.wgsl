struct Uniforms {
    freqs: array<vec4<f32>, 128>, // 512 bins
    time: f32,
    freq: f32,
    height: f32,
    width: f32,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

@fragment
fn main(@builtin(position) in_pos: vec4<f32>) -> @location(0) vec4<f32> {
    let resolution = vec2<f32>(u.width, u.height);
    let uv = in_pos.xy / resolution; // normalized 0..1

    // --- GRID SETTINGS ---
    let gridX = 22.6; // number of columns
    let gridY = 22.6; // number of rows
    let margin = 0.15; // fraction of cell to keep as empty border

    // cell coordinate
    let cell = vec2<f32>(floor(uv.x * gridX), floor(uv.y * gridY));
    let cellIndex = cell.y * gridX + cell.x; // 0..511

    // local position inside cell (0..1)
    let local = fract(uv * vec2<f32>(gridX, gridY));

    // check if we're inside the "square" area (with margin)
    let inside = step(margin, local.x) * step(margin, local.y)
               * step(local.x, 1.0 - margin) * step(local.y, 1.0 - margin);

    // pick frequency value
    let idx = u32(cellIndex);
    let slot = idx / 4u;
    let offset = idx % 4u;
    let f = max(u.freqs[slot][offset], 0.2) - 0.2;

    // color from frequency + time shifting hue
    let hue = fract(cellIndex / (gridX * gridY) );
    let baseColor = vec3<f32>(
        0.5 + 0.5 * sin(6.2831 * (hue + 0.0)),
        0.5 + 0.5 * sin(6.2831 * (hue + 0.33)),
        0.5 + 0.5 * sin(6.2831 * (hue + 0.66))
    );
    let color = baseColor * f*f;

    // mix: black background where not inside the square
    return vec4<f32>(mix(vec3<f32>(0.0), color, inside), 1.0);
}
