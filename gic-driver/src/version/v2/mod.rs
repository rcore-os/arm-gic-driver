mod reg;

use core::ptr::NonNull;
use log::trace;
use tock_registers::interfaces::*;

use reg::*;

use crate::{
    IntId,
    version::{IrqVecReadable, IrqVecWriteable},
};

/// GICv2 driver. (support GICv1)
pub struct Gic {
    gicd: NonNull<DistributorReg>,
    gicc: NonNull<CpuInterfaceReg>,
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

    fn gicd(&self) -> &DistributorReg {
        unsafe { self.gicd.as_ref() }
    }

    pub fn init_cpu_interface(&self) -> CpuInterface {
        let mut c = CpuInterface { gicc: self.gicc };
        c.init();
        c
    }

    /// Initialize the GIC according to GICv2 specification
    /// This includes both Distributor and CPU Interface initialization
    pub fn init(&mut self) {
        trace!("Initializing GICv2 Distributor...");
        // 1. Disable the Distributor first
        self.gicd().disable();

        // 2. Get the number of interrupt lines supported
        let max_spi = self.gicd().max_spi_num();

        // 3. Disable all interrupts first
        self.gicd().disable_all_interrupts(max_spi);

        // 4. Clear all pending interrupts
        self.gicd().clear_all_pending_interrupts(max_spi);

        // 5. Clear all active interrupts
        self.gicd().clear_all_active_interrupts(max_spi);

        // 6. Configure all interrupts as Group 1 (Non-secure) by default
        self.gicd().configure_interrupt_groups(max_spi);
        trace!("[GICv2] Configure all interrupts as Group 1 (Non-secure) by default");

        // 7. Set default priority for all interrupts
        self.gicd().set_default_priorities(max_spi);

        // 8. Configure interrupt targets (for SPIs)
        self.gicd().configure_interrupt_targets(max_spi);
        trace!("[GICv2] Configure all SPIs to target cpu 0");
        // 9. Configure interrupt configuration (edge/level trigger)
        self.gicd().configure_interrupt_config(max_spi);

        // 10. Enable the Distributor
        self.gicd().enable();
    }

    /// Enable a specific interrupt
    pub fn irq_enable(&self, id: IntId) {
        self.gicd().ISENABLER.set_irq_bit(id.into());
    }

    /// Disable a specific interrupt
    pub fn irq_disable(&self, id: IntId) {
        self.gicd().ICENABLER.set_irq_bit(id.into());
    }

    pub fn irq_is_enabled(&self, id: IntId) -> bool {
        self.gicd().ISENABLER.get_irq_bit(id.into())
    }

    /// Set interrupt priority (0 = highest priority, 255 = lowest priority)
    pub fn set_priority(&self, id: IntId, priority: u8) {
        self.gicd().IPRIORITYR[id.to_u32() as usize].set(priority);
    }

    pub fn get_priority(&self, id: IntId) -> u8 {
        self.gicd().IPRIORITYR[id.to_u32() as usize].get()
    }

    /// Set interrupt target CPU for SPIs
    pub fn set_target_cpu(&self, id: IntId, target_list: TargetList) {
        if id.is_private() {
            return;
        }
        self.gicd().ITARGETSR[id.to_u32() as usize].set(target_list.as_u8());
    }

    /// Configure interrupt as Group 0 (Secure) or Group 1 (Non-secure)
    pub fn set_interrupt_group1(&self, id: IntId, group1: bool) {
        if group1 {
            self.gicd().IGROUPR.set_irq_bit(id.into());
        } else {
            self.gicd().IGROUPR.clear_irq_bit(id.into());
        }
    }

