use core::{arch::asm, hint::spin_loop, marker::PhantomData, ops::Index, ptr::NonNull, u32};

use log::debug;
use tock_registers::{interfaces::*, register_bitfields, register_structs, registers::*};

use super::*;

const GICC_SRE_SRE: usize = 1 << 0;
const GICC_SRE_DFB: usize = 1 << 1;
const GICC_SRE_DIB: usize = 1 << 2;
const GICC_SRE_ENABLE: usize = 1 << 3;

pub struct GicV3 {
    gicd: NonNull<Distributor>,
    gicr: NonNull<u8>,
}

impl GicV3 {
    pub fn new(gicd: NonNull<u8>, gicr: NonNull<u8>) -> GicResult<Self> {
        let mut s = Self {
            gicd: gicd.cast(),
            gicr,
        };
        s.init_gicd()?;
        s.init_gicr()?;
        Ok(s)
    }

    fn gicd(&self) -> &Distributor {
        unsafe { self.gicd.as_ref() }
    }

    fn rd_slice<'a>(&'a self) -> RDv3Slice<'a> {
        RDv3Slice::new(self.gicr)
    }

    fn init_gicd(&mut self) -> GicResult<()> {
        // Disable the distributor.
        self.gicd().CTLR.set(0);
        self.wait_ctlr();

        debug!("disable all interrupts, and default to group 1");

        for reg in self.gicd().ICENABLER.iter() {
            reg.set(u32::MAX);
        }

        for reg in self.gicd().ICPENDR.iter() {
            reg.set(u32::MAX);
        }

        for reg in self.gicd().IGROUPR.iter() {
            reg.set(u32::MAX);
        }

        if !self.is_one_ns_security() {
            for reg in self.gicd().IGRPMODR.iter() {
                reg.set(u32::MAX);
            }
        }

        self.wait_ctlr();

        if self.is_one_ns_security() {
            self.gicd()
                .CTLR
                .write(CTLR::ARE_S.val(1) + CTLR::EnableGrp1NS.val(1));
        } else {
            self.gicd()
                .CTLR
                .write(CTLR::ARE_NS.val(1) + CTLR::EnableGrp1NS.val(1));
        }

        Ok(())
    }

    fn init_gicr(&mut self) -> GicResult<()> {
        for rd in self.rd_slice().iter() {
            rd.lpi.wake();
        }
        Ok(())
    }

    fn is_one_ns_security(&self) -> bool {
        self.gicd().CTLR.read(CTLR::DS) > 0
    }

    fn wait_ctlr(&self) {
        while self.gicd().CTLR.is_set(CTLR::RWP) {
            spin_loop();
        }
    }
}

unsafe impl Send for GicV3 {}
unsafe impl Sync for GicV3 {}

pub type RDv3Slice<'a> = RedistributorSlice<'a, RedistributorV3>;
pub type RDv4Slice<'a> = RedistributorSlice<'a, RedistributorV4>;

pub trait RedistributorItem {
    fn lpi_ref(&self) -> &LPI;
    fn sgi_ref(&self) -> &SGI;
}

pub(crate) struct RedistributorV3 {
    pub lpi: LPI,
    pub sgi: SGI,
}

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
pub struct RedistributorSlice<'a, T: RedistributorItem> {
    ptr: NonNull<T>,
    _phantom: PhantomData<&'a T>,
}

impl<'a, T: RedistributorItem> RedistributorSlice<'a, T> {
    pub fn new(ptr: NonNull<u8>) -> Self {
        Self {
            ptr: ptr.cast(),
            _phantom: PhantomData,
        }
    }

    pub fn iter(&self) -> RedistributorIter<'a, T> {
        RedistributorIter::new(self.ptr)
    }
}

pub struct RedistributorIter<'a, T: RedistributorItem> {
    ptr: NonNull<T>,
    is_last: bool,
    _marker: PhantomData<&'a T>,
}

impl<'a, T: RedistributorItem> RedistributorIter<'a, T> {
    const STEP: usize = size_of::<T>();

    pub fn new(p: NonNull<T>) -> Self {
        Self {
            ptr: p,
            is_last: false,
            _marker: PhantomData,
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

impl<'a, T: RedistributorItem> Index<CPUTarget> for RedistributorSlice<'a, T> {
    type Output = T;

    fn index(&self, index: CPUTarget) -> &Self::Output {
        let affinity = index.affinity();
        // let step = size_of::<T>();

        // unsafe {
        //     let mut ptr = self.ptr;
        //     loop {
        //         let lpi = ptr.as_ref().lpi_ref();
        //         let affi = lpi.TYPER.read(TYPER::Affinity) as u32;
        //         if affi == affinity {
        //             return ptr.as_ref();
        //         }

        //         if lpi.TYPER.read(TYPER::Last) > 0 {
        //             panic!("out of range!");
        //         }

        //         ptr = ptr.add(step);
        //     }
        // }

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
        (0x0fec => _rsv1),
        (0x10000 => @END),
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

register_structs! {
    #[allow(non_snake_case)]
    pub SGI {
        (0x0000 => _rsv0),
        // (0x0080 => IGROUPR0: ReadWrite<u32>),
        // (0x0084 => IGROUPR_E: [ReadWrite<u32>; 2]),
        (0x0080 => IGROUPR: [ReadWrite<u32>; 3]),
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
        (0x10000 => @END),
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

macro_rules! cpu_read {
    ($name: expr) => {{
        let x: usize;
        unsafe {
            core::arch::asm!(concat!("mrs {}, ", $name), out(reg) x);
        }
        x
    }};
}

macro_rules! cpu_write {
    ($name: expr, $value: expr) => {{
        let x = $value;
        unsafe {
            core::arch::asm!(concat!("msr ", $name, ", {}"), in(reg) x);
        }
    }};
}
fn enable_group1() {
    unsafe {
        asm!(
            "
    MOV   w0, #1
    MSR   ICC_IGRPEN1_EL1, x0
    ISB"
        )
    }
}

impl GicGeneric for GicV3 {
    fn get_and_acknowledge_interrupt(&self) -> Option<IntId> {
        let intid = cpu_read!("icc_iar1_el1") as u32;

        if intid == SPECIAL_RANGE.start {
            None
        } else {
            Some(unsafe { IntId::raw(intid) })
        }
    }

    fn end_interrupt(&self, intid: IntId) {
        let intid = u32::from(intid) as usize;
        cpu_write!("icc_eoir1_el1", intid);
    }

    fn irq_max_size(&self) -> usize {
        self.gicd().irq_line_max() as _
    }

    fn irq_enable(&mut self, intid: IntId) {
        todo!()
    }

    fn irq_disable(&mut self, intid: IntId) {
        todo!()
    }

    fn set_priority(&mut self, intid: IntId, priority: usize) {
        todo!()
    }

    fn set_trigger(&mut self, intid: IntId, trigger: Trigger) {
        todo!()
    }

    fn set_bind_cpu(&mut self, intid: IntId, cpu_list: &[CPUTarget]) {
        todo!()
    }

    fn current_cpu_setup(&self) {
        let mut reg = cpu_read!("ICC_SRE_EL1");
        if (reg & GICC_SRE_SRE) == 0 {
            reg |= GICC_SRE_SRE | GICC_SRE_DFB | GICC_SRE_DIB;
            cpu_write!("ICC_SRE_EL1", reg);
        }

        cpu_write!("ICC_PMR_EL1", 0xFF);
        enable_group1();
        const GICC_CTLR_CBPR: usize = 1 << 0;
        cpu_write!("ICC_CTLR_EL1", GICC_CTLR_CBPR);
    }
}
