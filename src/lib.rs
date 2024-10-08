#![no_std]

extern crate alloc;
extern crate core;

#[cfg(test)]
mod tests;
pub(crate) mod define;
mod version;

pub use define::{CPUTarget, GicGeneric, IntId, SGITarget, Trigger, MPID};
pub use version::{v2::GicV2, v3::GicV3};
