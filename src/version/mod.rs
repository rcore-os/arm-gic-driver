use core::error::Error;

use alloc::boxed::Box;
use rdif_intc::*;
use tock_registers::{interfaces::*, register_bitfields, register_structs, registers::*};
use v3::IROUTER;

pub mod v2;
pub mod v3;

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
        (0x0d00 => pub IGRPMODR: [ReadWrite<u32>; 32]),
        (0x0d80 => _rsv2),
        /// Software Generated Interrupt Register.
        (0x0f00 => SGIR: WriteOnly<u32, SGIR::Register>),
        (0x0f04 => _rsv3),
        (0x0f10 => CPENDSGIR: [ReadWrite<u32>; 0x4]),
        (0x0f20 => SPENDSGIR: [ReadWrite<u32>; 0x4]),
        (0x0f30 => _rsv4),
        (0x0fe8 => ICPIDR2 : ReadOnly<u32, PIDR2::Register>),
        (0x0fec => _rsv5),
        /// v3

        (0x6000 => IROUTER: [ReadWrite<u64, IROUTER::Register>; 1019]),
        (0x7FD8 => _rsv6),
        (0xFFE8 => PIDR2 : ReadOnly<u32, PIDR2::Register>),
        (0xFFEC => _rsv7),
        (0xFFFC => @END),
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
        DS OFFSET(6) NUMBITS(1) [],
        RWP OFFSET(31) NUMBITS(1) [],
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

#[allow(unused)]
impl Distributor {
    pub fn version(&self) -> u32 {
        let v = self.ICPIDR2.read(PIDR2::ArchRev);
        if v == 1 || v == 2 {
            return v;
        }
        self.PIDR2.read(PIDR2::ArchRev)
    }

    pub fn implementer(&self) -> u32 {
        self.IIDR.read(IIDR::Implementer)
    }

    pub fn irq_line_max(&self) -> u32 {
        (self.TYPER.read(TYPER::ITLinesNumber) + 1) * 32
    }

    pub fn max_spi_num(&self) -> usize {
        let max_line = self.irq_line_max() as usize;
        ((max_line + 1) << 5) - 1
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

    pub fn disable_all_interrupts(&self) {
        for reg in &self.ICENABLER {
            reg.set(u32::MAX);
        }
        for reg in &self.ICPENDR {
            reg.set(u32::MAX);
        }
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

    pub fn set_all_group1(&self) {
        for i in 0..32 {
            self.IGROUPR[i].set(u32::MAX);
        }
    }

    pub fn set_route(&self, intid: IntId, target: CPUTarget) {
        self.IROUTER[u32::from(intid) as usize].write(
            IROUTER::InterruptRoutingMode::Aff
                + IROUTER::AFF0.val(target.aff0 as _)
                + IROUTER::AFF1.val(target.aff1 as _)
                + IROUTER::AFF2.val(target.aff2 as _)
                + IROUTER::AFF3.val(target.aff3 as _),
        );
    }

    fn set_cfgr(&self, intid: IntId, trigger: Trigger) {
        let val = u32::from(intid);

        let index = (val / 16) as usize;
        let bit = 1 << (((val % 16) * 2) + 1);

        let v = self.ICFGR[index].get();
        self.ICFGR[index].set(match trigger {
            Trigger::EdgeBoth => v | bit,
            Trigger::EdgeRising => v | bit,
            Trigger::EdgeFailling => v | bit,
            Trigger::LevelHigh => v & !bit,
            Trigger::LevelLow => v & !bit,
        })
    }

    // pub fn cpu_num(&self) -> u32 {
    //     self.TYPER.read(TYPER::CPUNumber) + 1
    // }
}

// The `bitflags!` macro generates `struct`s that manage a set of flags.
bitflags::bitflags! {
    /// Represents a set of flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct TriggerFlag: usize {
        const NONE = 0;
        const EDGE_RISING = 1;
        const EDGE_FALLING = 2;
        const EDGE_BOTH = Self::EDGE_RISING.bits()| Self::EDGE_FALLING.bits();
        const LEVEL_HIGH = 4;
        const LEVEL_LOW = 8;
    }
}
pub fn fdt_parse_irq_config(itr: &[u32]) -> Result<IrqConfig, Box<dyn Error>> {
    const SPI: u32 = 0;
    const PPI: u32 = 1;

    let num = itr[1];

    let irq_id: u32 = match itr[0] {
        SPI => IntId::spi(num),
        PPI => IntId::ppi(num),
        _ => panic!("Invalid irq type {}", itr[0]),
    }
    .into();

    let flag = TriggerFlag::from_bits_truncate(itr[2] as _);

    let trigger = if flag.contains(TriggerFlag::EDGE_BOTH) {
        Trigger::EdgeBoth
    } else if flag.contains(TriggerFlag::EDGE_RISING) {
        Trigger::EdgeRising
    } else if flag.contains(TriggerFlag::EDGE_FALLING) {
        Trigger::EdgeFailling
    } else if flag.contains(TriggerFlag::LEVEL_HIGH) {
        Trigger::LevelHigh
    } else if flag.contains(TriggerFlag::LEVEL_LOW) {
        Trigger::LevelLow
    } else {
        panic!("Invalid irq type {}", itr[2])
    };

    Ok(IrqConfig {
        irq: (irq_id as usize).into(),
        trigger,
    })
}
