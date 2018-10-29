//! This module does the OpenGL stuff.

use gl;
use gl::types::*;
use image::load_image;
use std::error::Error;
use std::mem;
use std::ptr;
use std::sync::Mutex;
use text;

const TEXTURE_COUNT: usize = 2; // UI elements, glyph cache

pub(crate) const DRAW_CALL_INDEX_UI: usize = 0;
pub(crate) const DRAW_CALL_INDEX_TEXT: usize = 1;

type PositionAttribute = (f32, f32, f32);
type TexCoordAttribute = (f32, f32);
type ColorAttribute = (u8, u8, u8, u8);
type TexQuad = [(PositionAttribute, TexCoordAttribute, ColorAttribute); 6];
type Texture = GLuint;
type VertexBufferObject = GLuint;
type VertexArrayObject = GLuint;

#[derive(Clone, Copy, Debug)]
struct ShaderProgram {
    program: GLuint,
    projection_matrix_location: GLint,
    position_attrib_location: GLuint,
    texcoord_attrib_location: GLuint,
    color_attrib_location: GLuint,
}

#[derive(Clone, Debug)]
struct Attributes {
    vbo: VertexBufferObject,
    vao: VertexArrayObject,
    vbo_data: Vec<TexQuad>,
    allocated_vbo_data_size: isize,
}

#[derive(Clone, Debug)]
struct DrawCall {
    texture: Texture,
    program: ShaderProgram,
    attributes: Attributes,
}

#[derive(Debug)]
struct DrawState {
    calls: Vec<DrawCall>,
    opengl21: bool,
}

lazy_static! {
    static ref DRAW_STATE: Mutex<DrawState> = Mutex::new(DrawState {
        calls: Vec::with_capacity(TEXTURE_COUNT),
        opengl21: true
    });
}

const VERTEX_SHADER_SOURCE: [&str; TEXTURE_COUNT] = [
    include_str!("shaders/texquad.vert"),
    include_str!("shaders/text.vert"),
];
const FRAGMENT_SHADER_SOURCE: [&str; TEXTURE_COUNT] = [
    include_str!("shaders/texquad.frag"),
    include_str!("shaders/text.frag"),
];

const VERTEX_SHADER_SOURCE_210: [&str; TEXTURE_COUNT] = [
    include_str!("shaders/legacy/texquad.vert"),
    include_str!("shaders/legacy/text.vert"),
];
const FRAGMENT_SHADER_SOURCE_210: [&str; TEXTURE_COUNT] = [
    include_str!("shaders/legacy/texquad.frag"),
    include_str!("shaders/legacy/text.frag"),
];

/// Initialize the UI rendering system. Handled by
/// `window_bootstrap`. This must be done after window and context
/// creation, but before any drawing calls.
///
/// `ui_spritesheet_image` should a Vec of the bytes of a .png file
/// with an alpha channel. To load the image at compile-time, you
/// could run the following (of course, with your own path):
/// ```no_run
/// fungui::initialize_renderer(include_bytes!("resources/gui.png"));
/// ```
pub fn initialize_renderer(opengl21: bool, ui_spritesheet_image: &[u8]) -> Result<(), Box<Error>> {
    let mut draw_state = DRAW_STATE.lock().unwrap();
    draw_state.opengl21 = opengl21;

    unsafe {
        if draw_state.opengl21 {
            gl::Enable(gl::TEXTURE_2D);
        }
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    }

    // TODO: Use create_draw_call here and clean up
    for i in 0..TEXTURE_COUNT {
        let program = if draw_state.opengl21 {
            create_program(VERTEX_SHADER_SOURCE_210[i], FRAGMENT_SHADER_SOURCE_210[i])
        } else {
            create_program(VERTEX_SHADER_SOURCE[i], FRAGMENT_SHADER_SOURCE[i])
        };
        let attributes = create_attributes(draw_state.opengl21, program);
        let texture = create_texture();
        let call = DrawCall {
            texture,
            program,
            attributes,
        };
        draw_state.calls.push(call);
    }

    let image = load_image(ui_spritesheet_image).unwrap();
    insert_texture(
        draw_state.calls[DRAW_CALL_INDEX_UI].texture,
        gl::RGBA as GLint,
        image.width,
        image.height,
        &image.pixels,
    );

    // This creates the glyph cache texture
    insert_texture(
        draw_state.calls[DRAW_CALL_INDEX_TEXT].texture,
        gl::RED as GLint,
        text::GLYPH_CACHE_WIDTH as GLint,
        text::GLYPH_CACHE_HEIGHT as GLint,
        &[0; (text::GLYPH_CACHE_WIDTH * text::GLYPH_CACHE_HEIGHT) as usize],
    );

    print_gl_errors("after initialization");
    Ok(())
}

