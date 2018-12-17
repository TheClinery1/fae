#version 330

in vec3 position;
in vec2 texcoord;
in vec4 color;
in vec3 rotation;
out vec2 frag_texcoord;
out vec4 frag_color;
uniform mat4 projection_matrix;

void main(void) {
  float rot_radians = rotation.x;
  if (rot_radians != 0.0) {
    mat4 rotation_matrix =
      mat4(cos(rot_radians), -sin(rot_radians), 0.0, 0.0,
           sin(rot_radians), cos(rot_radians), 0.0, 0.0,
           0.0, 0.0, 1.0, 0.0,
           0.0, 0.0, 0.0, 1.0);
    vec2 rot_pivot = rotation.yz;
    mat4 pivot_matrix =
      mat4(1.0, 0.0, 0.0, -rot_pivot.x,
           0.0, 1.0, 0.0, -rot_pivot.y,
           0.0, 0.0, 1.0, 0.0,
           0.0, 0.0, 0.0, 1.0);
    mat4 unpivot_matrix =
      mat4(1.0, 0.0, 0.0, rot_pivot.x,
           0.0, 1.0, 0.0, rot_pivot.y,
           0.0, 0.0, 1.0, 0.0,
           0.0, 0.0, 0.0, 1.0);
    gl_Position = vec4(position, 1.0) * pivot_matrix * rotation_matrix * unpivot_matrix * projection_matrix;
  } else {
    gl_Position = vec4(position, 1.0) * projection_matrix;
  }
  frag_texcoord = texcoord;
  frag_color = color;
}
