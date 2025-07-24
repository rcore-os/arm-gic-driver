#![no_std]
#![doc = include_str!("../../README.md")]

pub(crate) mod define;
#[cfg(test)]
mod tests;
mod version;

pub use define::IntId;
pub use version::*;

pub type VirtAddr = usize;
