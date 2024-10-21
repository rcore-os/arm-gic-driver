#![no_std]
#![doc = include_str!("../README.md")]

extern crate alloc;
extern crate core;

pub(crate) mod define;
#[cfg(test)]
mod tests;
mod version;

pub use define::{CPUTarget, GicGeneric, IntId, SGITarget, Trigger, MPID};
pub use version::{v2::GicV2, v3::GicV3};
