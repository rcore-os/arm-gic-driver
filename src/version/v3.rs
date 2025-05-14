use core::{arch::asm, hint::spin_loop, ops::Index, ptr::NonNull};

use super::IntId;
use aarch64_cpu::registers::{CurrentEL, MPIDR_EL1};
use alloc::boxed::Box;
use rdif_intc::*;
use tock_registers::{interfaces::*, register_bitfields, register_structs, registers::*};

use super::*;

const GICC_SRE_SRE: usize = 1 << 0;
const GICC_SRE_DFB: usize = 1 << 1;
const GICC_SRE_DIB: usize = 1 << 2;

#[derive(Debug, Clone, Copy)]
pub enum Security {
    Two,
    OneNS,
}

impl Default for Security {
    fn default() -> Self {
        Self::Two
    }
}

pub struct Gic {
    gicd: NonNull<Distributor>,
    gicr: NonNull<u8>,
    security: Security,
    max_spi_num: usize,
}

impl Gic {
    pub fn new(gicd: NonNull<u8>, gicr: NonNull<u8>, security: Security) -> Self {
        Self {
            gicd: gicd.cast(),
            gicr,
            security,
            max_spi_num: 0,
        }
    }

    fn reg(&self) -> &Distributor {
        unsafe { self.gicd.as_ref() }
    }

    fn reg_mut(&mut self) -> &mut Distributor {
        unsafe { self.gicd.as_mut() }
    }

    fn wait_ctlr(&self) {
        while self.reg().CTLR.is_set(CTLR::RWP) {
            spin_loop();
        }
    }
    // fn rd_slice(&self) -> RDv3Slice {
    //     RDv3Slice::new(self.gicr)
    // }
    // fn current_rd(&self) -> NonNull<RedistributorV3> {
    //     let want = (MPIDR_EL1.get() & 0xFFF) as u32;

    //     for rd in self.rd_slice().iter() {
    //         let affi = unsafe { rd.as_ref() }.lpi_ref().TYPER.read(TYPER::Affinity) as u32;
    //         if affi == want {
    //             return rd;
    //         }
    //     }
    //     panic!("No current redistributor")
    // }

    // fn rd_mut(&mut self) -> &mut RedistributorV3 {
    //     unsafe { self.current_rd().as_mut() }
    // }
}

unsafe impl Send for Gic {}

impl DriverGeneric for Gic {
    fn open(&mut self) -> DriverResult {
        // Disable the distributor
        self.reg_mut().CTLR.set(0);
        self.wait_ctlr();

        self.max_spi_num = self.reg().max_spi_num();

        if matches!(self.security, Security::OneNS) {
            self.reg_mut().CTLR.modify(CTLR::DS::SET);
        }

        // 关闭所有中断，并将中断分组默认为group 1
        for reg in self.reg_mut().ICENABLER.iter_mut() {
            reg.set(u32::MAX);
        }

        for reg in self.reg_mut().ICPENDR.iter_mut() {
            reg.set(u32::MAX);
        }

        for reg in self.reg_mut().IGROUPR.iter_mut() {
            reg.set(u32::MAX);
        }

        for reg in self.reg_mut().IGRPMODR.iter() {
            reg.set(u32::MAX);
        }

        self.wait_ctlr();

        for reg in self.reg_mut().IPRIORITYR.iter_mut() {
            reg.set(0xa0);
        }

        for reg in self.reg_mut().ICFGR.iter_mut() {
            reg.set(0x0);
        }

        match self.security {
            Security::Two => self
                .reg_mut()
                .CTLR
                .write(CTLR::ARE_NS::SET + CTLR::EnableGrp1NS::SET),
            Security::OneNS => self
                .reg_mut()
                .CTLR
                .write(CTLR::ARE_S::SET + CTLR::EnableGrp1S::SET),
        }
        Ok(())
    }

    fn close(&mut self) -> DriverResult {
        Ok(())
    }
}

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
impl Interface for Gic {
    fn cpu_interface(&self) -> BoxCPU {
        Box::new(GicCpu::new(self.gicr))
    }

    fn irq_enable(&mut self, irq: IrqId) -> Result<(), IntcError> {
        let id = IntId::from(irq);
        if id.is_private() {
            return Err(IntcError::IrqIdNotCompatible { id: irq });
        }
        self.reg_mut().set_enable_interrupt(id, true);
        Ok(())
    }

    fn irq_disable(&mut self, irq: IrqId) -> Result<(), IntcError> {
        let intid = IntId::from(irq);
        if intid.is_private() {
            return Err(IntcError::IrqIdNotCompatible { id: irq });
        }
        self.reg_mut().set_enable_interrupt(intid, false);
        Ok(())
    }

    fn set_priority(&mut self, irq: IrqId, priority: usize) -> Result<(), IntcError> {
        let intid = IntId::from(irq);
        if intid.is_private() {
            return Err(IntcError::IrqIdNotCompatible { id: irq });
        }
        self.reg_mut().set_priority(intid, priority as _);
        Ok(())
    }

    fn set_trigger(&mut self, irq: IrqId, trigger: Trigger) -> Result<(), IntcError> {
        let intid = IntId::from(irq);
        if intid.is_private() {
            return Err(IntcError::IrqIdNotCompatible { id: irq });
        }
        self.reg_mut().set_cfgr(intid, trigger);
        Ok(())
    }

    fn set_target_cpu(&mut self, irq: IrqId, cpu: CpuId) -> Result<(), IntcError> {
        let intid = IntId::from(irq);
        if intid.is_private() {
            return Err(IntcError::IrqIdNotCompatible { id: irq });
        }

        let mpid: usize = cpu.into();
        let target = CPUTarget::from(MPID::from(mpid as u64));
        self.reg_mut().set_route(intid, target);

        Ok(())
    }

