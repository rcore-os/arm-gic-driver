use core::ptr::NonNull;

use aarch64_cpu::{
    asm::barrier,
    registers::{CurrentEL, MPIDR_EL1},
};
use log::*;
use tock_registers::{LocalRegisterCopy, interfaces::*};

mod gicd;
mod gicr;

use crate::{
    IntId, VirtAddr,
    define::Trigger,
    sys_reg::*,
    version::{IrqVecReadable, IrqVecWriteable},
};
use gicd::*;
use gicr::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Affinity {
    pub aff0: u8,
    pub aff1: u8,
    pub aff2: u8,
    pub aff3: u8,
}

impl Affinity {
    pub(crate) fn affinity(&self) -> u32 {
        self.aff0 as u32
            | ((self.aff1 as u32) << 8)
            | ((self.aff2 as u32) << 16)
            | ((self.aff3 as u32) << 24)
    }
    pub fn from_mpidr(mpidr: u64) -> Self {
        let val = LocalRegisterCopy::<u64, MPIDR_EL1::Register>::new(mpidr);
        Self {
            aff0: val.read(MPIDR_EL1::Aff0) as u8,
            aff1: val.read(MPIDR_EL1::Aff1) as u8,
            aff2: val.read(MPIDR_EL1::Aff2) as u8,
            aff3: val.read(MPIDR_EL1::Aff3) as u8,
        }
    }

    pub fn current() -> Self {
        Self::from_mpidr(MPIDR_EL1.get())
    }
}

/// GICv3 driver.
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

    pub fn max_intid(&self) -> u32 {
        self.gicd().max_intid()
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

    fn rd_slice(&self) -> RDv3Slice {
        RDv3Slice::new(unsafe { NonNull::new_unchecked(self.gicr.as_ptr()) })
    }

    fn current_rd(&self) -> NonNull<RedistributorV3> {
        let want = (MPIDR_EL1.get() & 0xFFF) as u32;

        for rd in self.rd_slice().iter() {
            let affi = unsafe { rd.as_ref() }
                .lpi_ref()
                .TYPER
                .read(gicr::TYPER::Affinity) as u32;
            if affi == want {
                return rd;
            }
        }
        panic!("No current redistributor")
    }

    pub fn cpu_interface(&self) -> CpuInterface {
        CpuInterface {
            rd: self.current_rd().as_ptr(),
            security_state: self.security_state,
        }
    }

    pub fn set_irq_enable(&mut self, intid: IntId, enable: bool) {
        assert!(
            !intid.is_private(),
            "Cannot enable/disable private interrupts directly"
        );

        if enable {
            self.gicd().irq_enable(intid.to_u32());
        } else {
            self.gicd().irq_disable(intid.to_u32());
        }
    }

    pub fn is_irq_enable(&self, id: IntId) -> bool {
        self.gicd().ISENABLER.get_irq_bit(id.into())
    }

    pub fn set_priority(&self, intid: IntId, priority: u8) {
        self.gicd().set_priority(intid.to_u32(), priority);
    }

    pub fn get_priority(&self, intid: IntId) -> u8 {
        self.gicd().get_priority(intid.to_u32())
    }

    pub fn set_active(&self, id: IntId, active: bool) {
        if active {
            self.gicd().ISACTIVER.set_irq_bit(id.into());
        } else {
            self.gicd().ICACTIVER.set_irq_bit(id.into());
        }
    }

    pub fn is_active(&self, id: IntId) -> bool {
        self.gicd().ISACTIVER.get_irq_bit(id.into())
    }

    pub fn set_pending(&self, id: IntId, pending: bool) {
        if pending {
            self.gicd().set_pending(id.into());
        } else {
            self.gicd().clear_pending(id.into());
        }
    }

    pub fn is_pending(&self, id: IntId) -> bool {
        self.gicd().ISPENDR.get_irq_bit(id.into())
    }

    pub fn iidr_raw(&self) -> u32 {
        self.gicd().IIDR.get()
    }

    pub fn typer_raw(&self) -> u32 {
        self.gicd().TYPER.get()
    }

    pub fn set_cfg(&self, id: IntId, cfg: Trigger) {
        let int_num = id.to_u32();
        let reg_index = (int_num / 16) as usize;
        let bit_offset = (int_num % 16) * 2 + 1; // Each interrupt uses 2 bits, we use bit 1 for edge/level

        assert!(
            reg_index < self.gicd().ICFGR.len(),
            "Invalid interrupt ID for config: {id:?}"
        );

        let current = self.gicd().ICFGR[reg_index].get();
        let mask = 1 << bit_offset;

        let new_value = match cfg {
            Trigger::Level => current & !mask, // Clear bit for level-triggered
            Trigger::Edge => current | mask,   // Set bit for edge-triggered
        };

        self.gicd().ICFGR[reg_index].set(new_value);
    }

    pub fn get_cfg(&self, id: IntId) -> Trigger {
        let int_num = id.to_u32();
        let reg_index = (int_num / 16) as usize;
        let bit_offset = (int_num % 16) * 2 + 1; // Each interrupt uses 2 bits, we use bit 1 for edge/level

        assert!(
            reg_index < self.gicd().ICFGR.len(),
            "Invalid interrupt ID for config: {id:?}"
        );

        let current = self.gicd().ICFGR[reg_index].get();
        let mask = 1 << bit_offset;

        if current & mask != 0 {
            Trigger::Edge
        } else {
            Trigger::Level
        }
    }

    /// If `affinity` is `None`, interrupts routed to any PE defined as a participating node.
    pub fn set_target_cpu(&self, id: IntId, affinity: Option<Affinity>) {
        // Only SPIs (Shared Peripheral Interrupts) can have their target CPU set
        // SGIs and PPIs are always private to a specific CPU core
        assert!(
            !id.is_private(),
            "Cannot set target CPU for private interrupt (SGI/PPI): {id:?}"
        );
        self.gicd().set_interrupt_route(id.to_u32(), affinity);
    }

    pub fn get_target_cpu(&self, id: IntId) -> Option<Affinity> {
        // Only SPIs (Shared Peripheral Interrupts) can have their target CPU set
        // SGIs and PPIs are always private to a specific CPU core
        assert!(
            !id.is_private(),
            "Cannot get target CPU for private interrupt (SGI/PPI): {id:?}"
        );
        self.gicd().get_interrupt_route(id.to_u32())
    }

    pub fn max_cpu_num(&self) -> usize {
        self.gicd().max_cpu_num() as _
    }
}

