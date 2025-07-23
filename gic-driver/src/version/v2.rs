use core::ptr::NonNull;

use rdif_intc::*;
use tock_registers::{register_structs, registers::*};

use super::*;

/// GICv2 driver. (support GICv1)
pub struct Gic {
    gicd: NonNull<Distributor>,
    gicc: NonNull<CpuInterface>,
}

unsafe impl Send for Gic {}

impl Gic {
    /// `gicd`: Distributor register base address. `gicc`: CPU interface register base address.
    pub fn new(gicd: NonNull<u8>, gicc: NonNull<u8>) -> Self {
        Self {
            gicd: gicd.cast(),
            gicc: gicc.cast(),
        }
    }

    fn gicd(&self) -> &Distributor {
        unsafe { self.gicd.as_ref() }
    }
}

impl DriverGeneric for Gic {
    fn open(&mut self) -> Result<(), KError> {
        self.gicd().disable_all_interrupts();
        self.gicd().CTLR.write(CTLR::EnableGrp0::SET);
        Ok(())
    }

    fn close(&mut self) -> Result<(), KError> {
        Ok(())
    }
}

impl Interface for Gic {
    fn irq_enable(&mut self, irq: IrqId) -> Result<(), IntcError> {
        self.gicd().set_enable_interrupt(irq.into(), true);
        Ok(())
    }

    fn irq_disable(&mut self, irq: IrqId) -> Result<(), IntcError> {
        self.gicd().set_enable_interrupt(irq.into(), false);
        Ok(())
    }

    fn set_priority(&mut self, irq: IrqId, priority: usize) -> Result<(), IntcError> {
        self.gicd().set_priority(irq.into(), priority as _);
        Ok(())
    }

    fn set_trigger(&mut self, irq: IrqId, trigger: Trigger) -> Result<(), IntcError> {
        self.gicd().set_cfgr(irq.into(), trigger);
        Ok(())
    }

    fn set_target_cpu(&mut self, irq: IrqId, cpu: CpuId) -> Result<(), IntcError> {
        let target_list = 1u8 << usize::from(cpu);
        self.gicd().set_bind_cpu(irq.into(), target_list);
        Ok(())
    }

    fn parse_dtb_fn(&self) -> Option<FuncFdtParseConfig> {
        Some(fdt_parse_irq_config)
    }

    fn cpu_local(&self) -> Option<local::Boxed> {
        Some(Box::new(GicCpu { ptr: self.gicc }))
    }
}

pub struct GicCpu {
    ptr: NonNull<CpuInterface>,
}

unsafe impl Sync for GicCpu {}
unsafe impl Send for GicCpu {}

impl GicCpu {
    fn gicc(&self) -> &CpuInterface {
        unsafe { self.ptr.as_ref() }
    }
}

impl DriverGeneric for GicCpu {
    fn open(&mut self) -> Result<(), KError> {
        self.gicc().enable();
        self.gicc().set_priority_mask(0xff);
        Ok(())
    }

    fn close(&mut self) -> Result<(), KError> {
        Ok(())
    }
}

impl local::Interface for GicCpu {
    fn set_eoi_mode(&self, b: bool) {
        self.gicc()
            .CTLR
            .modify(GICC_CTLR::EOIMODENS.val(if b { 1 } else { 0 }));
    }

    fn get_eoi_mode(&self) -> bool {
        self.gicc().CTLR.is_set(GICC_CTLR::EOIMODENS)
    }

    fn ack(&self) -> Option<IrqId> {
        self.gicc().ack().map(|i| (u32::from(i) as usize).into())
    }

    fn eoi(&self, intid: IrqId) {
        self.gicc().eoi(intid.into())
    }

    fn dir(&self, intid: IrqId) {
        self.gicc().dir(intid.into());
    }

    fn capability(&self) -> local::Capability {
        local::Capability::None
    }
}

