pub mod gicc;
mod gicd;

use aarch64_cpu::asm::barrier;
pub use gicd::*;

use log::*;
use tock_registers::interfaces::*;

use crate::VirtAddr;

/// GICv3 driver. (support GICv1)
pub struct Gic {
    gicd: VirtAddr,
    #[allow(dead_code)]
    gicr: VirtAddr,
    security_state: SecurityState,
}

unsafe impl Send for Gic {}

impl Gic {
    /// # Safety
    ///
    /// The addresses must be valid.
    pub const unsafe fn new(gicd: VirtAddr, gicr: VirtAddr) -> Self {
        Self {
            gicd,
            gicr,
            security_state: SecurityState::Single,
        }
    }

    fn gicd(&self) -> &DistributorReg {
        unsafe { &*self.gicd.as_ptr() }
    }

    #[allow(dead_code)]
    fn redistributor_reg(&self) -> VirtAddr {
        self.gicr
    }

    /// Initialize the GICv3 Distributor according to ARM GIC Architecture Specification v3/v4
    ///
    /// This function implements the initialization sequence described in section 12.9.4
    /// of the ARM GIC Architecture Specification, supporting different security configurations:
    ///
    /// 1. **Single Security State**: When DS=1, only one security state exists
    ///    - Uses EnableGrp0 and EnableGrp1 bits
    ///    - Uses ARE bit for affinity routing
    ///
    /// 2. **Two Security States**: When DS=0, both Secure and Non-secure states exist
    ///    - Uses EnableGrp0, EnableGrp1NS, and EnableGrp1S bits
    ///    - Uses ARE_S and ARE_NS bits for separate affinity routing control
    ///
    /// The initialization sequence:
    /// 1. Disable all interrupt groups
    /// 2. Wait for register writes to complete (RWP=0)
    /// 3. Initialize distributor registers to known state
    /// 4. Configure CTLR based on security state
    /// 5. Enable affinity routing
    /// 6. Enable appropriate interrupt groups
    pub fn init(&mut self) {
        // Read current configuration to determine security state

        self.security_state = self.gicd().get_security_state();

        trace!(
            "Initializing GICv3 Distributor@{:#p}, security state: {:?}...",
            self.gicd.as_ptr::<u8>(),
            self.security_state
        );

        // 1. Disable all interrupt groups before configuration
        self.disable();
        barrier::isb(barrier::SY);

        // Wait for register write to complete
        if let Err(e) = self.gicd().wait_for_rwp() {
            panic!("Failed to disable GICv3 during init: {}", e);
        }
        trace!("GICv3 Distributor disabled");

        self.gicd().reset_registers();

        let ctrl = match self.security_state {
            SecurityState::Secure => (CTLR_S::EnableGrp1NS::SET + CTLR_S::ARE_NS::SET).value,
            SecurityState::NonSecure => {
                (CTLR_NS::EnableGrp1::SET + CTLR_NS::EnableGrp1A::SET + CTLR_NS::ARE_NS::SET).value
            }
            SecurityState::Single => (CTLR_ONE::EnableGrp1::SET + CTLR_ONE::ARE::SET).value,
        };
        self.gicd().CTLR.set(ctrl);

        barrier::isb(barrier::SY);

        // Wait for final configuration to complete
        if let Err(e) = self.gicd().wait_for_rwp() {
            panic!("Failed to complete GICv3 initialization: {}", e);
        }
    }

    fn disable(&self) {
        let old = self.gicd().CTLR.get();
        let val = match self.security_state {
            SecurityState::Secure => {
                (CTLR_S::EnableGrp0::CLEAR
                    + CTLR_S::EnableGrp1S::CLEAR
                    + CTLR_S::EnableGrp1NS::CLEAR)
                    .value
            }
            SecurityState::NonSecure => {
                (CTLR_NS::EnableGrp1::CLEAR + CTLR_NS::EnableGrp1A::CLEAR).value
            }
            SecurityState::Single => {
                (CTLR_ONE::EnableGrp0::CLEAR + CTLR_ONE::EnableGrp1::CLEAR).value
            }
        };
        self.gicd().CTLR.set(old & !val);
        barrier::isb(barrier::SY);
    }
}
