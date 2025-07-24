use log::trace;
use tock_registers::{interfaces::*, LocalRegisterCopy};

mod gicc;
mod gicd;
mod gich;
mod gicv;

use gicc::CpuInterfaceReg;
use gicd::DistributorReg;
use gich::HypervisorRegs;
use gicv::VirtualCpuInterfaceReg;

use crate::{
    version::{IrqVecReadable, IrqVecWriteable},
    IntId,
};

/// GICv2 driver. (support GICv1)
pub struct Gic {
    gicd: *mut DistributorReg,
    gicc: *mut CpuInterfaceReg,
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
            gicd: gicd as _,
            gicc: gicc as _,
        }
    }

    fn gicd(&self) -> &DistributorReg {
        unsafe { &*self.gicd }
    }

    pub fn init_cpu_interface(&self) -> CpuInterface {
        let mut c = CpuInterface {
            gicd: self.gicd,
            gicc: self.gicc,
        };
        c.init();
        c
    }

    /// Initialize the GIC according to GICv2 specification
    /// This includes both Distributor and CPU Interface initialization
    pub fn init(&mut self) {
        trace!("Initializing GICv2 Distributor@{:#p}...", self.gicd);
        // 1. Disable the Distributor first
        self.gicd().disable();

        // 2. Get the number of interrupt lines supported
        let max_spi = self.gicd().max_spi_num();

        // 3. Disable all interrupts first
        self.gicd().irq_disable_all(max_spi);

        // 4. Clear all pending interrupts
        self.gicd().pending_clear_all(max_spi);

        // 5. Clear all active interrupts
        self.gicd().active_clear_all(max_spi);

        // 6. Configure all interrupts as Group 1 (Non-secure) by default
        self.gicd().groups_all_to_0(max_spi);
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
        let index = id.to_u32() as usize;
        assert!(
            index < self.gicd().IPRIORITYR.len(),
            "Invalid interrupt ID for priority: {id:?}"
        );
        self.gicd().IPRIORITYR[index].set(priority);
    }

    pub fn get_priority(&self, id: IntId) -> u8 {
        let index = id.to_u32() as usize;
        assert!(
            index < self.gicd().IPRIORITYR.len(),
            "Invalid interrupt ID for priority: {id:?}"
        );
        self.gicd().IPRIORITYR[index].get()
    }

    /// Set interrupt target CPU for SPIs
    pub fn set_target_cpu(&self, id: IntId, target_list: TargetList) {
        assert!(
            !id.is_private(),
            "Cannot set target CPU for private interrupt: {id:?}"
        );
        let index = id.to_u32() as usize;
        assert!(
            index < self.gicd().ITARGETSR.len(),
            "Invalid interrupt ID for target: {id:?}"
        );
        self.gicd().ITARGETSR[index].set(target_list.as_u8());
    }

    pub fn get_target_cpu(&self, id: IntId) -> TargetList {
        assert!(
            !id.is_private(),
            "Cannot get target CPU for private interrupt: {id:?}"
        );
        let index = id.to_u32() as usize;
        assert!(
            index < self.gicd().ITARGETSR.len(),
            "Invalid interrupt ID for target: {id:?}"
        );
        TargetList(self.gicd().ITARGETSR[index].get())
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
            SGITarget::TargetList(list) => (
                gicd::SGIR::TargetListFilter::TargetList,
                list.as_u8() as u32,
            ),
            SGITarget::AllOther => (gicd::SGIR::TargetListFilter::AllOther, 0),
            SGITarget::Current => (gicd::SGIR::TargetListFilter::Current, 0),
        };

        self.gicd().SGIR.write(
            gicd::SGIR::SGIINTID.val(sgi_id) + gicd::SGIR::CPUTargetList.val(target_list) + filter,
        );
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
            self.gicd().ISPENDR.set_irq_bit(id.into());
        } else {
            self.gicd().ICPENDR.set_irq_bit(id.into());
        }
    }

    pub fn is_pending(&self, id: IntId) -> bool {
        self.gicd().ISPENDR.get_irq_bit(id.into())
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

    pub fn cpu_id_list(&self) -> impl Iterator<Item = usize> {
        (0..8).filter(move |i| (self.0 & (1 << i)) != 0)
    }
}