/// Creates a new draw call in the pipeline, and returns its
/// index. Using the index, you can call `draw_quad` to draw sprites
/// from your image. As a rule of thumb, try to minimize the amount of
/// draw calls.
pub fn create_draw_call(image: &[u8]) -> usize {
    let mut draw_state = DRAW_STATE.lock().unwrap();
    let vert = if draw_state.opengl21 {
        VERTEX_SHADER_SOURCE_210[DRAW_CALL_INDEX_UI]
    } else {
        VERTEX_SHADER_SOURCE[DRAW_CALL_INDEX_UI]
    };
    let frag = if draw_state.opengl21 {
        FRAGMENT_SHADER_SOURCE_210[DRAW_CALL_INDEX_UI]
    } else {
        FRAGMENT_SHADER_SOURCE[DRAW_CALL_INDEX_UI]
    };
    let index = draw_state.calls.len();
    let opengl21 = draw_state.opengl21;

    let program = create_program(vert, frag);
    let attributes = create_attributes(opengl21, program);
    let texture = create_texture();
    draw_state.calls.push(DrawCall {
        texture,
        program,
        attributes,
    });

    let image = load_image(image).unwrap();
    insert_texture(
        draw_state.calls[index].texture,
        gl::RGBA as GLint,
        image.width,
        image.height,
        &image.pixels,
    );

    index
}

#[inline]
fn create_program(vert_source: &str, frag_source: &str) -> ShaderProgram {
    let print_shader_error = |shader, shader_type| unsafe {
        let mut compilation_status = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut compilation_status);
        if compilation_status as u8 != gl::TRUE {
            let mut info = [0; 1024];
            gl::GetShaderInfoLog(shader, 1024, ptr::null_mut(), info.as_mut_ptr());
            println!(
                "Shader ({}) compilation failed:\n{}",
                shader_type,
                String::from_utf8_lossy(&mem::transmute::<[i8; 1024], [u8; 1024]>(info)[..])
            );
        }
    };

    let program;
    let projection_matrix_location;
    let position_attrib_location;
    let texcoord_attrib_location;
    let color_attrib_location;
    unsafe {
        program = gl::CreateProgram();

        let vert_shader = gl::CreateShader(gl::VERTEX_SHADER);
        gl::ShaderSource(
            vert_shader,
            1,
            [vert_source.as_ptr() as *const _].as_ptr(),
            [vert_source.len() as GLint].as_ptr(),
        );
        gl::CompileShader(vert_shader);
        print_shader_error(vert_shader, "vertex");

        let frag_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
        gl::ShaderSource(
            frag_shader,
            1,
            [frag_source.as_ptr() as *const _].as_ptr(),
            [frag_source.len() as GLint].as_ptr(),
        );
        gl::CompileShader(frag_shader);
        print_shader_error(frag_shader, "fragment");

        gl::AttachShader(program, vert_shader);
        gl::AttachShader(program, frag_shader);
        gl::LinkProgram(program);
        let mut link_status = 0;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut link_status);
        if link_status as u8 != gl::TRUE {
            let mut info = [0; 1024];
            gl::GetProgramInfoLog(program, 1024, ptr::null_mut(), info.as_mut_ptr());
            println!(
                "Program linking failed:\n{}",
                String::from_utf8_lossy(&mem::transmute::<[i8; 1024], [u8; 1024]>(info)[..])
            );
        }

        gl::UseProgram(program);
        projection_matrix_location =
            gl::GetUniformLocation(program, "projection_matrix\0".as_ptr() as *const _);
        position_attrib_location =
            gl::GetAttribLocation(program, "position\0".as_ptr() as *const _) as GLuint;
        texcoord_attrib_location =
            gl::GetAttribLocation(program, "texcoord\0".as_ptr() as *const _) as GLuint;
        color_attrib_location =
            gl::GetAttribLocation(program, "color\0".as_ptr() as *const _) as GLuint;
    }

    ShaderProgram {
        program,
        projection_matrix_location,
        position_attrib_location,
        texcoord_attrib_location,
        color_attrib_location,
    }
}

#[inline]
fn create_attributes(opengl21: bool, program: ShaderProgram) -> Attributes {
    let mut vao = 0;
    if !opengl21 {
        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);
        }
    }

    let mut vbo = 0;
    unsafe {
        gl::GenBuffers(1, &mut vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
    }

    if !opengl21 {
        unsafe {
            enable_vertex_attribs(program);
        }
    }

    Attributes {
        vao,
        vbo,
        vbo_data: Vec::new(),
        allocated_vbo_data_size: 0,
    }
}

