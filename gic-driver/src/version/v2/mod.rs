mod reg;
use core::ptr::NonNull;

use reg::*;


/// GICv2 driver. (support GICv1)
pub struct Gic {
    gicd: NonNull<Distributor>,
    gicc: NonNull<CpuInterface>,
}

unsafe impl Send for Gic {}

impl Gic {
    /// `gicd`: Distributor register base address. `gicc`: CPU interface register base address.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided pointers are valid and point to the correct GICv2 registers.
    pub const unsafe fn new(gicd: *mut u8, gicc: *mut u8) -> Self {
        Self {
            gicd: unsafe { NonNull::new_unchecked(gicd as _) },
            gicc: unsafe { NonNull::new_unchecked(gicc as _) },
        }
    }

    fn gicd(&self) -> &Distributor {
        unsafe { self.gicd.as_ref() }
    }
}