register_structs! {
    /// GIC CPU Interface registers.
    #[allow(non_snake_case)]
    pub CpuInterface {
        /// CPU Interface Control Register.
        (0x0000 => CTLR: ReadWrite<u32, GICC_CTLR::Register>),
        /// Interrupt Priority Mask Register.
        (0x0004 => PMR: ReadWrite<u32>),
        /// Binary Point Register.
        (0x0008 => BPR: ReadWrite<u32>),
        /// Interrupt Acknowledge Register.
        (0x000c => IAR: ReadOnly<u32, IAR::Register>),
        /// End of Interrupt Register.
        (0x0010 => EOIR: WriteOnly<u32>),
        /// Running Priority Register.
        (0x0014 => RPR: ReadOnly<u32>),
        /// Highest Priority Pending Interrupt Register.
        (0x0018 => HPPIR: ReadOnly<u32>),
        (0x001c => _reserved_1),
        /// CPU Interface Identification Register.
        (0x00fc => IIDR: ReadOnly<u32>),
        (0x0100 => _reserved_2),
        (0x0f00 => SGIR: WriteOnly<u32, GICD_SGIR::Register>),
        (0x0f04 => reserve3),
        /// Deactivate Interrupt Register.
        (0x1000 => DIR: WriteOnly<u32>),
        (0x1004 => @END),
    }
}

register_bitfields! [
    u32,
    pub GICC_CTLR [
        EnableGrp0 OFFSET(0) NUMBITS(1) [],
        EOIMODENS OFFSET(9) NUMBITS(1) [],
    ],
    pub GICD_SGIR [
        /// [25:24] TargetListFilter
        /// Determines how the distributor must process the requested SGI:
        /// - 0b00 Forward the interrupt to the CPU interfaces specified in the CPUTargetList fielda.
        /// - 0b01 Forward the interrupt to all CPU interfaces except that of the processor that requested the interrupt.
        /// - 0b10 Forward the interrupt only to the CPU interface of the processor that requested the interrupt.
        /// - 0b11 Reserved.
        TargetListFilter OFFSET(24) NUMBITS(2) [
            ForwardToCPUTargetList = 0b00,
            ForwardToAllExceptRequester = 0b01,
            ForwardToRequester = 0b10,
            Reserved = 0b11
        ],
        /// [23:16] CPUTargetList
        /// When TargetList Filter = 0b00, defines the CPU interfaces to which the Distributor must forward the interrupt.
        /// Each bit of CPUTargetList[7:0] refers to the corresponding CPU interface, for example CPUTargetList[0] corresponds to CPU interface 0. Setting a bit to 1 indicates that the interrupt must be forwarded to the corresponding interface.
        /// If this field is 0x00 when TargetListFilter is 0b00, the Distributor does not forward the interrupt to any CPU interface.
        CPUTargetList OFFSET(16) NUMBITS(8) [],
        /// [15] NSATT
        /// Implemented only if the GIC includes the Security Extensions.
        /// Specifies the required security value of the SGI:
        /// - 0 Forward the SGI specified in the SGIINTID field to a specified CPU interface only if the SGI is configured as Group 0 on that interface.
        /// - 1 Forward the SGI specified in the SGIINTID field to a specified CPU interfaces only if the SGI is configured as Group 1 on that interface.
        ///
        /// This field is writable only by a Secure access. Any Non-secure write to the GICD_SGIR generates an SGI only if the specified SGI is programmed as Group 1, regardless of the value of bit[15] of the write.
        NSATT OFFSET(15) NUMBITS(1) [],
        /// [14:4] - Reserved, SBZ.
        Reserved14_4 OFFSET(4) NUMBITS(11) [],
        /// [3:0] SGIINTID
        /// The Interrupt ID of the SGI to forward to the specified CPU interfaces.
        /// The value of this field is the Interrupt ID, in the range 0-15,
        /// for example a value of 0b0011 specifies Interrupt ID 3.
        SGIINTID OFFSET(0) NUMBITS(4) []
    ]
];

impl CpuInterface {
    pub fn set_priority_mask(&self, priority: u8) {
        self.PMR.set(priority as u32);
    }

    pub fn enable(&self) {
        self.CTLR.write(GICC_CTLR::EnableGrp0::SET);
    }

    pub fn ack(&self) -> Option<IntId> {
        let id = self.IAR.read(IAR::INTID);
        if id == 1023 {
            None
        } else {
            unsafe { Some(IntId::raw(id)) }
        }
    }

    pub fn eoi(&self, intid: IntId) {
        self.EOIR.set(intid.into())
    }

    pub fn dir(&self, intid: IntId) {
        self.DIR.set(intid.into())
    }
}