    fn capabilities(&self) -> Vec<Capability> {
        alloc::vec![Capability::FdtParseConfig(fdt_parse_irq_config)]
    }
}

#[derive(Debug)]
pub struct GicCpu {
    gicr: NonNull<u8>,
}

unsafe impl Send for GicCpu {}
unsafe impl Sync for GicCpu {}

impl GicCpu {
    fn new(gicr: NonNull<u8>) -> Self {
        Self { gicr }
    }

    fn rd_slice(&self) -> RDv3Slice {
        RDv3Slice::new(self.gicr)
    }

    fn current_rd(&self) -> NonNull<RedistributorV3> {
        let want = (MPIDR_EL1.get() & 0xFFF) as u32;

        for rd in self.rd_slice().iter() {
            let affi = unsafe { rd.as_ref() }.lpi_ref().TYPER.read(TYPER::Affinity) as u32;
            if affi == want {
                return rd;
            }
        }
        panic!("No current redistributor")
    }
}
const ICC_CTLR_EL1_EOIMODE: usize = 1 << 1;

impl InterfaceCPU for GicCpu {
    fn set_eoi_mode(&self, b: bool) {
        let mut reg = cpu_read!("ICC_CTLR_EL1");
        if b {
            reg |= ICC_CTLR_EL1_EOIMODE;
        } else {
            reg &= !ICC_CTLR_EL1_EOIMODE;
        }
        cpu_write!("ICC_CTLR_EL1", reg);
    }

    fn get_eoi_mode(&self) -> bool {
        let reg = cpu_read!("ICC_CTLR_EL1");
        (reg & ICC_CTLR_EL1_EOIMODE) != 0
    }

    fn ack(&self) -> Option<IrqId> {
        let intid = cpu_read!("icc_iar1_el1");

        if intid == SPECIAL_RANGE.start as usize {
            None
        } else {
            Some(intid.into())
        }
    }

    fn eoi(&self, irq: IrqId) {
        let intid: usize = irq.into();
        cpu_write!("icc_eoir1_el1", intid);
    }

    fn dir(&self, irq: IrqId) {
        let intid: usize = irq.into();
        cpu_write!("icc_dir_el1", intid);
    }

    fn setup(&self) {
        let rd = unsafe { self.current_rd().as_mut() };

        rd.lpi.wake();
        rd.sgi.ICENABLER0.set(u32::MAX);
        rd.sgi.ICPENDR0.set(u32::MAX);
        rd.sgi.IGROUPR0.set(u32::MAX);
        rd.sgi.IGRPMODR0.set(u32::MAX);

        if CurrentEL.read(CurrentEL::EL) == 2 {
            let mut reg = cpu_read!("ICC_SRE_EL2");
            reg |= GICC_SRE_SRE | GICC_SRE_DFB | GICC_SRE_DIB;
            cpu_write!("ICC_SRE_EL2", reg);
        } else {
            let mut reg = cpu_read!("ICC_SRE_EL1");
            if (reg & GICC_SRE_SRE) == 0 {
                reg |= GICC_SRE_SRE | GICC_SRE_DFB | GICC_SRE_DIB;
                cpu_write!("ICC_SRE_EL1", reg);
            }
        }

        cpu_write!("ICC_PMR_EL1", 0xFF);
        enable_group1();
        const GICC_CTLR_CBPR: usize = 1 << 0;
        cpu_write!("ICC_CTLR_EL1", GICC_CTLR_CBPR);
    }

    fn capability(&self) -> CPUCapability {
        let o = Box::new(GicCpu { gicr: self.gicr });
        CPUCapability::LocalIrq(o)
    }
}

impl CPUCapLocalIrq for GicCpu {
    fn irq_enable(&self, irq: IrqId) -> Result<(), IntcError> {
        let intid = IntId::from(irq);
        if !intid.is_private() {
            return Err(IntcError::IrqIdNotCompatible { id: irq });
        }
        unsafe {
            self.current_rd()
                .as_mut()
                .sgi
                .set_enable_interrupt(intid, true)
        };
        Ok(())
    }

    fn irq_disable(&self, irq: IrqId) -> Result<(), IntcError> {
        let intid = IntId::from(irq);
        if !intid.is_private() {
            return Err(IntcError::IrqIdNotCompatible { id: irq });
        }
        unsafe {
            self.current_rd()
                .as_mut()
                .sgi
                .set_enable_interrupt(intid, false)
        };
        Ok(())
    }

    fn set_priority(&self, irq: IrqId, priority: usize) -> Result<(), IntcError> {
        let intid = IntId::from(irq);
        if !intid.is_private() {
            return Err(IntcError::IrqIdNotCompatible { id: irq });
        }
        unsafe {
            self.current_rd()
                .as_mut()
                .sgi
                .set_priority(intid, priority as _)
        };
        Ok(())
    }

    fn set_trigger(&self, irq: IrqId, trigger: Trigger) -> Result<(), IntcError> {
        let intid = IntId::from(irq);
        if !intid.is_private() {
            return Err(IntcError::IrqIdNotCompatible { id: irq });
        }
        unsafe {
            self.current_rd().as_mut().sgi.set_cfgr(intid, trigger);
        };
        Ok(())
    }
}

#[allow(unused)]
type RDv3Slice = RedistributorSlice<RedistributorV3>;
#[allow(unused)]
type RDv4Slice = RedistributorSlice<RedistributorV4>;

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
        (0x0008 => TYPER: ReadOnly<u64, TYPER::Register>),
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
            Trigger::EdgeBoth => 1,
            Trigger::EdgeRising => 1,
            Trigger::EdgeFailling => 1,
            Trigger::LevelHigh => 0,
            Trigger::LevelLow => 0,
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
