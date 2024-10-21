use core::{arch::asm, hint::spin_loop, ops::Index, ptr::NonNull};

use aarch64_cpu::registers::MPIDR_EL1;
use log::debug;
use tock_registers::{interfaces::*, register_bitfields, register_structs, registers::*};

use super::*;

const GICC_SRE_SRE: usize = 1 << 0;
const GICC_SRE_DFB: usize = 1 << 1;
const GICC_SRE_DIB: usize = 1 << 2;

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

    fn rd_slice(&self) -> RDv3Slice {
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
            let rd = unsafe { rd.as_ref() };
            rd.lpi.wake();
            rd.sgi.disable_all();
            rd.sgi.set_group();
            let affi = rd.lpi_ref().TYPER.read(TYPER::Affinity) as u32;
            debug!(
                "Cpu [{}.{}.{}.{}] rd ok",
                affi & 0xff,
                (affi >> 8) & 0xff,
                (affi >> 16) & 0xff,
                affi >> 24
            );
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

    fn current_rd(&self) -> &RedistributorV3 {
        let target = CPUTarget {
            aff0: MPIDR_EL1.read(MPIDR_EL1::Aff0) as _,
            aff1: MPIDR_EL1.read(MPIDR_EL1::Aff1) as _,
            aff2: MPIDR_EL1.read(MPIDR_EL1::Aff2) as _,
            aff3: MPIDR_EL1.read(MPIDR_EL1::Aff3) as _,
        };
        for rd in self.rd_slice().iter() {
            let affi = unsafe { rd.as_ref() }.lpi_ref().TYPER.read(TYPER::Affinity) as u32;
            if affi == target.affinity() {
                return unsafe { rd.as_ref() };
            }
        }
        unreachable!()
    }
}

unsafe impl Send for GicV3 {}
unsafe impl Sync for GicV3 {}

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
        (0x0C00 => ICFGR : [ReadWrite<u32>; 6]),
        (0x0C18 => _rsv6),
        (0x0D00 => IGRPMODR0 : ReadWrite<u32>),
        (0x0D04 => IGRPMODR_E: [ReadWrite<u32>;2]),
        (0x0D0C => _rsv7),
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

    fn disable_all(&self) {
        self.ICENABLER0.set(u32::MAX);
        for reg in &self.ICENABLER_E {
            reg.set(u32::MAX);
        }
    }

    fn set_group(&self) {
        self.IGROUPR0.set(u32::MAX);
        self.IGRPMODR0.set(u32::MAX);
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
            core::arch::asm!(concat!("msr ", $name, ", {0:x}"), in(reg) x);
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
        if intid.is_private() {
            self.current_rd().sgi.set_enable_interrupt(intid, true);
        } else {
            self.gicd().set_enable_interrupt(intid, true);
        }
    }

    fn irq_disable(&mut self, intid: IntId) {
        if intid.is_private() {
            self.current_rd().sgi.set_enable_interrupt(intid, false);
        } else {
            self.gicd().set_enable_interrupt(intid, false);
        }
    }

    fn set_priority(&mut self, intid: IntId, priority: usize) {
        if intid.is_private() {
            self.current_rd().sgi.set_priority(intid, priority as _);
        } else {
            self.gicd().set_priority(intid, priority as _);
        }
    }

    fn set_trigger(&mut self, intid: IntId, trigger: Trigger) {
        if intid.is_private() {
            self.current_rd().sgi.set_cfgr(intid, trigger);
        } else {
            self.gicd().set_cfgr(intid, trigger);
        }
    }

    fn set_bind_cpu(&mut self, intid: IntId, cpu_list: &[CPUTarget]) {
        if intid.is_private() {
            panic!("Private interrupt cannot be bound to CPU");
        } else {
            self.gicd().set_bind_cpu(
                intid,
                cpu_list
                    .iter()
                    .fold(0, |acc, &cpu| acc | cpu.cpu_target_list()),
            );
        }
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