/// Every CPU interface has its own GICC registers
pub struct CpuInterface {
    rd: *mut RedistributorV3,
    security_state: SecurityState,
}

unsafe impl Send for CpuInterface {}

impl CpuInterface {
    fn rd(&self) -> &RedistributorV3 {
        unsafe { &*self.rd }
    }

    /// Initialize the CPU interface for the current CPU
    ///
    /// This follows the GICv3 architecture specification for CPU interface initialization:
    /// 1. Wake up the Redistributor
    /// 2. Initialize SGI/PPI registers to known state
    /// 3. Configure CPU interface registers
    pub fn init_current_cpu(&mut self) -> Result<(), &'static str> {
        let cpu = Affinity::current();
        trace!(
            "CPU interface initialization for CPU: {:#x}",
            cpu.affinity()
        );

        // 1. Wake up the Redistributor first
        self.rd().lpi.wake()?;

        // 2. Initialize SGI/PPI registers with proper sequence
        self.rd().sgi.init_sgi_ppi(self.security_state);

        // Wait for register writes to complete
        self.rd().lpi.wait_for_rwp()?;

        // 3. Configure CPU interface system registers
        if CurrentEL.read(CurrentEL::EL) == 2 {
            ICC_SRE_EL2
                .write(ICC_SRE_EL2::SRE::SET + ICC_SRE_EL2::DFB::SET + ICC_SRE_EL2::DIB::SET);
            ICC_CTLR_EL1.modify(ICC_CTLR_EL1::EOIMODE::SET);
        } else {
            ICC_SRE_EL1
                .write(ICC_SRE_EL1::SRE::SET + ICC_SRE_EL1::DFB::SET + ICC_SRE_EL1::DIB::SET);
        }

        // 4. Set interrupt priority mask to allow all priorities
        ICC_PMR_EL1.write(ICC_PMR_EL1::PRIORITY.val(0xFF));

        // 5. Enable Group 1 interrupts
        ICC_IGRPEN1_EL1.write(ICC_IGRPEN1_EL1::ENABLE::SET);

