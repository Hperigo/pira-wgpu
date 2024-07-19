@group(0)
@binding(0)
var dst: texture_storage_2d<rgba32float, write>;

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let max_iterations = 100u;


    let u = f32(global_id.x) / 500.0;
    let v = f32(global_id.y) / 500.0;

    var final_iteration = max_iterations;
    let c = vec2(
        // Translated to put everything nicely in frame.
        u * 3.0 / 1.0 - 1.5,
        v * 3.0 / 1.0 - 1.5
    );
    var current_z = c;
    var next_z: vec2<f32>;
    for (var i = 0u; i < max_iterations; i++) {
        next_z.x = (current_z.x * current_z.x - current_z.y * current_z.y) + c.x;
        next_z.y = (2.0 * current_z.x * current_z.y) + c.y;
        current_z = next_z;
        if length(current_z) * length(current_z) * length(current_z) > 4.0 {
            final_iteration = i;
            break;
        }
    }
    let value = f32(final_iteration) / f32(max_iterations);

    textureStore(dst, global_id.xy, vec4(value, value, value, 1.0));
}