impl SGITarget {
    /// Create a new SGITarget with a specific CPU target list. list is Cpu interface IDs.
    pub fn new_target_list(val: TargetList) -> Self {
        Self::TargetList(val)
    }
}
#[derive(Debug, Clone, Copy)]
pub enum Ack {
    Normal(IntId),
    SGI { intid: IntId, cpu_id: usize },
}

impl From<Ack> for u32 {
    fn from(ack: Ack) -> Self {
        match ack {
            Ack::Normal(intid) => gicc::IAR::InterruptID.val(intid.to_u32()),
            Ack::SGI { intid, cpu_id } => {
                gicc::IAR::InterruptID.val(intid.to_u32()) + gicc::IAR::CPUID.val(cpu_id as u32)
            }
        }
        .value
    }
}

impl From<u32> for Ack {
    fn from(value: u32) -> Self {
        let reg = LocalRegisterCopy::<u32, gicc::IAR::Register>::new(value);
        let intid = unsafe { IntId::raw(reg.read(gicc::IAR::InterruptID)) };
        if intid.is_sgi() {
            let cpu_id = reg.read(gicc::IAR::CPUID) as usize;
            Ack::SGI { intid, cpu_id }
        } else {
            Ack::Normal(intid)
        }
    }
}

pub struct CpuInterface {
    gicd: *mut DistributorReg,
    gicc: *mut CpuInterfaceReg,
}

unsafe impl Send for CpuInterface {}

impl CpuInterface {
    fn gicc(&self) -> &CpuInterfaceReg {
        unsafe { &*self.gicc }
    }

    fn gicd(&self) -> &DistributorReg {
        unsafe { &*self.gicd }
    }

    fn init(&mut self) {
        let gicc = self.gicc();

        // 1. Disable CPU interface first
        gicc.CTLR.set(0);

        // 2. Set priority mask to allow all interrupts (lowest priority)
        gicc.PMR.write(gicc::PMR::Priority.val(0xFF));

        // // 3. Set binary point to default value (no preemption)
        // gicc.BPR.write(BPR::BinaryPoint.val(0x2));

        // // 4. Set aliased binary point for Group 1 interrupts
        // gicc.ABPR.write(ABPR::BinaryPoint.val(0x3));

        // 5. Enable CPU interface for both Group 0 and Group 1 interrupts
        gicc.CTLR.write(gicc::CTLR::EnableGrp0::SET);
    }
    /// Set the EOI mode for non-secure interrupts
    ///
    /// - `false` GICC_EOIR has both priority drop and deactivate interrupt functionality. Accesses to the GICC_DIR are UNPREDICTABLE.
    /// - `true`  GICC_EOIR has priority drop functionality only. GICC_DIR has deactivate interrupt functionality.
    pub fn set_eoi_mode_ns(&self, is_two_step: bool) {
        if is_two_step {
            self.gicc().CTLR.modify(gicc::CTLR::EOImodeNS::SET);
        } else {
            self.gicc().CTLR.modify(gicc::CTLR::EOImodeNS::CLEAR);
        };
    }

    pub fn eoi_mode_ns(&self) -> bool {
        self.gicc().CTLR.is_set(gicc::CTLR::EOImodeNS)
    }

    /// Acknowledge an interrupt and return the interrupt ID
    /// Returns the interrupt ID and source CPU ID (for SGIs)
    pub fn ack(&self) -> Option<Ack> {
        let data = self.gicc().IAR.extract();
        let id = data.read(gicc::IAR::InterruptID);
        if id == 1023 {
            return None;
        }
        Some(data.get().into())
    }

    /// Signal end of interrupt processing
    pub fn eoi(&self, ack: Ack) {
        let val = match ack {
            Ack::Normal(intid) => gicc::EOIR::EOIINTID.val(intid.to_u32()),
            Ack::SGI { intid, cpu_id } => {
                gicc::EOIR::EOIINTID.val(intid.to_u32()) + gicc::EOIR::CPUID.val(cpu_id as u32)
            }
        };
        self.gicc().EOIR.write(val);
    }

    /// Deactivate an interrupt
    pub fn dir(&self, ack: Ack) {
        let val = match ack {
            Ack::Normal(intid) => gicc::DIR::InterruptID.val(intid.to_u32()),
            Ack::SGI { intid, cpu_id } => {
                gicc::DIR::InterruptID.val(intid.to_u32()) + gicc::DIR::CPUID.val(cpu_id as u32)
            }
        };
        self.gicc().DIR.write(val);
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
        self.gicc().PMR.write(gicc::PMR::Priority.val(mask as u32));
    }

