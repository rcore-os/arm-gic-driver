use core::{hint::spin_loop, ops::Index, ptr::NonNull};

use tock_registers::{interfaces::*, register_bitfields, register_structs, registers::*};

use crate::{IntId, define::Trigger, v3::CPUTarget};

pub type RDv3Slice = RedistributorSlice<RedistributorV3>;
#[allow(unused)]
pub type RDv4Slice = RedistributorSlice<RedistributorV4>;

pub trait RedistributorItem {
    fn lpi_ref(&self) -> &LPI;
}

pub(crate) struct RedistributorV3 {
    pub lpi: LPI,
    pub sgi: SGI,
}

#[allow(unused)]
pub(crate) struct RedistributorV4 {
    pub lpi: LPI,
    pub sgi: SGI,
    pub _vlpi: LPI,
    pub _vsgi: SGI,
}
impl RedistributorItem for RedistributorV3 {
    fn lpi_ref(&self) -> &LPI {
        &self.lpi
    }
}
impl RedistributorItem for RedistributorV4 {
    fn lpi_ref(&self) -> &LPI {
        &self.lpi
    }
}
pub struct RedistributorSlice<T: RedistributorItem> {
    ptr: NonNull<T>,
}

impl<T: RedistributorItem> RedistributorSlice<T> {
    pub fn new(ptr: NonNull<u8>) -> Self {
        Self { ptr: ptr.cast() }
    }

    pub fn iter(&self) -> RedistributorIter<T> {
        RedistributorIter::new(self.ptr)
    }
}

pub struct RedistributorIter<T: RedistributorItem> {
    ptr: NonNull<T>,
    is_last: bool,
}

impl<T: RedistributorItem> RedistributorIter<T> {
    pub fn new(p: NonNull<T>) -> Self {
        Self {
            ptr: p,
            is_last: false,
        }
    }
}

impl<T: RedistributorItem> Iterator for RedistributorIter<T> {
    type Item = NonNull<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_last {
            return None;
        }
        unsafe {
            let ptr = self.ptr;
            let rd = ptr.as_ref();
            let lpi = rd.lpi_ref();
            if lpi.TYPER.read(TYPER::Last) > 0 {
                self.is_last = true;
            }
            self.ptr = self.ptr.add(1);
            Some(ptr)
        }
    }
}

impl<T: RedistributorItem> Index<CPUTarget> for RedistributorSlice<T> {
    type Output = T;

    fn index(&self, index: CPUTarget) -> &Self::Output {
        let affinity = index.affinity();
        for rd in self.iter() {
            let affi = unsafe { rd.as_ref() }.lpi_ref().TYPER.read(TYPER::Affinity) as u32;
            if affi == affinity {
                return unsafe { rd.as_ref() };
            }
        }
        unreachable!()
    }
}

register_structs! {
    /// GIC CPU Interface registers.
    #[allow(non_snake_case)]
    pub LPI {
        (0x0000 => CTLR: ReadWrite<u32, RCtrl::Register>),
        (0x0004 => IIDR: ReadOnly<u32>),
        (0x0008 => pub TYPER: ReadOnly<u64, TYPER::Register>),
        (0x0010 => STATUSR: ReadWrite<u32>),
        (0x0014 => WAKER: ReadWrite<u32, WAKER::Register>),
        (0x0018 => _rsv0),
        (0x0fe8 => PIDR2 : ReadOnly<u32, PIDR2::Register>),
        (0x0fec => _rsv1),
        (0x10000 => @END),
    }
}
register_bitfields! [
    u32,
    RCtrl [
        EnableLPIs OFFSET(0) NUMBITS(1) [],
        CES OFFSET(1) NUMBITS(1) [],
        IR  OFFSET(2) NUMBITS(1) [],
        RWP OFFSET(3) NUMBITS(1) [],
        DPG OFFSET(24) NUMBITS(1) [],
        DPG1NS OFFSET(25) NUMBITS(1) [],
        DPG1S OFFSET(26) NUMBITS(1) [],
        UWP OFFSET(31) NUMBITS(1) [],
    ],
];

impl LPI {
    pub fn wake(&self) {
        self.WAKER.write(WAKER::ProcessorSleep::CLEAR);

        while self.WAKER.is_set(WAKER::ChildrenAsleep) {
            spin_loop();
        }

        while self.CTLR.is_set(RCtrl::RWP) {
            spin_loop();
        }
    }
}

