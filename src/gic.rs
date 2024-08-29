use core::{arch::asm, ptr::NonNull};

use tock_registers::interfaces::Writeable;

use crate::{
    define::*,
    register::{
        current_cpu, v2,
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
    pub target: CPUTarget,
    pub trigger: Trigger,
    pub priority: u8,
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
        self.match_v3v4(current_cpu(), |lpi, _| {
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
        let bit = 1 << (u32::from(cfg.intid) % 32);

        self.match_v3v4(cfg.target, |lpi, sgi| {
            if cfg.intid.is_private() {
                sgi.ISENABLER0.set(bit);
            }
        });
    }

    fn match_v3v4<F>(&self, id: CPUTarget, f: F)
    where
        F: FnOnce(&LPI, &SGI),
    {
        match &self.version_spec {
            VersionSpec::V3 { gicr } => {
                let rd = &gicr[id];
                f(rd.lpi_ref(), rd.sgi_ref());
            }
            VersionSpec::V4 { gicr } => {
                let rd = &gicr[id];
                f(rd.lpi_ref(), rd.sgi_ref());
            }
            _ => {}
        }
    }

    fn irq_disable(&self, intid: IntId) {}
}

enum VersionSpec {
    V1 { gicc: NonNull<v2::CpuInterface> },
    V2 { gicc: NonNull<v2::CpuInterface> },
    V3 { gicr: RDv3Vec },
    V4 { gicr: RDv4Vec },
}

// fn irq_enable(&self, cfg: IrqConfig);
// fn irq_disable(&self, intid: IntId);
// fn set_priority_mask(&self, priority: u8);
// /// Gets the ID of the highest priority signalled interrupt, and acknowledges it.
// ///
// /// Returns `None` if there is no pending interrupt of sufficient priority.
// fn get_and_acknowledge_interrupt(&self) -> Option<IntId>;

// /// Informs the interrupt controller that the CPU has completed processing the given interrupt.
// /// This drops the interrupt priority and deactivates the interrupt.
// fn end_interrupt(&self, intid: IntId);

// fn send_sgi(&self, intid: IntId, cpu_id: Option<CPUTarget>);
