pub mod gicc;
mod gicd;
use core::hint::spin_loop;

use aarch64_cpu::asm::barrier;
pub use gicd::*;

use tock_registers::interfaces::*;

use crate::VirtAddr;

/// GICv3 driver. (support GICv1)
pub struct Gic {
    gicd: VirtAddr,
    #[allow(dead_code)]
    gicr: VirtAddr,
}

unsafe impl Send for Gic {}

impl Gic {
    pub fn new(gicd: VirtAddr, gicr: VirtAddr) -> Self {
        Self { gicd, gicr }
    }

    fn reg(&self) -> &DistributorReg {
        unsafe { &*self.gicd.as_ptr() }
    }

    #[allow(dead_code)]
    fn redistributor_reg(&self) -> VirtAddr {
        self.gicr
    }

    fn wait_ctlr(&self) -> Result<(), &'static str> {
        let mut time_out_count = 1000;
        while self.reg().CTLR.is_set(CTLR::RWP) {
            spin_loop();
            time_out_count -= 1;
            if time_out_count == 0 {
                return Err("GICv3 Distributor CTLR RWP wait timeout.");
            }
        }
        barrier::isb(barrier::SY);
        Ok(())
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
        let distributor = self.reg();

        // Check if single security state (DS bit)
        let is_single_security = distributor.is_single_security_state();
        let has_security_ext = distributor.has_security_extensions();

        // 1. Disable all interrupt groups before configuration
        distributor.disable();
        barrier::isb(barrier::SY);

        // Wait for register write to complete
        if let Err(e) = self.wait_ctlr() {
            panic!("Failed to disable GICv3 during init: {}", e);
        }

        // 2. Configure the distributor
        distributor.init();

        // 3. Configure CTLR register based on security state
        if is_single_security {
            // Single security state configuration
            self.init_single_security_state();
        } else {
            // Two security states configuration
            self.init_two_security_states(has_security_ext);
        }

        barrier::isb(barrier::SY);

        // Wait for final configuration to complete
        if let Err(e) = self.wait_ctlr() {
            panic!("Failed to complete GICv3 initialization: {}", e);
        }
    }

    /// Initialize GICv3 for single security state configuration
    ///
    /// When DS=1 (single security state):
    /// - All interrupts are treated as if they belong to a single security state
    /// - Non-secure accesses can modify Group 0 interrupts
    /// - Only EnableGrp0 and EnableGrp1 bits are used
    /// - ARE bit controls affinity routing for the single security state
    fn init_single_security_state(&self) {
        let distributor = self.reg();

        // For single security state:
        // - DS bit is 1 (read-only)
        // - Only EnableGrp0 and EnableGrp1 are used
        // - ARE is used for affinity routing control

        // Enable affinity routing
        distributor.CTLR.modify(CTLR::ARE_NS::Enable);
        barrier::isb(barrier::SY);

        // Enable Group 0 and Group 1 interrupts
        distributor
            .CTLR
            .modify(CTLR::EnableGrp0::SET + CTLR::EnableGrp1NS::SET);
    }

    /// Initialize GICv3 for two security states configuration
    ///
    /// When DS=0 (two security states):
    /// - Secure and Non-secure states are separated
    /// - Group 0 interrupts are always Secure
    /// - Group 1 interrupts can be either Secure (Group 1S) or Non-secure (Group 1NS)
    /// - ARE_S controls affinity routing for Secure state
    /// - ARE_NS controls affinity routing for Non-secure state
    /// - EnableGrp0, EnableGrp1NS, and EnableGrp1S bits control respective interrupt groups
    fn init_two_security_states(&self, has_security_ext: bool) {
        let distributor = self.reg();

        if !has_security_ext {
            // If no security extensions, treat as single security
            self.init_single_security_state();
            return;
        }

        // For two security states (when accessed from Secure state):
        // - DS bit can be programmed (0 for two states, 1 for single state)
        // - EnableGrp0, EnableGrp1NS, EnableGrp1S are available
        // - ARE_S and ARE_NS control affinity routing separately

        // Keep two security states (DS = 0)
        distributor.CTLR.modify(CTLR::DS::TwoSecurityStates);
        barrier::isb(barrier::SY);

        // Enable affinity routing for both security states
        distributor
            .CTLR
            .modify(CTLR::ARE_S::Enable + CTLR::ARE_NS::Enable);
        barrier::isb(barrier::SY);

        // Enable all interrupt groups
        // Group 0 (Secure), Group 1 Non-secure, Group 1 Secure
        distributor
            .CTLR
            .modify(CTLR::EnableGrp0::SET + CTLR::EnableGrp1NS::SET + CTLR::EnableGrp1S::SET);
    }
}
