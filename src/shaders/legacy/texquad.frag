#version 110

varying vec2 frag_texcoord;
varying vec4 frag_color;
uniform sampler2D tex;

void main(void) {
  vec4 out_color;
  if (frag_texcoord.x == -1.0 && frag_texcoord.y == -1.0) {
    out_color = frag_color;
  } else {
    out_color = frag_color * texture2D(tex, frag_texcoord);
  }
  if (out_color.a < 0.01) {
    discard;
  }
  gl_FragColor = out_color;
}
