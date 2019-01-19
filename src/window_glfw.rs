use crate::gl;
use glfw::{Action, ClientApiHint, Context, Key, WindowEvent, WindowHint};
use std::env;
use std::error::Error;
use std::sync::mpsc::Receiver;

pub use crate::window_settings::WindowSettings;
pub use glfw;
pub use scancode::Scancode;

/// Manages the window and propagates events to the UI system.
pub struct Window {
    /// The width of the window.
    pub width: f32,
    /// The height of the window.
    pub height: f32,
    /// The dpi of the window.
    pub dpi_factor: f32,
    glfw: glfw::Glfw,
    glfw_window: glfw::Window,
    events: Receiver<(f64, WindowEvent)>,
    fb_width: f32,
    fb_height: f32,
    /// The opengl legacy status for Renderer.
    pub opengl21: bool,
    /// The keys which are currently held down.
    pub pressed_keys: Vec<Scancode>,
    /// The keys which were pressed this frame.
    pub just_pressed_keys: Vec<Scancode>,
    /// The keys which were released this frame.
    pub released_keys: Vec<Scancode>,
}

impl Window {
    /// Creates a new `Window`.
    ///
    /// Can result in an error if window creation fails or OpenGL
    /// context creation fails.
    pub fn create(settings: &WindowSettings) -> Result<Window, Box<Error>> {
        let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS)?;

        let opengl21 = false;
        let (mut glfw_window, events) = {
            // Forward compatibility flag needed for mac:
            // https://www.khronos.org/opengl/wiki/OpenGL_Context#Forward_compatibility
            if cfg!(target_os = "macos") {
                glfw.window_hint(WindowHint::OpenGlForwardCompat(true));
            }
            glfw.window_hint(WindowHint::SRgbCapable(true));

            let width = settings.width as u32;
            let height = settings.height as u32;
            let title = &settings.title;
            let window_mode = glfw::WindowMode::Windowed;

            if env::var_os("FAE_OPENGL_LEGACY").is_some() {
                if let Some(result) = {
                    glfw.window_hint(WindowHint::ClientApi(ClientApiHint::OpenGl));
                    glfw.window_hint(WindowHint::ContextVersion(2, 1));
                    glfw.create_window(width, height, title, window_mode)
                } {
                    result
                } else if let Some(result) = {
                    glfw.window_hint(WindowHint::ClientApi(ClientApiHint::OpenGlEs));
                    glfw.window_hint(WindowHint::ContextVersion(2, 0));
                    glfw.create_window(width, height, title, window_mode)
                } {
                    result
                } else {
                    return Err(Box::new(WindowCreationError));
                }
            } else {
                if let Some(result) = {
                    glfw.window_hint(WindowHint::ClientApi(ClientApiHint::OpenGl));
                    glfw.window_hint(WindowHint::ContextVersion(3, 3));
                    glfw.create_window(width, height, title, window_mode)
                } {
                    result
                } else if let Some(result) = {
                    glfw.window_hint(WindowHint::ClientApi(ClientApiHint::OpenGlEs));
                    glfw.window_hint(WindowHint::ContextVersion(3, 0));
                    glfw.create_window(width, height, title, window_mode)
                } {
                    result
                } else if let Some(result) = {
                    glfw.window_hint(WindowHint::ClientApi(ClientApiHint::OpenGl));
                    glfw.window_hint(WindowHint::ContextVersion(2, 1));
                    glfw.create_window(width, height, title, window_mode)
                } {
                    result
                } else if let Some(result) = {
                    glfw.window_hint(WindowHint::ClientApi(ClientApiHint::OpenGlEs));
                    glfw.window_hint(WindowHint::ContextVersion(2, 0));
                    glfw.create_window(width, height, title, window_mode)
                } {
                    result
                } else {
                    return Err(Box::new(WindowCreationError));
                }
            }
        };

        glfw_window.make_current();
        gl::load_with(|symbol| glfw_window.get_proc_address(symbol) as *const _);
        /* use std::ffi::CStr;

            Uncomment in case of opengl shenanigans

            let ptr = CStr::from_ptr(gl::GetString(gl::VERSION) as *const _).to_bytes();
            let opengl_version_string = String::from_utf8_lossy(ptr);
            if cfg!(debug_assertions) {
            println!("OpenGL version: {}", opengl_version_string);
        }*/

        if settings.vsync {
            if glfw.extension_supported("WGL_EXT_swap_control_tear")
                || glfw.extension_supported("GLX_EXT_swap_control_tear")
            {
                glfw.set_swap_interval(glfw::SwapInterval::Adaptive);
            } else {
                glfw.set_swap_interval(glfw::SwapInterval::Sync(1));
            }
        } else {
            glfw.set_swap_interval(glfw::SwapInterval::None);
        }

        glfw_window.set_all_polling(true);

        Ok(Window {
            width: settings.width,
            height: settings.height,
            dpi_factor: 1.0,
            glfw,
            glfw_window,
            events,
            fb_width: settings.width,
            fb_height: settings.height,
            opengl21,
            pressed_keys: Vec::new(),
            just_pressed_keys: Vec::new(),
            released_keys: Vec::new(),
        })
    }

    /// Re-renders the window, polls for new events and passes them on
    /// to the UI system, and clears the screen with the
    /// `background_*` colors, which consist of 0.0 - 1.0
    /// values. **Note**: Because of vsync, this function will hang
    /// for a while (usually 16ms at max).
    pub fn refresh(&mut self) -> bool {
        self.glfw_window.swap_buffers();
        let mut resize = false;

        self.just_pressed_keys.clear();
        self.released_keys.clear();
        self.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&self.events) {
            match event {
                WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    self.glfw_window.set_should_close(true)
                }
                WindowEvent::Key(_, scancode, Action::Press, _) if scancode < 0xFF => {
                    if let Some(code) = Scancode::new(scancode as u8) {
                        self.just_pressed_keys.push(code);
                        self.pressed_keys.push(code);
                    }
                }
                WindowEvent::Key(_, scancode, Action::Release, _) if scancode < 0xFF => {
                    if let Some(code) = Scancode::new(scancode as u8) {
                        self.released_keys.push(code);
                        for (i, key) in self.pressed_keys.iter().enumerate() {
                            if key == &code {
                                self.pressed_keys.remove(i);
                                break;
                            }
                        }
                    }
                }
                WindowEvent::Size(width, height) => {
                    self.width = width as f32;
                    self.height = height as f32;
                    resize = true;
                }
                WindowEvent::FramebufferSize(width, height) => {
                    self.fb_width = width as f32;
                    self.fb_height = height as f32;
                    resize = true;
                }
                _ => {}
            }
        }

        /* Resize event handling */
        if resize {
            let dpi_factor_horizontal = self.fb_width / self.width;
            let dpi_factor_vertical = self.fb_height / self.height;
            self.dpi_factor = dpi_factor_horizontal.max(dpi_factor_vertical);

            unsafe {
                gl::Viewport(0, 0, self.fb_width as i32, self.fb_height as i32);
            }
        }

        !self.glfw_window.should_close()
    }
}

#[derive(Debug, Clone)]
struct WindowCreationError;

impl std::fmt::Display for WindowCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "could not create a glfw window")
    }
}

impl Error for WindowCreationError {
    fn description(&self) -> &str {
        "could not create a glfw window"
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}
