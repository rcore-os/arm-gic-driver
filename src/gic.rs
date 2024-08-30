use core::{arch::asm, fmt::Display, ptr::NonNull};

use tock_registers::{
    interfaces::{Readable, Writeable},
    registers::ReadWrite,
};

use crate::{
    define::*,
    register::{
        current_cpu,
        v2::{self, CpuInterface},
        v3::{self, RDv3Vec, RDv4Vec, RedistributorItem, LPI, SGI},
        Distributor,
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

pub struct IrqConfig {
    pub intid: IntId,
    pub trigger: Trigger,
    pub priority: u8,
    pub cpu: Option<CPUTarget>,
}

pub struct Gic {
    gicd: NonNull<Distributor>,
    version_spec: VersionSpec,
}

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
        self.match_v3v4(Some(current_cpu()), |lpi, _| {
            lpi.wake();
        });

        match self.version_spec {
            VersionSpec::V1 { gicc } | VersionSpec::V2 { gicc } => unsafe {
                gicc.as_ref().enable();
            },
            VersionSpec::V3 { .. } | VersionSpec::V4 { .. } => {
                v3::enable_group0();
                v3::enable_group1();
            }
        }
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
        self.gicd().init();
    }

    pub fn irq_enable(&self, cfg: IrqConfig) {
        self.gicd().set_enable_interrupt(cfg.intid, true);

        self.match_v1v2(|_| {
            set_cfgr(&self.gicd().ICFGR, cfg.intid, cfg.trigger);
            self.gicd().set_priority(cfg.intid, cfg.priority);
            self.gicd().set_bind_cpu(cfg.intid, 0);
        });

        self.match_v3v4(cfg.cpu, |_, sgi| {
            if cfg.intid.is_private() {
                sgi.set_enable_interrupt(cfg.intid, true);
                set_cfgr(&sgi.ICFGR, cfg.intid, cfg.trigger);

                sgi.set_priority(cfg.intid, cfg.priority);
                self.gicd().set_bind_cpu(cfg.intid, 0);
            } else {
                set_cfgr(&self.gicd().ICFGR, cfg.intid, cfg.trigger);
                self.gicd().set_priority(cfg.intid, cfg.priority);
            }
        });
    }
    fn match_v1v2<F, O>(&self, f: F) -> Option<O>
    where
        F: FnOnce(&CpuInterface) -> O,
    {
        match &self.version_spec {
            VersionSpec::V1 { gicc } | VersionSpec::V2 { gicc } => {
                Some(f(unsafe { gicc.as_ref() }))
            }
            _ => None,
        }
    }
    fn match_v3v4<F, O>(&self, id: Option<CPUTarget>, f: F) -> Option<O>
    where
        F: FnOnce(&LPI, &SGI) -> O,
    {
        match &self.version_spec {
            VersionSpec::V3 { gicr } => {
                let rd = &gicr[id.unwrap_or(current_cpu())];
                Some(f(rd.lpi_ref(), rd.sgi_ref()))
            }
            VersionSpec::V4 { gicr } => {
                let rd = &gicr[id.unwrap_or(current_cpu())];
                Some(f(rd.lpi_ref(), rd.sgi_ref()))
            }
            _ => None,
        }
    }

    pub fn irq_disable(&self, intid: IntId, cpu: Option<CPUTarget>) {
        self.gicd().set_enable_interrupt(intid, false);
        self.match_v3v4(cpu, |_, sgi| {
            if intid.is_private() {
                sgi.set_enable_interrupt(intid, false);
            }
        });
    }

    pub fn get_and_acknowledge_interrupt(&self) -> Option<IntId> {
        if let Some(res) = self.match_v1v2(|gicc| gicc.get_and_acknowledge_interrupt()) {
            return res;
        }
        if matches!(
            self.version_spec,
            VersionSpec::V3 { .. } | VersionSpec::V4 { .. }
        ) {
            return v3::get_and_acknowledge_interrupt();
        }
        None
    }

    pub fn end_interrupt(&self, intid: IntId) {
        self.match_v1v2(|gicc| gicc.end_interrupt(intid));
        if matches!(
            self.version_spec,
            VersionSpec::V3 { .. } | VersionSpec::V4 { .. }
        ) {
            return v3::end_interrupt(intid);
        }
    }

    pub fn send_sgi(&self, intid: IntId, cpu_id: Option<CPUTarget>) {
        assert!(intid.is_sgi());

        let sgi_value = match cpu_id {
            None => {
                let irm = 0b1;
                (u64::from(u32::from(intid) & 0x0f) << 24) | (irm << 40)
            }
            Some(cpu) => {
                let irm = 0b0;
                u64::from(cpu.target_list)
                    | (u64::from(cpu.aff1) << 16)
                    | (u64::from(u32::from(intid) & 0x0f) << 24)
                    | (u64::from(cpu.aff2) << 32)
                    | (irm << 40)
                    | (u64::from(cpu.aff3) << 48)
            }
        };

        unsafe {
            asm!("
    msr icc_sgi1r_el1, {}", in(reg) sgi_value);
        }
    }
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
