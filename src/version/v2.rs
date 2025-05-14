use core::ptr::NonNull;

use rdif_intc::*;
use tock_registers::{register_structs, registers::*};

use super::*;

/// GICv2 driver. (support GICv1)
pub struct Gic {
    gicd: NonNull<Distributor>,
    gicc: NonNull<CpuInterface>,
}

unsafe impl Send for Gic {}

impl Gic {
    /// `gicd`: Distributor register base address. `gicc`: CPU interface register base address.
    pub fn new(gicd: NonNull<u8>, gicc: NonNull<u8>) -> Self {
        Self {
            gicd: gicd.cast(),
            gicc: gicc.cast(),
        }
    }

    fn gicd(&self) -> &Distributor {
        unsafe { self.gicd.as_ref() }
    }
}

impl DriverGeneric for Gic {
    fn open(&mut self) -> DriverResult {
        self.gicd().disable_all_interrupts();
        self.gicd().CTLR.write(CTLR::EnableGrp0::SET);
        Ok(())
    }

    fn close(&mut self) -> DriverResult {
        Ok(())
    }
}

impl Interface for Gic {
    fn irq_enable(&mut self, irq: IrqId) -> Result<(), IntcError> {
        self.gicd().set_enable_interrupt(irq.into(), true);
        Ok(())
    }

    fn irq_disable(&mut self, irq: IrqId) -> Result<(), IntcError> {
        self.gicd().set_enable_interrupt(irq.into(), false);
        Ok(())
    }

    fn set_priority(&mut self, irq: IrqId, priority: usize) -> Result<(), IntcError> {
        self.gicd().set_priority(irq.into(), priority as _);
        Ok(())
    }

    fn set_trigger(&mut self, irq: IrqId, trigger: Trigger) -> Result<(), IntcError> {
        self.gicd().set_cfgr(irq.into(), trigger);
        Ok(())
    }

    fn set_target_cpu(&mut self, irq: IrqId, cpu: CpuId) -> Result<(), IntcError> {
        let target_list = 1u8 << usize::from(cpu);
        self.gicd().set_bind_cpu(irq.into(), target_list);
        Ok(())
    }
    fn capabilities(&self) -> Vec<Capability> {
        alloc::vec![Capability::FdtParseConfig(fdt_parse_irq_config)]
    }

    fn cpu_interface(&self) -> BoxCPU {
        Box::new(GicCpu { ptr: self.gicc })
    }
}

pub struct GicCpu {
    ptr: NonNull<CpuInterface>,
}

unsafe impl Sync for GicCpu {}
unsafe impl Send for GicCpu {}

impl GicCpu {
    fn gicc(&self) -> &CpuInterface {
        unsafe { self.ptr.as_ref() }
    }
}

impl InterfaceCPU for GicCpu {
    fn set_eoi_mode(&self, b: bool) {
        self.gicc()
            .CTLR
            .modify(GICC_CTLR::EOIMODENS.val(if b { 1 } else { 0 }));
    }

    fn get_eoi_mode(&self) -> bool {
        self.gicc().CTLR.is_set(GICC_CTLR::EOIMODENS)
    }

    fn ack(&self) -> Option<IrqId> {
        self.gicc().ack().map(|i| (u32::from(i) as usize).into())
    }

    fn eoi(&self, intid: IrqId) {
        self.gicc().eoi(intid.into())
    }

    fn dir(&self, intid: IrqId) {
        self.gicc().dir(intid.into());
    }

    fn setup(&self) {
        self.gicc().enable();
        self.gicc().set_priority_mask(0xff);
    }

    fn capability(&self) -> CPUCapability {
        CPUCapability::None
    }
}

register_structs! {
    /// GIC CPU Interface registers.
    #[allow(non_snake_case)]
    pub CpuInterface {
        /// CPU Interface Control Register.
        (0x0000 => CTLR: ReadWrite<u32, GICC_CTLR::Register>),
        /// Interrupt Priority Mask Register.
        (0x0004 => PMR: ReadWrite<u32>),
        /// Binary Point Register.
        (0x0008 => BPR: ReadWrite<u32>),
        /// Interrupt Acknowledge Register.
        (0x000c => IAR: ReadOnly<u32, IAR::Register>),
        /// End of Interrupt Register.
        (0x0010 => EOIR: WriteOnly<u32>),
        /// Running Priority Register.
        (0x0014 => RPR: ReadOnly<u32>),
        /// Highest Priority Pending Interrupt Register.
        (0x0018 => HPPIR: ReadOnly<u32>),
        (0x001c => _reserved_1),
        /// CPU Interface Identification Register.
        (0x00fc => IIDR: ReadOnly<u32>),
        (0x0100 => _reserved_2),
        /// Deactivate Interrupt Register.
        (0x1000 => DIR: WriteOnly<u32>),
        (0x1004 => @END),
    }
}

register_bitfields! [
    u32,
    pub GICC_CTLR [
        EnableGrp0 OFFSET(0) NUMBITS(1) [],
        EOIMODENS OFFSET(9) NUMBITS(1) [],
    ],
];

impl CpuInterface {
    pub fn set_priority_mask(&self, priority: u8) {
        self.PMR.set(priority as u32);
    }

    pub fn enable(&self) {
        self.CTLR.write(GICC_CTLR::EnableGrp0::SET);
    }

    pub fn ack(&self) -> Option<IntId> {
        let id = self.IAR.read(IAR::INTID);
        if id == 1023 {
            None
        } else {
            unsafe { Some(IntId::raw(id)) }
        }
    }

    pub fn eoi(&self, intid: IntId) {
        self.EOIR.set(intid.into())
    }

    pub fn dir(&self, intid: IntId) {
        self.DIR.set(intid.into())
    }
}