    /// Enable a specific interrupt
    pub fn irq_enable(&self, id: IntId) {
        assert!(
            id.is_private(),
            "Cannot enable non-private interrupt: {id:?}"
        );
        self.gicd().ISENABLER.set_irq_bit(id.into());
    }

    /// Disable a specific interrupt
    pub fn irq_disable(&self, id: IntId) {
        assert!(
            id.is_private(),
            "Cannot disable non-private interrupt: {id:?}"
        );
        self.gicd().ICENABLER.set_irq_bit(id.into());
    }

    pub fn irq_is_enabled(&self, id: IntId) -> bool {
        assert!(
            id.is_private(),
            "Cannot check non-private interrupt: {id:?}"
        );
        self.gicd().ISENABLER.get_irq_bit(id.into())
    }

    /// Set interrupt priority (0 = highest priority, 255 = lowest priority)
    pub fn set_priority(&self, id: IntId, priority: u8) {
        assert!(
            id.is_private(),
            "Cannot set priority for non-private interrupt: {id:?}"
        );
        let index = id.to_u32() as usize;
        assert!(
            index < self.gicd().IPRIORITYR.len(),
            "Invalid interrupt ID for priority: {id:?}"
        );
        self.gicd().IPRIORITYR[index].set(priority);
    }

    pub fn get_priority(&self, id: IntId) -> u8 {
        assert!(
            id.is_private(),
            "Cannot get priority for non-private interrupt: {id:?}"
        );
        let index = id.to_u32() as usize;
        assert!(
            index < self.gicd().IPRIORITYR.len(),
            "Invalid interrupt ID for priority: {id:?}"
        );
        self.gicd().IPRIORITYR[index].get()
    }

    pub fn set_active(&self, id: IntId, active: bool) {
        assert!(
            id.is_private(),
            "Cannot set active state for non-private interrupt: {id:?}"
        );
        if active {
            self.gicd().ISACTIVER.set_irq_bit(id.into());
        } else {
            self.gicd().ICACTIVER.set_irq_bit(id.into());
        }
    }

    pub fn is_active(&self, id: IntId) -> bool {
        assert!(
            id.is_private(),
            "Cannot check active state for non-private interrupt: {id:?}"
        );
        self.gicd().ISACTIVER.get_irq_bit(id.into())
    }

    pub fn set_pending(&self, id: IntId, pending: bool) {
        assert!(
            id.is_private(),
            "Cannot set pending state for non-private interrupt: {id:?}"
        );
        if pending {
            self.gicd().ISPENDR.set_irq_bit(id.into());
        } else {
            self.gicd().ICPENDR.set_irq_bit(id.into());
        }
    }

    pub fn is_pending(&self, id: IntId) -> bool {
        assert!(
            id.is_private(),
            "Cannot check pending state for non-private interrupt: {id:?}"
        );
        self.gicd().ISPENDR.get_irq_bit(id.into())
    }
}

/// GIC Hypervisor Interface for virtualization support
pub struct HypervisorInterface {
    gich: *mut HypervisorRegs,
}

unsafe impl Send for HypervisorInterface {}

impl HypervisorInterface {
    /// Create a new HypervisorInterface
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided pointer is valid and points to the correct GICH registers.
    pub const unsafe fn new(gich: *mut u8) -> Self {
        Self { gich: gich as _ }
    }

    fn gich(&self) -> &HypervisorRegs {
        unsafe { &*self.gich }
    }

    /// Initialize the hypervisor interface
    pub fn init(&mut self) {
        let gich = self.gich();

        // Disable the hypervisor interface first
        gich.HCR.set(0);

        // Clear all list registers
        for lr in &gich.LR {
            lr.set(0);
        }

        // Clear active priorities
        gich.APR.set(0);
    }

    /// Enable the virtual CPU interface
    pub fn enable(&self) {
        self.gich().HCR.modify(gich::HCR::En::SET);
    }

    /// Disable the virtual CPU interface
    pub fn disable(&self) {
        self.gich().HCR.modify(gich::HCR::En::CLEAR);
    }