unsafe fn enable_vertex_attribs(program: ShaderProgram) {
    /* Setup the position attribute */
    gl::VertexAttribPointer(
        program.position_attrib_location, /* Attrib location */
        3,                                /* Components */
        gl::FLOAT,                        /* Type */
        gl::FALSE,                        /* Normalize */
        24,                               /* Stride: sizeof(f32) * (Total component count) */
        ptr::null(),                      /* Offset */
    );
    gl::EnableVertexAttribArray(program.position_attrib_location);

    /* Setup the texture coordinate attribute */
    gl::VertexAttribPointer(
        program.texcoord_attrib_location, /* Attrib location */
        2,                                /* Components */
        gl::FLOAT,                        /* Type */
        gl::FALSE,                        /* Normalize */
        24,                               /* Stride: sizeof(f32) * (Total component count) */
        12 as *const _,                   /* Offset: sizeof(f32) * (Position's component count) */
    );
    gl::EnableVertexAttribArray(program.texcoord_attrib_location);

    /* Setup the color attribute */
    gl::VertexAttribPointer(
        program.color_attrib_location, /* Attrib location */
        4,                             /* Components */
        gl::UNSIGNED_BYTE,             /* Type */
        gl::TRUE,                      /* Normalize */
        24,                            /* Stride: sizeof(f32) * (Total component count) */
        20 as *const _,                /* Offset: sizeof(f32) * (Pos + tex component count) */
    );
    gl::EnableVertexAttribArray(program.color_attrib_location);
}

unsafe fn disable_vertex_attribs(program: ShaderProgram) {
    gl::DisableVertexAttribArray(program.position_attrib_location);
    gl::DisableVertexAttribArray(program.texcoord_attrib_location);
    gl::DisableVertexAttribArray(program.color_attrib_location);
}

#[inline]
fn create_texture() -> GLuint {
    let mut tex = 0;
    unsafe {
        gl::GenTextures(1, &mut tex);
        gl::BindTexture(gl::TEXTURE_2D, tex);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
    }
    tex
}

#[inline]
fn insert_texture(tex: GLuint, components: GLint, w: GLint, h: GLint, pixels: &[u8]) {
    unsafe {
        gl::BindTexture(gl::TEXTURE_2D, tex);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            components,
            w,
            h,
            0,
            components as GLuint,
            gl::UNSIGNED_BYTE,
            pixels.as_ptr() as *const _,
        );
    }
}

/// Draws a textured rectangle on the screen.
///
/// - `coords`: The coordinates of the corners of the quad, in
/// (logical) pixels. Arrangement: (left, top, right, bottom)
///
/// - `texcoords`: The texture coordinates (UVs) of the quad, in the
/// range 0.0 - 1.0. Same arrangement as `coords`.
///
/// - `color`: The color tint of the quad, in the range
/// 0-255. Arrangement: (red, green, blue, alpha)
///
/// - `z`: Used for ordering sprites on screen, in the range -1.0 -
/// 1.0. Positive values are in front.
///
/// - `tex_index`: The index of the texture / draw call to draw the
/// quad in. This is the returned value from `create_draw_call`.
pub fn draw_quad(
    coords: (f32, f32, f32, f32),
    texcoords: (f32, f32, f32, f32),
    color: (u8, u8, u8, u8),
    z: f32,
    tex_index: usize,
) {
    let (x0, y0, x1, y1) = coords;
    let (tx0, ty0, tx1, ty1) = texcoords;

    let mut draw_state = DRAW_STATE.lock().unwrap();
    draw_state.calls[tex_index].attributes.vbo_data.push([
        ((x0, y0, z), (tx0, ty0), color),
        ((x1, y0, z), (tx1, ty0), color),
        ((x1, y1, z), (tx1, ty1), color),
        ((x0, y0, z), (tx0, ty0), color),
        ((x1, y1, z), (tx1, ty1), color),
        ((x0, y1, z), (tx0, ty1), color),
    ]);
}

