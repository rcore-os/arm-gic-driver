#![no_std]

extern crate alloc;
extern crate core;

pub(crate) mod define;
mod version;
// pub mod gic;
// pub(crate) mod register;

pub use define::{CPUTarget, GicGeneric, IntId, SGITarget, Trigger, MPID};
// pub use gic::{Config, Gic, IrqConfig};

pub use version::{v2::GicV2, v3::GicV3};
