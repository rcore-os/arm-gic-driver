use core::{arch::asm, hint::spin_loop, ops::Index, ptr::NonNull};

use tock_registers::{
    interfaces::*,
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};

use super::{current_cpu, CPUTarget, IntId, PIDR2};

pub type RDv3Vec = RedistributorVec<RedistributorV3>;
pub type RDv4Vec = RedistributorVec<RedistributorV4>;

pub trait RedistributorItem {
    fn lpi_ref(&self) -> &LPI;
    fn sgi_ref(&self) -> &SGI;
}

#[repr(align(0x10000))]
pub(crate) struct RedistributorV3 {
    pub lpi: LPI,
    pub sgi: SGI,
}

#[repr(align(0x10000))]
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

    fn sgi_ref(&self) -> &SGI {
        &self.sgi
    }
}
impl RedistributorItem for RedistributorV4 {
    fn lpi_ref(&self) -> &LPI {
        &self.lpi
    }
    fn sgi_ref(&self) -> &SGI {
        &self.sgi
    }
}
pub struct RedistributorVec<T: RedistributorItem> {
    ptr: NonNull<u8>,
    _marker: core::marker::PhantomData<T>,
}

impl<T: RedistributorItem> RedistributorVec<T> {
    pub fn new(ptr: NonNull<u8>) -> Self {
        Self {
            ptr,
            _marker: core::marker::PhantomData,
        }
    }

    pub fn iter(&self) -> RedistributorIter<T> {
        RedistributorIter::new(self.ptr)
    }

    pub fn wake(&self) {
        let rd = &self[current_cpu()];

        rd.lpi_ref().WAKER.write(WAKER::ProcessorSleep::CLEAR);

        while rd.lpi_ref().WAKER.is_set(WAKER::ChildrenAsleep) {
            spin_loop();
        }
    }
}

pub struct RedistributorIter<'a, T: RedistributorItem> {
    ptr: NonNull<u8>,
    is_last: bool,
    _phantom: core::marker::PhantomData<&'a T>,
}

impl<'a, T: RedistributorItem> RedistributorIter<'a, T> {
    const STEP: usize = size_of::<T>();

    pub fn new(p: NonNull<u8>) -> Self {
        Self {
            ptr: p,
            is_last: false,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<'a, T: RedistributorItem> Iterator for RedistributorIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_last {
            return None;
        }

        unsafe {
            let rd = self.ptr.cast::<T>().as_ref();
            let lpi = rd.lpi_ref();
            if lpi.TYPER.read(TYPER::Last) > 0 {
                self.is_last = true;
            }
            self.ptr = self.ptr.offset(Self::STEP as _);
            Some(rd)
        }
    }
}

impl<T: RedistributorItem> Index<CPUTarget> for RedistributorVec<T> {
    type Output = T;

    fn index(&self, index: CPUTarget) -> &Self::Output {
        let affinity = index.affinity();
        for rd in self.iter() {
            let affi = rd.lpi_ref().TYPER.read(TYPER::Affinity) as u32;
            if affi == affinity {
                return rd;
            }
        }
        unreachable!()
    }
}

register_structs! {
    /// GIC CPU Interface registers.
    #[allow(non_snake_case)]
    pub LPI {
        (0x0000 => CTLR: ReadWrite<u32>),
        (0x0004 => IIDR: ReadOnly<u32>),
        (0x0008 => TYPER: ReadOnly<u64, TYPER::Register>),
        (0x0010 => STATUSR: ReadWrite<u32>),
        (0x0014 => WAKER: ReadWrite<u32, WAKER::Register>),
        (0x0018 => _rsv0),
        (0x0fe8 => PIDR2 : ReadOnly<u32, PIDR2::Register>),
        (0x0fec => @END),
    }
}

register_structs! {
    #[allow(non_snake_case)]
    pub SGI {
        (0x0000 => _rsv0),
        (0x0080 => IGROUPR0: ReadWrite<u32>),
        (0x0084 => IGROUPR_E: ReadWrite<u32>),
        (0x0088 => _rsv1),
        (0x0100 => ISENABLER0: ReadWrite<u32>),
        (0x0104 => ISENABLER_E: ReadWrite<u32>),
        (0x0108 => _rsv2),
        (0x0180 => ICENABLER0 : ReadWrite<u32>),
        (0x0184 => ICENABLER_E: ReadWrite<u32>),
        (0x0188 => _rsv3),
        (0x0400 => IPRIORITYR: [ReadWrite<u8>; 28]),
        (0x041C => _rsv4),
        (0x0420 => IPRIORITYR_E: [ReadWrite<u8>; 60]),
        (0x045C => _rsv5),
        (0x0C00 => pub ICFGR : [ReadWrite<u32>;5]),
        (0x0C14 => _rsv6),
        (0xFFFC => @END),
    }
}

register_bitfields! [
    u64,
    TYPER [
        //Indicates whether the GIC implementation supports physical LPIs.
        PLPIS OFFSET(0) NUMBITS(1) [],
        VLPIS OFFSET(1) NUMBITS(1) [],
        Dirty OFFSET(2) NUMBITS(1) [],
        Last OFFSET(4) NUMBITS(1) [],
        Affinity OFFSET(32) NUMBITS(32) [],
    ],
];
register_bitfields! [
    u32,
    WAKER [
        ProcessorSleep OFFSET(1) NUMBITS(1) [],
        ChildrenAsleep OFFSET(2) NUMBITS(1) [],
    ],
];

pub fn enable_group0() {
    unsafe {
        asm!(
            "
    MOV   w0, #1
    MSR   ICC_IGRPEN0_EL1, x0
    ISB
        "
        )
    }
}

pub fn enable_group1() {
    unsafe {
        asm!(
            "
    MOV   w0, #1
    MSR   ICC_IGRPEN1_EL1, x0
    ISB
        "
        )
    }
}

impl LPI {
    pub fn wake(&self) {
        self.WAKER.write(WAKER::ProcessorSleep::CLEAR);

        while self.WAKER.is_set(WAKER::ChildrenAsleep) {
            spin_loop();
        }
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
}
