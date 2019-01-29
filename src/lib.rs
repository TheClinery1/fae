//#![warn(missing_docs)]

#![feature(test)]
#[cfg(test)]
extern crate test;
#[cfg(test)]
mod tests;

#[allow(missing_docs, unknown_lints, clippy::all)]
pub mod gl {
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

mod image;
pub mod renderer;

pub use crate::image::Image;

#[cfg(feature = "text")]
pub mod text;

pub mod window;
