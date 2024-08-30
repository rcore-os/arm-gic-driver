#![no_std]

extern crate alloc;
extern crate core;

pub(crate) mod define;
pub mod gic;
pub(crate) mod register;

pub use define::{CPUTarget, IntId, SGITarget, Trigger, MPID};
pub use gic::{Config, Gic, IrqConfig};
