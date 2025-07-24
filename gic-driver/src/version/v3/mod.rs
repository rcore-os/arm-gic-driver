use core::ptr::NonNull;

use aarch64_cpu::{
    asm::barrier,
    registers::{CurrentEL, MPIDR_EL1},
};
use log::*;
use tock_registers::{LocalRegisterCopy, interfaces::*};

macro_rules! cpu_read {
    ($name: expr) => {{
        let x: usize;
        unsafe {
            core::arch::asm!(concat!("mrs {}, ", $name), out(reg) x);
        }
        x
    }};
}

macro_rules! cpu_write {
    ($name: expr, $value: expr) => {{
        let x = $value;
        unsafe {
            core::arch::asm!(concat!("msr ", $name, ", {0:x}"), in(reg) x);
        }
    }};
}

pub mod gicc;
mod gicd;
mod gicr;

use crate::{VirtAddr, v3::gicc::enable_group1};
use gicd::*;
use gicr::*;

const GICC_SRE_SRE: usize = 1 << 0;
const GICC_SRE_DFB: usize = 1 << 1;
const GICC_SRE_DIB: usize = 1 << 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CPUTarget {
    pub aff0: u8,
    pub aff1: u8,
    pub aff2: u8,
    pub aff3: u8,
}

impl CPUTarget {
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
        let cpu = CPUTarget::current();
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
            let mut reg = cpu_read!("ICC_SRE_EL2");
            reg |= GICC_SRE_SRE | GICC_SRE_DFB | GICC_SRE_DIB;
            cpu_write!("ICC_SRE_EL2", reg);
        } else {
            let mut reg = cpu_read!("ICC_SRE_EL1");
            if (reg & GICC_SRE_SRE) == 0 {
                reg |= GICC_SRE_SRE | GICC_SRE_DFB | GICC_SRE_DIB;
                cpu_write!("ICC_SRE_EL1", reg);
            }
        }

        // 4. Set interrupt priority mask to allow all priorities
        cpu_write!("ICC_PMR_EL1", 0xFF);

        // 5. Enable Group 1 interrupts
        enable_group1();

        // 6. Configure control register based on security state
        match self.security_state {
            SecurityState::Single => {
                // In single security state, use CBPR (Common Binary Point Register)
                const GICC_CTLR_CBPR: usize = 1 << 0;
                cpu_write!("ICC_CTLR_EL1", GICC_CTLR_CBPR);
            }
            SecurityState::Secure => {
                // In secure state, don't set CBPR to allow separate binary point registers
                cpu_write!("ICC_CTLR_EL1", 0);
            }
            SecurityState::NonSecure => {
                // In non-secure state, use CBPR
                const GICC_CTLR_CBPR: usize = 1 << 0;
                cpu_write!("ICC_CTLR_EL1", GICC_CTLR_CBPR);
            }
        }

        trace!("CPU interface initialized successfully");
        Ok(())
    }
}
