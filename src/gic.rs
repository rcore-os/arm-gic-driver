use core::{arch::asm, fmt::Display, ptr::NonNull};

use log::{debug, info, trace};
use tock_registers::{
    interfaces::{Readable, Writeable},
    registers::ReadWrite,
};

use crate::register::v3::icc_ctlr;
use crate::{
    define::*,
    register::{
        current_cpu,
        v2::{self, CpuInterface},
        v3::{self, enable_system_register_access, RDv3Vec, RDv4Vec, RedistributorItem, LPI, SGI},
        Distributor, CTLR,
    },
};

type Result<T = ()> = core::result::Result<T, GicError>;

pub enum Config {
    AutoDetect { reg: NonNull<u8> },
    V1 { gicc: NonNull<u8> },
    V2 { gicc: NonNull<u8> },
    V3 { gicr: NonNull<u8> },
    V4 { gicr: NonNull<u8> },
}

pub struct IrqConfig<'a> {
    pub intid: IntId,
    /// Not used for SPI.
    pub trigger: Trigger,
    /// 0xff is the minimum priority.
    pub priority: u8,
    /// If it is empty, irq will bind to core 0.
    pub cpu_list: &'a [CPUTarget],
}

pub struct Gic {
    gicd: NonNull<Distributor>,
    version_spec: VersionSpec,
}
unsafe impl Send for Gic {}
unsafe impl Sync for Gic {}

impl Gic {
    pub fn new(gicd: NonNull<u8>, config: Config) -> Result<Self> {
        let gicd = gicd.cast::<Distributor>();

        let s = match config {
            Config::AutoDetect { reg } => {
                let version = unsafe { gicd.as_ref().version() } as usize;
                let config = match version {
                    1 => Config::V1 { gicc: reg },
                    2 => Config::V2 { gicc: reg },
                    3 => Config::V3 { gicr: reg },
                    4 => Config::V4 { gicr: reg },
                    _ => return Err(GicError::Notimplemented),
                };

                Self::new_with_version(gicd, config)
            }
            _ => Self::new_with_version(gicd, config),
        }?;
        s.init();
        Ok(s)
    }
    fn new_with_version(gicd: NonNull<Distributor>, config: Config) -> Result<Self> {
        let version_spec = match config {
            Config::V1 { gicc } => VersionSpec::V1 {
                gicc: gicc.cast::<v2::CpuInterface>(),
            },
            Config::V2 { gicc } => VersionSpec::V2 {
                gicc: gicc.cast::<v2::CpuInterface>(),
            },
            Config::V3 { gicr } => VersionSpec::V3 {
                gicr: RDv3Vec::new(gicr.cast()),
            },
            Config::V4 { gicr } => VersionSpec::V4 {
                gicr: RDv4Vec::new(gicr.cast()),
            },
            _ => return Err(GicError::Notimplemented),
        };

        Ok(Self { gicd, version_spec })
    }

    fn gicd(&self) -> &Distributor {
        unsafe { self.gicd.as_ref() }
    }

    pub fn irq_max(&self) -> usize {
        self.gicd().irq_line_max() as _
    }

    pub fn current_cpu_setup(&self) {
        self.match_version(
            None,
            |grcc| {
                grcc.enable();
            },
            |lpi, sgi| {
                enable_system_register_access();
                debug!("register access enabled");
                lpi.wake();
                sgi.set_all_group1();
                v3::icc_ctlr();
                v3::enable_group1();
                info!("current priority mask: {}", v3::get_running_priority());
            },
        );

        self.set_priority_mask(0xff);
    }

    pub fn set_priority_mask(&self, priority: u8) {
        match self.version_spec {
            VersionSpec::V1 { gicc } | VersionSpec::V2 { gicc } => unsafe {
                gicc.as_ref().set_priority_mask(priority);
            },
            VersionSpec::V3 { .. } | VersionSpec::V4 { .. } => unsafe {
                asm!("msr icc_pmr_el1, {}", in(reg) priority as usize);
            },
        }
    }

    fn init(&self) {
        self.gicd().disable_all_interrupts();

        self.match_version_no_rd(
            |_| {
                self.gicd().CTLR.write(CTLR::EnableGrp0::SET);
            },
            || {
                // First set the ARE bits
                self.gicd().CTLR.write(CTLR::ARE_S::SET + CTLR::ARE_NS::SET);

                // The split here is because the register layout is different once ARE==1

                // Now set the rest of the options
                self.gicd().CTLR.write(
                    CTLR::ARE_S::SET
                        + CTLR::ARE_NS::SET
                        + CTLR::EnableGrp1NS::SET
                        + CTLR::EnableGrp1S::SET
                        + CTLR::EnableGrp0::SET,
                );
                // Put all SPIs into non-secure group 1.
                self.gicd().set_all_group1();
            },
        );
    }
    pub fn version(&self) -> usize {
        self.gicd().version() as _
    }

