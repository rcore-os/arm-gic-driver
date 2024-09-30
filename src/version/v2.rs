use core::ptr::NonNull;

use tock_registers::{interfaces::*, register_bitfields, register_structs, registers::*};

use super::*;

pub struct GicV2 {
    gicd: NonNull<Distributor>,
    gicc: NonNull<CpuInterface>,
}

unsafe impl Send for GicV2 {}
unsafe impl Sync for GicV2 {}

impl GicV2 {
    pub fn new(gicd: NonNull<u8>, gicc: NonNull<u8>) -> Self {
        let mut s = Self {
            gicd: gicd.cast(),
            gicc: gicc.cast(),
        };
        unsafe {
            s.gicd.as_ref().disable_all_interrupts();
            s.gicd.as_ref().CTLR.write(CTLR::EnableGrp0::SET);
        }
        s
    }

    fn gicd(&self) -> &Distributor {
        unsafe { self.gicd.as_ref() }
    }
    fn gicc(&self) -> &CpuInterface {
        unsafe { self.gicc.as_ref() }
    }
}

impl GicGeneric for GicV2 {
    fn get_and_acknowledge_interrupt(&self) -> Option<super::IntId> {
        self.gicc().get_and_acknowledge_interrupt()
    }

    fn end_interrupt(&self, intid: super::IntId) {
        self.gicc().end_interrupt(intid)
    }

    fn irq_max_size(&self) -> usize {
        self.gicd().irq_line_max() as _
    }

    fn irq_disable(&mut self, intid: super::IntId) {
        self.gicd().set_enable_interrupt(intid, false);
    }

    fn current_cpu_setup(&self) {
        self.gicc().enable();
        self.gicc().set_priority_mask(0xff);
    }

    fn irq_enable(&mut self, intid: super::IntId) {
        self.gicd().set_enable_interrupt(intid, true);
    }

    fn set_priority(&mut self, intid: super::IntId, priority: usize) {
        self.gicd().set_priority(intid, priority as _);
    }

    fn set_triger(&mut self, intid: super::IntId, triger: Trigger) {
        self.gicd().set_cfgr(intid, trigger);
    }

    fn set_bind_cpu(&mut self, intid: super::IntId, target_list: &[super::CPUTarget]) {
        self.gicd().set_bind_cpu(
            intid,
            target_list
                .iter()
                .fold(0, |acc, &cpu| acc | cpu.cpu_target_list()),
        );
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