        // 6. Configure control register based on security state
        match self.security_state {
            SecurityState::Single => {
                // In single security state, use CBPR (Common Binary Point Register)
                ICC_CTLR_EL1.modify(ICC_CTLR_EL1::CBPR::SET);
            }
            SecurityState::Secure => {}
            SecurityState::NonSecure => {
                ICC_CTLR_EL1.modify(ICC_CTLR_EL1::CBPR::SET);
            }
        }

        trace!("CPU interface initialized successfully");
        Ok(())
    }

    /// Set the EOI mode for non-secure interrupts
    ///
    /// - `false` GICC_EOIR has both priority drop and deactivate interrupt functionality. Accesses to the GICC_DIR are UNPREDICTABLE.
    /// - `true`  GICC_EOIR has priority drop functionality only. GICC_DIR has deactivate interrupt functionality.
    pub fn set_eoi_mode(&self, is_two_step: bool) {
        ICC_CTLR_EL1.modify(if is_two_step {
            ICC_CTLR_EL1::EOIMODE::SET
        } else {
            ICC_CTLR_EL1::EOIMODE::CLEAR
        });
    }

    pub fn ack0(&self) -> IntId {
        let raw = ICC_IAR0_EL1.read(ICC_IAR0_EL1::INTID) as u32;
        unsafe { IntId::raw(raw) }
    }

    pub fn ack1(&self) -> IntId {
        let raw = ICC_IAR1_EL1.read(ICC_IAR1_EL1::INTID) as u32;
        unsafe { IntId::raw(raw) }
    }

    pub fn eoi0(&self, ack: IntId) {
        ICC_EOIR0_EL1.write(ICC_EOIR0_EL1::INTID.val(ack.to_u32() as _));
    }

    pub fn eoi1(&self, ack: IntId) {
        ICC_EOIR1_EL1.write(ICC_EOIR1_EL1::INTID.val(ack.to_u32() as _));
    }

    /// Deactivate an interrupt
    pub fn dir(&self, ack: IntId) {
        ICC_DIR_EL1.write(ICC_DIR_EL1::INTID.val(ack.to_u32() as _));
    }

    /// Set the priority mask (interrupts with priority >= mask will be masked)
    pub fn set_priority_mask(&self, mask: u8) {
        ICC_PMR_EL1.write(ICC_PMR_EL1::PRIORITY.val(mask as _));
    }

    pub fn set_irq_enable(&self, id: IntId, enable: bool) {
        assert!(
            id.is_private(),
            "Cannot enable non-private interrupt: {id:?}"
        );
        self.rd().sgi.set_enable_interrupt(id, enable);
    }

    pub fn is_irq_enable(&self, id: IntId) -> bool {
        assert!(
            id.is_private(),
            "Cannot check non-private interrupt: {id:?}"
        );
        self.rd().sgi.is_interrupt_enabled(id)
    }

    /// Set interrupt priority (0 = highest priority, 255 = lowest priority)
    pub fn set_priority(&self, id: IntId, priority: u8) {
        assert!(
            id.is_private(),
            "Cannot set priority for non-private interrupt: {id:?}"
        );

        self.rd().sgi.set_priority(id, priority);
    }

    pub fn get_priority(&self, id: IntId) -> u8 {
        assert!(
            id.is_private(),
            "Cannot get priority for non-private interrupt: {id:?}"
        );
        self.rd().sgi.get_priority(id)
    }

    pub fn set_active(&self, id: IntId, active: bool) {
        assert!(
            id.is_private(),
            "Cannot set active state for non-private interrupt: {id:?}"
        );
        self.rd().sgi.set_active(id, active);
    }

    pub fn is_active(&self, id: IntId) -> bool {
        assert!(
            id.is_private(),
            "Cannot check active state for non-private interrupt: {id:?}"
        );
        self.rd().sgi.is_active(id)
    }

    pub fn set_pending(&self, id: IntId, pending: bool) {
        assert!(
            id.is_private(),
            "Cannot set pending state for non-private interrupt: {id:?}"
        );
        self.rd().sgi.set_pending(id, pending);
    }

    pub fn is_pending(&self, id: IntId) -> bool {
        assert!(
            id.is_private(),
            "Cannot check pending state for non-private interrupt: {id:?}"
        );
        self.rd().sgi.is_pending(id)
    }
}
