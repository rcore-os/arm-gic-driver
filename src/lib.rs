#![no_std]
#![doc = include_str!("../README.md")]

extern crate alloc;
extern crate core;

pub(crate) mod define;
#[cfg(test)]
mod tests;
mod version;

pub use define::IntId;
pub use rdif_intc::*;
pub use version::*;