    /// Enable an interrupt.
    ///  
    pub fn irq_enable(&mut self, cfg: IrqConfig) {
        if cfg.intid.is_private() {
            assert!(
                cfg.cpu_list.len() < 2,
                "private interrupt can only be assigned to one cpu"
            );
        }

        let intid = cfg.intid;
        self.gicd().set_enable_interrupt(intid, true);
        self.gicd().set_priority(cfg.intid, cfg.priority);
        let core0 = [CPUTarget::CORE0];
        let target_list = if cfg.cpu_list.is_empty() {
            &core0
        } else {
            cfg.cpu_list
        };

        self.match_version_no_rd(
            |_| {
                self.gicd().set_bind_cpu(
                    intid,
                    target_list
                        .iter()
                        .fold(0, |acc, &cpu| acc | cpu.cpu_target_list()),
                );
            },
            || {},
        );

        for target in target_list {
            self.match_version(
                Some(*target),
                |_| {},
                |_, sgi| {
                    if intid.is_private() {
                        trace!("sgi enable ");
                        sgi.set_enable_interrupt(intid, true);
                        sgi.set_priority(intid, cfg.priority);
                    }
                    set_cfgr(&self.gicd().ICFGR, intid, cfg.trigger);
                    self.gicd().set_route(intid, target);
                },
            );
        }
    }

    fn match_version<FV1, FV3, O>(&self, target: Option<CPUTarget>, fv1: FV1, fv3: FV3) -> O
    where
        FV1: FnOnce(&CpuInterface) -> O,
        FV3: FnOnce(&LPI, &SGI) -> O,
    {
        match &self.version_spec {
            VersionSpec::V1 { gicc } | VersionSpec::V2 { gicc } => fv1(unsafe { gicc.as_ref() }),
            VersionSpec::V3 { gicr } => {
                let rd = &gicr[target.unwrap_or(current_cpu())];
                fv3(rd.lpi_ref(), rd.sgi_ref())
            }
            VersionSpec::V4 { gicr } => {
                let rd = &gicr[target.unwrap_or(current_cpu())];
                fv3(rd.lpi_ref(), rd.sgi_ref())
            }
        }
    }

    fn match_version_no_rd<FV1, FV3, O>(&self, fv1: FV1, fv3: FV3) -> O
    where
        FV1: FnOnce(&CpuInterface) -> O,
        FV3: FnOnce() -> O,
    {
        match &self.version_spec {
            VersionSpec::V1 { gicc } | VersionSpec::V2 { gicc } => fv1(unsafe { gicc.as_ref() }),
            VersionSpec::V3 { .. } | VersionSpec::V4 { .. } => fv3(),
        }
    }

    pub fn irq_disable(&self, intid: IntId) {
        self.gicd().set_enable_interrupt(intid, false);
    }

    pub fn get_and_acknowledge_interrupt(&self) -> Option<IntId> {
        self.match_version_no_rd(
            |gicc| gicc.get_and_acknowledge_interrupt(),
            || v3::get_and_acknowledge_interrupt(),
        )
    }

    pub fn end_interrupt(&self, intid: IntId) {
        self.match_version_no_rd(
            |gicc| gicc.end_interrupt(intid),
            || v3::end_interrupt(intid),
        );
    }

    pub fn send_sgi(&self, intid: IntId, target: SGITarget) {
        assert!(intid.is_sgi());
        self.match_version_no_rd(
            |_| self.gicd().sgi(intid, target),
            || v3::sgi(intid, target),
        );
    }

    // fn set_group(&self, intid: IntId, group: bool){

    // }
}

impl Display for Gic {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let com = match self.gicd().implementer() {
            0x43b => "Arm",
            _ => "unknown",
        };

        write!(
            f,
            "{}-GIC{}",
            com,
            match &self.version_spec {
                VersionSpec::V1 { .. } => "v1",
                VersionSpec::V2 { .. } => "v2",
                VersionSpec::V3 { .. } => "v3",
                VersionSpec::V4 { .. } => "v4",
            }
        )
    }
}

fn set_cfgr(icfgr: &[ReadWrite<u32, ()>], intid: IntId, trigger: Trigger) {
    if intid.is_sgi() {
        return;
    }

    let index = (u32::from(intid) / 16) as usize;
    let bit = 1 << (((u32::from(intid) % 16) * 2) + 1);

    let v = icfgr[index].get();
    icfgr[index].set(match trigger {
        Trigger::Edge => v | bit,
        Trigger::Level => v & !bit,
    })
}

enum VersionSpec {
    V1 { gicc: NonNull<v2::CpuInterface> },
    V2 { gicc: NonNull<v2::CpuInterface> },
    V3 { gicr: RDv3Vec },
    V4 { gicr: RDv4Vec },
}
