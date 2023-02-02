// Vertex Shader
// ?? Places the verts in an isosoles triangle shape according to the index?

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32> // TODO: remind yourself exactly what clip space is
};

@vertex
fn vert_main(
    @builtin(vertex_index) curr_vert_index:u32,
) -> VertexOutput {
    var return_data:VertexOutput;
    let x = f32(1 - i32(curr_vert_index)) * 0.5;
    let y = f32(i32(curr_vert_index & 1u) * 2 - 1) * 0.5; // ?? wtf is & 1u ??
    return_data.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return return_data;
}

// In combo with the vert shader, creates triangle of arbitrary returned
@fragment
fn frag_main(vert_data: VertexOutput) -> @location(0) vec4<f32> {
    // return this arbitrary color, 
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}