use tock_registers::{
    interfaces::*,
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};

use super::IntId;

register_bitfields! [
    u32,
    IAR [
        INTID OFFSET(0) NUMBITS(10) [],
        CPUID OFFSET(10) NUMBITS(3) []
    ],
    ICPIDR2 [
        ArchRev OFFSET(4) NUMBITS(4) [],
    ],
];

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