register_structs! {
    #[allow(non_snake_case)]
    pub SGI {
        (0x0000 => _rsv0),
        (0x0080 => IGROUPR0: ReadWrite<u32>),
        (0x0084 => IGROUPR_E: [ReadWrite<u32>; 2]),
        (0x008C => _rsv1),
        (0x0100 => ISENABLER0: ReadWrite<u32>),
        (0x0104 => ISENABLER_E: [ReadWrite<u32>;2]),
        (0x010C => _rsv2),
        (0x0180 => ICENABLER0 : ReadWrite<u32>),
        (0x0184 => ICENABLER_E: [ReadWrite<u32>;2]),
        (0x018C => _rsv3),
        (0x0200 => ISPENDR0: ReadWrite<u32>),
        (0x0204 => ISPENDR_E: [ReadWrite<u32>; 2]),
        (0x020C => _rsv4),
        (0x0280 => ICPENDR0: ReadWrite<u32>),
        (0x0284 => ICPENDR_E: [ReadWrite<u32>; 2]),
        (0x028C => _rsv5),
        (0x0400 => IPRIORITYR: [ReadWrite<u8>; 32]),
        (0x0420 => IPRIORITYR_E: [ReadWrite<u8>; 64]),
        (0x0460 => _rsv6),
        (0x0C00 => ICFGR : [ReadWrite<u32>; 6]),
        (0x0C18 => _rsv7),
        (0x0D00 => IGRPMODR0 : ReadWrite<u32>),
        (0x0D04 => IGRPMODR_E: [ReadWrite<u32>;2]),
        (0x0D0C => _rsv8),
        (0x10000 => @END),
    }
}
impl SGI {
    pub fn set_enable_interrupt(&self, irq: IntId, enable: bool) {
        let int_id: u32 = irq.into();
        let bit = 1 << (int_id % 32);
        if enable {
            self.ISENABLER0.set(bit);
        } else {
            self.ICENABLER0.set(bit);
        }
    }
    pub fn set_priority(&self, intid: IntId, priority: u8) {
        self.IPRIORITYR[u32::from(intid) as usize].set(priority)
    }

    fn set_cfgr(&self, intid: IntId, trigger: Trigger) {
        let clean = !((intid.to_u32() % 16) << 1);
        let bit: u32 = match trigger {
            Trigger::Edge => 1,
            Trigger::Level => 0,
        } << ((intid.to_u32() % 16) << 1);

        if intid.is_sgi() {
            let mut mask = self.ICFGR[0].get();
            mask &= clean;
            mask |= bit;

            self.ICFGR[0].set(mask);
        } else {
            let mut mask = self.ICFGR[1].get();
            mask &= clean;
            mask |= bit;
            self.ICFGR[1].set(mask);
        }
    }
}

register_bitfields! [
    u64,
    pub TYPER [
        //Indicates whether the GIC implementation supports physical LPIs.
        PLPIS OFFSET(0) NUMBITS(1) [],
        VLPIS OFFSET(1) NUMBITS(1) [],
        Dirty OFFSET(2) NUMBITS(1) [],
        Last OFFSET(4) NUMBITS(1) [],
        Affinity OFFSET(32) NUMBITS(32) [],
    ],

    pub IROUTER [
        AFF0 OFFSET(0) NUMBITS(8) [],
        AFF1 OFFSET(8) NUMBITS(8) [],
        AFF2 OFFSET(16) NUMBITS(8) [],
        InterruptRoutingMode OFFSET(31) NUMBITS(1) [
            Aff=0,
            Any=1,
        ],
        AFF3 OFFSET(32) NUMBITS(8) [],
    ]
];
register_bitfields! [
    u32,
    WAKER [
        ProcessorSleep OFFSET(1) NUMBITS(1) [],
        ChildrenAsleep OFFSET(2) NUMBITS(1) [],
    ],
    CTLR_TWO_S [
        EnableGrp0 OFFSET(0) NUMBITS(1) [],
        EnableGrp1NS OFFSET(1) NUMBITS(1) [],
        EnableGrp1S OFFSET(2) NUMBITS(1) [],
        ARE_S OFFSET(4) NUMBITS(1) [],
        ARE_NS OFFSET(5) NUMBITS(1) [],
        DS OFFSET(6) NUMBITS(1) [],
        RWP OFFSET(31) NUMBITS(1) [],
    ],
    CTLR_TWO_NS [
        EnableGrp1 OFFSET(0) NUMBITS(1) [],
        EnableGrp1A OFFSET(1) NUMBITS(1) [],
        ARE_NS OFFSET(4) NUMBITS(1) [],
        RWP OFFSET(31) NUMBITS(1) [],
    ],
    CTLR_ONE_NS [
        EnableGrp0 OFFSET(0) NUMBITS(1) [],
        EnableGrp1 OFFSET(1) NUMBITS(1) [],
        ARE OFFSET(4) NUMBITS(1) [],
        DS OFFSET(6) NUMBITS(1) [],
        RWP OFFSET(31) NUMBITS(1) [],
    ],
];
