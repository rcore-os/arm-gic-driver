pub mod v2;
pub mod v3;

use core::arch::asm;

use tock_registers::{
    interfaces::*,
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};

use crate::define::*;

register_structs! {
    #[allow(non_snake_case)]
    pub Distributor {
        /// Distributor Control Register.
        (0x0000 => pub CTLR: ReadWrite<u32, CTLR::Register>),
        /// Interrupt Controller Type Register.
        (0x0004 => TYPER: ReadOnly<u32, TYPER::Register>),
        /// Distributor Implementer Identification Register.
        (0x0008 => IIDR: ReadOnly<u32, IIDR::Register>),
        (0x000c => _rsv1),
        /// Interrupt Group Registers.
        (0x0080 => IGROUPR: [ReadWrite<u32>; 0x20]),
        /// Interrupt Set-Enable Registers.
        (0x0100 => ISENABLER: [ReadWrite<u32>; 0x20]),
        /// Interrupt Clear-Enable Registers.
        (0x0180 => ICENABLER: [ReadWrite<u32>; 0x20]),
        /// Interrupt Set-Pending Registers.
        (0x0200 => ISPENDR: [ReadWrite<u32>; 0x20]),
        /// Interrupt Clear-Pending Registers.
        (0x0280 => ICPENDR: [ReadWrite<u32>; 0x20]),
        /// Interrupt Set-Active Registers.
        (0x0300 => ISACTIVER: [ReadWrite<u32>; 0x20]),
        /// Interrupt Clear-Active Registers.
        (0x0380 => ICACTIVER: [ReadWrite<u32>; 0x20]),
        /// Interrupt Priority Registers.
        (0x0400 => IPRIORITYR: [ReadWrite<u8>; 1024]),
        /// Interrupt Processor Targets Registers.
        (0x0800 => ITARGETSR: [ReadWrite<u8>; 1024]),
        /// Interrupt Configuration Registers.
        (0x0c00 => pub ICFGR: [ReadWrite<u32>; 0x40]),
        (0x0d00 => _rsv2),
        /// Software Generated Interrupt Register.
        (0x0f00 => SGIR: WriteOnly<u32, SGIR::Register>),
        (0x0f04 => _rsv3),
        (0x0f10 => CPENDSGIR: [ReadWrite<u32>; 0x4]),
        (0x0f20 => SPENDSGIR: [ReadWrite<u32>; 0x4]),
        (0x0f30 => _rsv4),
        (0x0fe8 => ICPIDR2 : ReadOnly<u32, PIDR2::Register>),
        (0x0fec => @END),
    }
}
register_bitfields! [
    u32,
    pub CTLR [
        EnableGrp0 OFFSET(0) NUMBITS(1) [],
        EnableGrp1NS OFFSET(1) NUMBITS(1) [],
        EnableGrp1S OFFSET(2) NUMBITS(1) [],
        ARE_S OFFSET(4) NUMBITS(1) [],
        ARE_NS OFFSET(5) NUMBITS(1) [],
    ],
    TYPER [
        ITLinesNumber OFFSET(0) NUMBITS(5) [],
        CPUNumber OFFSET(5) NUMBITS(3) []
    ],
    IIDR [
        Implementer OFFSET(0) NUMBITS(12) [],
        Revision OFFSET(12) NUMBITS(4) [],
        Variant OFFSET(16) NUMBITS(4) [],
        ProductId OFFSET(24) NUMBITS(8) []
    ],
    SGIR [
         SGIINTID OFFSET(0) NUMBITS(4) [],
         NSATT OFFSET(15) NUMBITS(1) [],
         CPUTargetList OFFSET(16) NUMBITS(8) [],
         TargetListFilter OFFSET(24) NUMBITS(2) [
            TargetList=0,
            AllOther=0b01,
            Current=0b10,
         ],
    ],
    IAR [
        INTID OFFSET(0) NUMBITS(10) [],
        CPUID OFFSET(10) NUMBITS(3) []
    ],
    pub PIDR2 [
        ArchRev OFFSET(4) NUMBITS(4) [],
    ],
];

impl Distributor {
    pub fn version(&self) -> u32 {
        self.ICPIDR2.read(PIDR2::ArchRev)
    }

    pub fn implementer(&self) -> u32 {
        self.IIDR.read(IIDR::Implementer)
    }

    pub fn irq_line_max(&self) -> u32 {
        (self.TYPER.read(TYPER::ITLinesNumber) + 1) * 32
    }

    pub fn set_enable_interrupt(&self, irq: IntId, enable: bool) {
        let int_id: u32 = irq.into();
        let index = (int_id / 32) as usize;
        let bit = 1 << (int_id % 32);
        if enable {
            self.ISENABLER[index].set(bit);
        } else {
            self.ICENABLER[index].set(bit);
        }
    }

    pub fn set_priority(&self, intid: IntId, priority: u8) {
        self.IPRIORITYR[u32::from(intid) as usize].set(priority)
    }

    pub fn set_bind_cpu(&self, intid: IntId, target_list: u8) {
        self.ITARGETSR[u32::from(intid) as usize].set(target_list)
    }

    pub fn init(&self) {
        let max_irqs = self.irq_line_max() as usize;

        // Disable all interrupts
        for i in (0..max_irqs).step_by(32) {
            self.ICENABLER[i / 32].set(u32::MAX);
            self.ICPENDR[i / 32].set(u32::MAX);
        }

        self.CTLR.write(
            CTLR::ARE_S::SET
                + CTLR::ARE_NS::SET
                + CTLR::EnableGrp0::SET
                + CTLR::EnableGrp1S::SET
                + CTLR::EnableGrp1NS::SET,
        );

        // if self.cpu_num() > 1 {
        //     for i in (SPI_RANGE.start as usize..max_irqs).step_by(4) {
        //         // Set external interrupts to target cpu 0
        //         self.ITARGETSR[i / 4].set(0x01_01_01_01);
        //     }
        // }
    }

    pub fn sgi(&self, intid: IntId, target: SGITarget) {
        assert!(intid.is_sgi());

        let mut val = SGIR::SGIINTID.val(intid.into());

        match target {
            SGITarget::AllOther => {
                val += SGIR::TargetListFilter::AllOther;
            }
            SGITarget::Targets(list) => {
                if list.is_empty() {
                    return;
                }

                let target_list = list
                    .iter()
                    .fold(0, |acc, &target| acc | target.cpu_target_list());

                val += SGIR::TargetListFilter::TargetList
                    + SGIR::CPUTargetList.val(target_list as u32);
            }
        }

        self.SGIR.write(val);
    }

    // pub fn cpu_num(&self) -> u32 {
    //     self.TYPER.read(TYPER::CPUNumber) + 1
    // }
}

pub fn mpid() -> MPID {
    let mpid: usize;
    unsafe {
        asm!("
        mrs {0}, MPIDR_EL1
        ", out(reg) mpid);
    }
    mpid.into()
}

pub fn current_cpu() -> CPUTarget {
    mpid().into()
}
