#![no_std]
#![doc = include_str!("../../README.md")]

pub(crate) mod define;
pub mod sys_reg;

#[cfg(test)]
mod tests;
mod version;

use core::{
    fmt::{Debug, Display},
    ptr::NonNull,
};

pub use define::IntId;
pub use version::*;

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtAddr(usize);

impl VirtAddr {
    /// Create a new `VirtAddr` from a raw pointer.
    pub const fn new(val: usize) -> Self {
        Self(val)
    }

    /// Get the raw pointer as a `*mut u8`.
    pub const fn as_ptr<T>(&self) -> *mut T {
        self.0 as *mut T
    }
}

impl From<usize> for VirtAddr {
    fn from(addr: usize) -> Self {
        Self(addr)
    }
}

impl From<VirtAddr> for usize {
    fn from(addr: VirtAddr) -> Self {
        addr.0
    }
}

impl From<*mut u8> for VirtAddr {
    fn from(addr: *mut u8) -> Self {
        Self(addr as usize)
    }
}

impl<T> From<NonNull<T>> for VirtAddr {
    fn from(addr: NonNull<T>) -> Self {
        Self(addr.as_ptr() as usize)
    }
}

impl Display for VirtAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "VirtAddr({:#p})", self.0 as *const u8)
    }
}