    /// Set a maintenance interrupt enable
    pub fn set_maintenance_interrupt(&self, int_type: MaintenanceInterruptType, enable: bool) {
        match int_type {
            MaintenanceInterruptType::Underflow => {
                if enable {
                    self.gich().HCR.modify(gich::HCR::UIE::SET);
                } else {
                    self.gich().HCR.modify(gich::HCR::UIE::CLEAR);
                }
            }
            MaintenanceInterruptType::ListRegEntryNotPresent => {
                if enable {
                    self.gich().HCR.modify(gich::HCR::LRENPIE::SET);
                } else {
                    self.gich().HCR.modify(gich::HCR::LRENPIE::CLEAR);
                }
            }
            MaintenanceInterruptType::NoPending => {
                if enable {
                    self.gich().HCR.modify(gich::HCR::NPIE::SET);
                } else {
                    self.gich().HCR.modify(gich::HCR::NPIE::CLEAR);
                }
            }
            MaintenanceInterruptType::VGrp0Enable => {
                if enable {
                    self.gich().HCR.modify(gich::HCR::VGrp0EIE::SET);
                } else {
                    self.gich().HCR.modify(gich::HCR::VGrp0EIE::CLEAR);
                }
            }
            MaintenanceInterruptType::VGrp0Disable => {
                if enable {
                    self.gich().HCR.modify(gich::HCR::VGrp0DIE::SET);
                } else {
                    self.gich().HCR.modify(gich::HCR::VGrp0DIE::CLEAR);
                }
            }
            MaintenanceInterruptType::VGrp1Enable => {
                if enable {
                    self.gich().HCR.modify(gich::HCR::VGrp1EIE::SET);
                } else {
                    self.gich().HCR.modify(gich::HCR::VGrp1EIE::CLEAR);
                }
            }
            MaintenanceInterruptType::VGrp1Disable => {
                if enable {
                    self.gich().HCR.modify(gich::HCR::VGrp1DIE::SET);
                } else {
                    self.gich().HCR.modify(gich::HCR::VGrp1DIE::CLEAR);
                }
            }
        }
    }

    /// Set a virtual interrupt in a list register
    pub fn set_virtual_interrupt(
        &self,
        lr_index: usize,
        config: VirtualInterruptConfig,
    ) -> Result<(), &'static str> {
        if lr_index >= 64 {
            return Err("Invalid list register index");
        }

        let mut lr_val = gich::LR::VirtualID.val(config.virtual_id.to_u32())
            + gich::LR::Priority.val(config.priority as u32)
            + gich::LR::State.val(config.state as u32);

        if config.group1 {
            lr_val += gich::LR::Grp1::SET;
        }

        if config.hw_interrupt {
            lr_val += gich::LR::HW::SET + gich::LR::PhysicalID.val(config.physical_id.unwrap_or(0));
        } else {
            if let Some(cpu_id) = config.cpu_id {
                lr_val += gich::LR::CPUID.val(cpu_id as u32);
            }
            if config.eoi_maintenance {
                lr_val += gich::LR::EOI::SET;
            }
        }

        self.gich().LR[lr_index].write(lr_val);
        Ok(())
    }

    /// Get the maintenance interrupt status
    pub fn get_maintenance_status(&self) -> u32 {
        self.gich().MISR.get()
    }

    /// Get the number of implemented list registers
    pub fn get_list_register_count(&self) -> usize {
        (self.gich().VTR.read(gich::VTR::ListRegs) + 1) as usize
    }

    /// Get EOI status registers
    pub fn get_eoi_status(&self) -> (u32, u32) {
        (self.gich().EISR0.get(), self.gich().EISR1.get())
    }

    /// Get empty list register status
    pub fn get_empty_lr_status(&self) -> (u32, u32) {
        (self.gich().ELRSR0.get(), self.gich().ELRSR1.get())
    }
}

/// GIC Virtual CPU Interface for guest VMs
pub struct VirtualCpuInterface {
    gicv: *mut VirtualCpuInterfaceReg,
}

unsafe impl Send for VirtualCpuInterface {}

impl VirtualCpuInterface {
    /// Create a new VirtualCpuInterface
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided pointer is valid and points to the correct GICV registers.
    pub const unsafe fn new(gicv: *mut u8) -> Self {
        Self { gicv: gicv as _ }
    }

