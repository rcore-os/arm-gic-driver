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
    fn gicc(&self) -> &CpuInterface {
        unsafe { self.gicc.as_ref() }
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
    fn current_cpu_setup(&self) -> HardwareCPU {
        self.gicc().enable();
        self.gicc().set_priority_mask(0xff);
        Box::new(GicCpu { ptr: self.gicc })
    }

    fn irq_enable(&mut self, irq: IrqId) {
        self.gicd().set_enable_interrupt(irq.into(), true);
    }

    fn irq_disable(&mut self, irq: IrqId) {
        self.gicd().set_enable_interrupt(irq.into(), false);
    }

    fn set_priority(&mut self, irq: IrqId, priority: usize) {
        self.gicd().set_priority(irq.into(), priority as _);
    }

    fn set_trigger(&mut self, irq: IrqId, trigger: Trigger) {
        self.gicd().set_cfgr(irq.into(), trigger);
    }

    fn set_target_cpu(&mut self, irq: IrqId, cpu: CpuId) {
        let target_list = 1u8 << usize::from(cpu);
        self.gicd().set_bind_cpu(irq.into(), target_list);
    }
    fn capabilities(&self) -> Vec<Capability> {
        alloc::vec![Capability::FdtParseConfigFn(fdt_parse_irq_config)]
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
    fn get_and_acknowledge_interrupt(&self) -> Option<IrqId> {
        self.gicc()
            .get_and_acknowledge_interrupt()
            .map(|i| (u32::from(i) as usize).into())
    }

    fn end_interrupt(&self, irq: IrqId) {
        self.gicc().end_interrupt(IntId::from(irq))
    }
}

register_structs! {
    /// GIC CPU Interface registers.
    #[allow(non_snake_case)]
    pub CpuInterface {
        /// CPU Interface Control Register.
        (0x0000 => CTLR: ReadWrite<u32>),
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

impl CpuInterface {
    pub fn set_priority_mask(&self, priority: u8) {
        self.PMR.set(priority as u32);
    }

    pub fn enable(&self) {
        self.CTLR.set(1);
    }

    pub fn get_and_acknowledge_interrupt(&self) -> Option<IntId> {
        let id = self.IAR.read(IAR::INTID);
        if id == 1023 {
            None
        } else {
            unsafe { Some(IntId::raw(id)) }
        }
    }

    pub fn end_interrupt(&self, intid: IntId) {
        self.EOIR.set(intid.into())
    }
}