/// Draws a textured rectangle on the screen.
///
/// See docs for `draw_quad`. The only difference is `rotation`, which
/// describes how much the quad is rotated, in radians.
pub fn draw_rotated_quad(
    coords: (f32, f32, f32, f32),
    texcoords: (f32, f32, f32, f32),
    c: (u8, u8, u8, u8),
    z: f32,
    tex_index: usize,
    rotation: f32,
) {
    let cos = rotation.cos();
    let sin = rotation.sin();
    let rotx = |x, y| x * cos - y * sin;
    let roty = |x, y| x * sin + y * cos;

    let (x0, y0, x1, y1) = coords;
    let (cx, cy) = ((x0 + x1) * 0.5, (y0 + y1) * 0.5);
    let (rx0, ry0, rx1, ry1) = (x0 - cx, y0 - cy, x1 - cx, y1 - cy);

    let (x00, y00) = (cx + rotx(rx0, ry0), cy + roty(rx0, ry0));
    let (x10, y10) = (cx + rotx(rx1, ry0), cy + roty(rx1, ry0));
    let (x11, y11) = (cx + rotx(rx1, ry1), cy + roty(rx1, ry1));
    let (x01, y01) = (cx + rotx(rx0, ry1), cy + roty(rx0, ry1));

    let (tx0, ty0, tx1, ty1) = texcoords;

    let mut draw_state = DRAW_STATE.lock().unwrap();
    draw_state.calls[tex_index].attributes.vbo_data.push([
        ((x00, y00, z), (tx0, ty0), c),
        ((x10, y10, z), (tx1, ty0), c),
        ((x11, y11, z), (tx1, ty1), c),
        ((x00, y00, z), (tx0, ty0), c),
        ((x11, y11, z), (tx1, ty1), c),
        ((x01, y01, z), (tx0, ty1), c),
    ]);
}

pub(crate) fn render(width: f32, height: f32) {
    let m00 = 2.0 / width;
    let m11 = -2.0 / height;
    let matrix = [
        m00, 0.0, 0.0, -1.0, 0.0, m11, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];

    text::draw_text();

    let mut draw_state = DRAW_STATE.lock().unwrap();
    let opengl21 = draw_state.opengl21;
    for (i, call) in draw_state.calls.iter_mut().enumerate() {
        if call.attributes.vbo_data.is_empty() {
            continue;
        }

        unsafe {
            gl::UseProgram(call.program.program);
            gl::UniformMatrix4fv(
                call.program.projection_matrix_location,
                1,
                gl::FALSE,
                matrix.as_ptr(),
            );
            if !opengl21 {
                gl::BindVertexArray(call.attributes.vao);
            }
            gl::BindTexture(gl::TEXTURE_2D, call.texture);
            gl::BindBuffer(gl::ARRAY_BUFFER, call.attributes.vbo);
        }

        let buffer_length = (mem::size_of::<TexQuad>() * call.attributes.vbo_data.len()) as isize;
        let buffer_ptr = call.attributes.vbo_data.as_ptr() as *const _;

        if buffer_length < call.attributes.allocated_vbo_data_size {
            unsafe {
                gl::BufferSubData(gl::ARRAY_BUFFER, 0, buffer_length, buffer_ptr);
            }
        } else {
            call.attributes.allocated_vbo_data_size = buffer_length;
            unsafe {
                gl::BufferData(gl::ARRAY_BUFFER, buffer_length, buffer_ptr, gl::STREAM_DRAW);
            }
        }

        if opengl21 {
            unsafe {
                enable_vertex_attribs(call.program);
            }
        }

        unsafe {
            gl::DrawArrays(gl::TRIANGLES, 0, call.attributes.vbo_data.len() as i32 * 6);
        }

        if opengl21 {
            unsafe {
                disable_vertex_attribs(call.program);
            }
        }

        call.attributes.vbo_data.clear();
        print_gl_errors(&*format!("after render #{}", i));
    }
}

pub(crate) fn get_texture(index: usize) -> GLuint {
    let draw_state = DRAW_STATE.lock().unwrap();
    draw_state.calls[index].texture
}

fn print_gl_errors(context: &str) {
    let mut error = unsafe { gl::GetError() };
    while error != gl::NO_ERROR {
        println!("GL error @ {}: {}", context, gl_error_to_string(error));
        error = unsafe { gl::GetError() };
    }
}

fn gl_error_to_string(error: GLuint) -> &'static str {
    match error {
        0x0500 => "GL_INVALID_ENUM",
        0x0501 => "GL_INVALID_VALUE",
        0x0502 => "GL_INVALID_OPERATION",
        0x0503 => "GL_STACK_OVERFLOW",
        0x0504 => "GL_STACK_UNDERFLOW",
        0x0505 => "GL_OUT_OF_MEMORY",
        0x0506 => "GL_INVALID_FRAMEBUFFER_OPERATION",
        0x0507 => "GL_CONTEXT_LOST",
        0x0531 => "GL_TABLE_TOO_LARGE",
        _ => "unknown error",
    }
}