    fn gicv(&self) -> &VirtualCpuInterfaceReg {
        unsafe { &*self.gicv }
    }

    /// Initialize the virtual CPU interface
    pub fn init(&mut self) {
        let gicv = self.gicv();

        // Disable the interface first
        gicv.CTLR.set(0);

        // Set priority mask to allow all interrupts
        gicv.PMR.write(gicv::PMR::Priority.val(0x1F)); // 5 bits for virtual interface

        // Set binary point to default
        gicv.BPR.write(gicv::BPR::BinaryPoint.val(0x2));
        gicv.ABPR.write(gicv::ABPR::BinaryPoint.val(0x3));

        // Enable both Group 0 and Group 1 virtual interrupts
        gicv.CTLR
            .write(gicv::CTLR::EnableGrp0::SET + gicv::CTLR::EnableGrp1::SET);
    }

    /// Set the EOI mode for virtual interrupts
    pub fn set_eoi_mode(&self, is_two_step: bool) {
        if is_two_step {
            self.gicv().CTLR.modify(gicv::CTLR::EOImode::SET);
        } else {
            self.gicv().CTLR.modify(gicv::CTLR::EOImode::CLEAR);
        }
    }

    /// Acknowledge a virtual interrupt
    pub fn ack(&self) -> Option<VirtualAck> {
        let data = self.gicv().IAR.extract();
        let id = data.read(gicv::IAR::InterruptID);
        if id == 1023 {
            return None;
        }

        let intid = unsafe { IntId::raw(id) };
        if intid.is_sgi() {
            let cpu_id = data.read(gicv::IAR::CPUID) as usize;
            Some(VirtualAck::SGI { intid, cpu_id })
        } else {
            Some(VirtualAck::Normal(intid))
        }
    }

    /// Signal end of virtual interrupt processing
    pub fn eoi(&self, ack: VirtualAck) {
        let val = match ack {
            VirtualAck::Normal(intid) => gicv::EOIR::EOIINTID.val(intid.to_u32()),
            VirtualAck::SGI { intid, cpu_id } => {
                gicv::EOIR::EOIINTID.val(intid.to_u32()) + gicv::EOIR::CPUID.val(cpu_id as u32)
            }
        };
        self.gicv().EOIR.write(val);
    }

    /// Deactivate a virtual interrupt
    pub fn dir(&self, ack: VirtualAck) {
        let val = match ack {
            VirtualAck::Normal(intid) => gicv::DIR::InterruptID.val(intid.to_u32()),
            VirtualAck::SGI { intid, cpu_id } => {
                gicv::DIR::InterruptID.val(intid.to_u32()) + gicv::DIR::CPUID.val(cpu_id as u32)
            }
        };
        self.gicv().DIR.write(val);
    }

    /// Get the highest priority pending virtual interrupt
    pub fn get_highest_priority_pending(&self) -> u32 {
        self.gicv().HPPIR.get() & 0x3FF
    }

    /// Get the current virtual running priority
    pub fn get_running_priority(&self) -> u8 {
        (self.gicv().RPR.get() & 0xFF) as u8
    }

    /// Set the virtual priority mask
    pub fn set_priority_mask(&self, mask: u8) {
        // Only bits [7:3] are used in virtual interface
        self.gicv()
            .PMR
            .write(gicv::PMR::Priority.val((mask >> 3) as u32));
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MaintenanceInterruptType {
    Underflow,
    ListRegEntryNotPresent,
    NoPending,
    VGrp0Enable,
    VGrp0Disable,
    VGrp1Enable,
    VGrp1Disable,
}

#[derive(Debug, Clone, Copy)]
pub struct VirtualInterruptConfig {
    pub virtual_id: IntId,
    pub priority: u8,
    pub state: VirtualInterruptState,
    pub group1: bool,
    pub hw_interrupt: bool,
    pub physical_id: Option<u32>,
    pub cpu_id: Option<usize>,
    pub eoi_maintenance: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum VirtualInterruptState {
    Invalid = 0,
    Pending = 1,
    Active = 2,
    PendingAndActive = 3,
}

#[derive(Debug, Clone, Copy)]
pub enum VirtualAck {
    Normal(IntId),
    SGI { intid: IntId, cpu_id: usize },
}

// Export public items
pub use gich::HypervisorRegs;
pub use gicv::VirtualCpuInterfaceReg;
