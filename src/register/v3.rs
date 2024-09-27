use core::{arch::asm, hint::spin_loop, ops::Index, ptr::NonNull};

use tock_registers::{
    interfaces::*,
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite},
};

use super::{CPUTarget, IntId, SGITarget, PIDR2, SPECIAL_RANGE};

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
        (0x0084 => IGROUPR_E: [ReadWrite<u32>; 2]),
        (0x008C => _rsv1),
        (0x0100 => ISENABLER0: ReadWrite<u32>),
        (0x0104 => ISENABLER_E: [ReadWrite<u32>;2]),
        (0x010C => _rsv2),
        (0x0180 => ICENABLER0 : ReadWrite<u32>),
        (0x0184 => ICENABLER_E: [ReadWrite<u32>;2]),
        (0x018C => _rsv3),
        (0x0400 => IPRIORITYR: [ReadWrite<u8>; 32]),
        (0x0420 => IPRIORITYR_E: [ReadWrite<u8>; 64]),
        (0x0460 => _rsv5),
        (0x0C00 => pub ICFGR : [ReadWrite<u32>;6]),
        (0x0C18 => _rsv6),
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
pub fn get_and_acknowledge_interrupt() -> Option<IntId> {
    let x: usize;

    unsafe {
        asm!("
    mrs {}, icc_iar1_el1", out(reg) x);
    }
    let intid = x as u32;

    if intid == SPECIAL_RANGE.start {
        None
    } else {
        Some(unsafe { IntId::raw(intid) })
    }
}

pub fn end_interrupt(intid: IntId) {
    let intid = u32::from(intid) as usize;
    unsafe {
        asm!("
    msr icc_eoir1_el1, {}", in(reg) intid);
    }
}
pub fn enable_system_register_access() {
    let x: usize = 1;
    unsafe {
        asm!("
    msr icc_sre_el1, {}", in(reg) x);
    }
}

pub fn icc_ctlr() {
    let x: usize = 0;
    unsafe {
        asm!("
    msr icc_ctlr_el1, {}", in(reg) x);
    }
}

pub fn sgi(intid: IntId, target: SGITarget) {
    let val = match target {
        SGITarget::AllOther => {
            let irm = 0b1;
            (u64::from(u32::from(intid) & 0x0f) << 24) | (irm << 40)
        }
        SGITarget::Targets(list) => {
            if list.is_empty() {
                return;
            }
            let aff1 = list[0].aff1;
            let aff2 = list[0].aff2;
            let aff3 = list[0].aff3;
            let target_list = list
                .iter()
                .fold(0, |acc, target| acc | target.cpu_target_list());

            let irm = 0b0;
            u64::from(target_list)
                | (u64::from(aff1) << 16)
                | (u64::from(u32::from(intid) & 0x0f) << 24)
                | (u64::from(aff2) << 32)
                | (irm << 40)
                | (u64::from(aff3) << 48)
        }
    };

    unsafe {
        asm!("
    msr icc_sgi1r_el1, {}", in(reg) val);
    };
}

pub fn get_running_priority() -> u8 {
    let mut val: u64;
    unsafe {
        asm!("mrs {}, ICC_RPR_EL1", out(reg) val);
    }
    (val & 0xff) as u8
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

    pub fn set_all_group1(&self) {
        self.IGROUPR0.set(u32::MAX);
    }
}