    /// Send a Software Generated Interrupt (SGI) to target CPUs
    ///
    /// # Arguments
    /// * `sgi_id` - SGI interrupt ID (0-15)
    /// * `target` - Target CPUs for the SGI
    pub fn send_sgi(&self, sgi_id: u32, target: SGITarget) {
        assert!(sgi_id < 16, "Invalid SGI ID: {sgi_id}");
        let (filter, target_list) = match target {
            SGITarget::TargetList(list) => {
                (SGIR::TargetListFilter::TargetList, list.as_u8() as u32)
            }
            SGITarget::AllOther => (SGIR::TargetListFilter::AllOther, 0),
            SGITarget::Current => (SGIR::TargetListFilter::Current, 0),
        };

        self.gicd()
            .SGIR
            .write(SGIR::SGIINTID.val(sgi_id) + SGIR::CPUTargetList.val(target_list) + filter);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SGITarget {
    /// Forward to CPUs listed in CPUTargetList (cpu mask)
    TargetList(TargetList),
    /// Forward to all CPUs except the requesting CPU
    AllOther,
    /// Forward only to the requesting CPU
    Current,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct TargetList(u8);

impl TargetList {
    /// Create a new TargetList with a specific CPU target list. list is Cpu interface IDs.
    pub fn new(list: impl Iterator<Item = usize>) -> Self {
        let mut raw = 0;
        for cpu in list {
            assert!(cpu < 8, "Invalid CPU Interface: {cpu}");
            raw |= 1 << cpu; // Set bit for each target CPU
        }
        Self(raw)
    }

    pub fn add(&mut self, cpu: usize) {
        assert!(cpu < 8, "Invalid CPU Interface: {cpu}");
        self.0 |= 1 << cpu; // Set bit for the target CPU
    }

    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl SGITarget {
    /// Create a new SGITarget with a specific CPU target list. list is Cpu interface IDs.
    pub fn new_target_list(val: TargetList) -> Self {
        Self::TargetList(val)
    }
}

pub struct CpuInterface {
    gicc: NonNull<CpuInterfaceReg>,
}

impl CpuInterface {
    fn gicc(&self) -> &CpuInterfaceReg {
        unsafe { self.gicc.as_ref() }
    }

    fn init(&mut self) {
        let gicc = self.gicc();

        // 1. Disable CPU interface first
        gicc.CTLR.set(0);

        // 2. Set priority mask to allow all interrupts (lowest priority)
        gicc.PMR.write(PMR::Priority.val(0xFF));

        // 3. Set binary point to default value (no preemption)
        gicc.BPR.write(BPR::BinaryPoint.val(0x2));

        // 4. Set aliased binary point for Group 1 interrupts
        gicc.ABPR.write(ABPR::BinaryPoint.val(0x3));

        // 5. Enable CPU interface for both Group 0 and Group 1 interrupts
        gicc.CTLR.write(
            GICC_CTLR::EnableGrp0::SET +
            GICC_CTLR::EnableGrp1::SET +
            GICC_CTLR::FIQEn::CLEAR +      // Use IRQ for Group 0 interrupts
            GICC_CTLR::AckCtl::CLEAR, // Separate acknowledge for groups
        );
    }

    /// Acknowledge an interrupt and return the interrupt ID
    /// Returns the interrupt ID and source CPU ID (for SGIs)
    pub fn acknowledge_interrupt(&self) -> (u32, u32) {
        let iar = self.gicc().IAR.get();
        let interrupt_id = iar & 0x3FF; // Bits [9:0]
        let cpu_id = (iar >> 10) & 0x7; // Bits [12:10]
        (interrupt_id, cpu_id)
    }

    /// Signal end of interrupt processing
    pub fn end_of_interrupt(&self, interrupt_id: u32, cpu_id: u32) {
        let eoir_val = interrupt_id | (cpu_id << 10);
        self.gicc().EOIR.set(eoir_val);
    }

    /// Get the highest priority pending interrupt ID
    pub fn get_highest_priority_pending(&self) -> u32 {
        let hppir = self.gicc().HPPIR.get();
        hppir & 0x3FF // Bits [9:0]
    }

    /// Get the current running priority
    pub fn get_running_priority(&self) -> u8 {
        (self.gicc().RPR.get() & 0xFF) as u8
    }

    /// Set the priority mask (interrupts with priority >= mask will be masked)
    pub fn set_priority_mask(&self, mask: u8) {
        self.gicc().PMR.write(PMR::Priority.val(mask as u32));
    }
}
