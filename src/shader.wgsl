// this is the output of Vertex Shader
//      in the context of the fragment shader "@builtin(position)" refers to the framebuffer position of fragment
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color:vec3<f32> // TODO: are we overwriting the vert buffer (position part that is at loc 0) ??
};


// Vertex Shader
// @location(0) is the position of the vert in clip space, written to the vertex buffer in rendering.rs
// @location(1) is the color that we assigned to this vert and wrote to the vertex buffer
@vertex
fn vert_main(
    @builtin(vertex_index) vert_index:u32, // we can use these because they are buffers defined and written to in the configuration of the vert buffers
    @builtin(instance_index) inst_index:u32,
    @location(0) clip_position:vec3<f32>, 
    @location(1) color:vec3<f32>,
    @location(2) instance_pos:vec2<f32>,
) -> VertexOutput {
    var return_data:VertexOutput;
    // write some data to the vertex's position attribute, THIS VALUE WILL BE CHANGED INBETWEEN THE VERT AND FRAG SHADERS
    return_data.position = vec4<f32>(clip_position[0] + instance_pos[0], clip_position[1] + instance_pos[1], 1.0, 1.0);
    return_data.color = color;
    return return_data;
}

@group(0) @binding(0) var<uniform> cursor_pos: vec4<f32>;

// Puts a red circle around the cursor, rest of the plane is the color of the UV position of the fragment
@fragment
fn frag_main(
    vert_data: VertexOutput, 
) -> @location(0) vec4<f32> {
    // We assigned a clip space postion to the @builtin position attribute before, but now it has been transformed
    // into the framebuffer coordinate position of this fragment. This happened INBETWEEN vert and frag stages
    // whereas the color will be a direct interpolation of the value we assigned in the vert shader
    var diff_vec = vert_data.position - cursor_pos;
    var cull = diff_vec[0] > 50.0 || diff_vec[1] > 50.0;
    if cull == false {
        var dist = length(diff_vec);
        // creates a 50px radius red circle around the cursor
        if dist < 50.0 {
            return vec4<f32>(1.0, 0.0, 0.0, 1.0); 
        }
    }

    return vec4<f32>(vert_data.color, 1.0); // print the vertex color
}