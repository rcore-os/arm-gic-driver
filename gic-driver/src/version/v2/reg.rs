use tock_registers::{interfaces::*, register_bitfields, register_structs, registers::*};

use crate::version::set_vector32_bit;

register_structs! {
    #[allow(non_snake_case)]
    pub DistributorReg {
        /// Distributor Control Register.
        (0x0000 => pub CTLR: ReadWrite<u32, CTLR::Register>),
        /// Interrupt Controller Type Register.
        (0x0004 => pub TYPER: ReadOnly<u32, TYPER::Register>),
        /// Distributor Implementer Identification Register.
        (0x0008 => pub IIDR: ReadOnly<u32, IIDR::Register>),
        (0x000c => _rsv1),
        /// Interrupt Group Registers.
        (0x0080 => pub IGROUPR: [ReadWrite<u32>; 0x20]),
        /// Interrupt Set-Enable Registers.
        (0x0100 => pub ISENABLER: [ReadWrite<u32>; 0x20]),
        /// Interrupt Clear-Enable Registers.
        (0x0180 => pub ICENABLER: [ReadWrite<u32>; 0x20]),
        /// Interrupt Set-Pending Registers.
        (0x0200 => pub ISPENDR: [ReadWrite<u32>; 0x20]),
        /// Interrupt Clear-Pending Registers.
        (0x0280 => pub ICPENDR: [ReadWrite<u32>; 0x20]),
        /// Interrupt Set-Active Registers.
        (0x0300 => pub ISACTIVER: [ReadWrite<u32>; 0x20]),
        /// Interrupt Clear-Active Registers.
        (0x0380 => pub ICACTIVER: [ReadWrite<u32>; 0x20]),
        /// Interrupt Priority Registers.
        (0x0400 => pub IPRIORITYR: [ReadWrite<u8>; 1024]),
        /// Interrupt Processor Targets Registers.
        (0x0800 => pub ITARGETSR: [ReadWrite<u8>; 1024]),
        /// Interrupt Configuration Registers.
        (0x0c00 => pub ICFGR: [ReadWrite<u32>; 0x40]),
        /// Private Peripheral Interrupt Status Register.
        (0x0d00 => pub PPISR: ReadOnly<u32>),
        /// Shared Peripheral Interrupt Status Registers.
        (0x0d04 => pub SPISR: [ReadOnly<u32>; 0x1f]),
        (0x0d80 => _rsv2),
        /// Non-secure Access Control Registers.
        (0x0e00 => pub NSACR: [ReadWrite<u32>; 0x40]),
        /// Software Generated Interrupt Register.
        (0x0f00 => pub SGIR: WriteOnly<u32, SGIR::Register>),
        (0x0f04 => _rsv4),
        /// SGI Clear-Pending Registers.
        (0x0f10 => pub CPENDSGIR: [ReadWrite<u32>; 0x4]),
        /// SGI Set-Pending Registers.
        (0x0f20 => pub SPENDSGIR: [ReadWrite<u32>; 0x4]),
        (0x0f30 => _rsv5),
        /// Peripheral ID4 Register.
        (0x0fd0 => pub PIDR4: ReadOnly<u32>),
        /// Peripheral ID5 Register.
        (0x0fd4 => pub PIDR5: ReadOnly<u32>),
        /// Peripheral ID6 Register.
        (0x0fd8 => pub PIDR6: ReadOnly<u32>),
        /// Peripheral ID7 Register.
        (0x0fdc => pub PIDR7: ReadOnly<u32>),
        /// Peripheral ID0 Register.
        (0x0fe0 => pub PIDR0: ReadOnly<u32>),
        /// Peripheral ID1 Register.
        (0x0fe4 => pub PIDR1: ReadOnly<u32>),
        /// Peripheral ID2 Register.
        (0x0fe8 => pub PIDR2: ReadOnly<u32, PIDR2::Register>),
        /// Peripheral ID3 Register.
        (0x0fec => pub PIDR3: ReadOnly<u32>),
        /// Component ID0 Register.
        (0x0ff0 => pub CIDR0: ReadOnly<u32>),
        /// Component ID1 Register.
        (0x0ff4 => pub CIDR1: ReadOnly<u32>),
        /// Component ID2 Register.
        (0x0ff8 => pub CIDR2: ReadOnly<u32>),
        /// Component ID3 Register.
        (0x0ffc => pub CIDR3: ReadOnly<u32>),
        (0x1000 => @END),
    }
}

impl DistributorReg {
    /// Disable the GIC Distributor
    pub fn disable(&self) {
        self.CTLR
            .modify(CTLR::EnableGrp0::CLEAR + CTLR::EnableGrp1::CLEAR);
    }

    /// Enable the GIC Distributor for both Group 0 and Group 1 interrupts
    pub fn enable(&self) {
        self.CTLR
            .modify(CTLR::EnableGrp0::SET + CTLR::EnableGrp1::SET);
    }

    /// Disable all interrupts
    pub fn disable_all_interrupts(&self, max_interrupts: u32) {
        // Calculate number of ICENABLER registers needed
        let num_regs = max_interrupts.div_ceil(32) as usize;
        let num_regs = num_regs.min(self.ICENABLER.len());

        for i in 0..num_regs {
            self.ICENABLER[i].set(u32::MAX);
        }
    }

    /// Clear all pending interrupts
    pub fn clear_all_pending_interrupts(&self, max_interrupts: u32) {
        // Calculate number of ICPENDR registers needed
        let num_regs = max_interrupts.div_ceil(32) as usize;
        let num_regs = num_regs.min(self.ICPENDR.len());

        for i in 0..num_regs {
            self.ICPENDR[i].set(u32::MAX);
        }
    }

    /// Clear all active interrupts
    pub fn clear_all_active_interrupts(&self, max_interrupts: u32) {
        // Calculate number of ICACTIVER registers needed
        let num_regs = max_interrupts.div_ceil(32) as usize;
        let num_regs = num_regs.min(self.ICACTIVER.len());

        for i in 0..num_regs {
            self.ICACTIVER[i].set(u32::MAX);
        }
    }

    /// Configure interrupt groups - set all interrupts to Group 1 (Non-secure) by default
    pub fn configure_interrupt_groups(&self, max_interrupts: u32) {
        // Calculate number of IGROUPR registers needed
        let num_regs = max_interrupts.div_ceil(32) as usize;
        let num_regs = num_regs.min(self.IGROUPR.len());

        // Set all interrupts to Group 1 (Non-secure)
        for i in 0..num_regs {
            self.IGROUPR[i].set(u32::MAX);
        }
    }

    /// Set default priorities for all interrupts
    pub fn set_default_priorities(&self, max_interrupts: u32) {
        // Calculate number of priority registers needed (4 interrupts per register)
        let num_regs = max_interrupts.div_ceil(4) as usize;
        let num_regs = num_regs.min(self.IPRIORITYR.len());

        // Set default priority (0xA0 - middle priority) for all interrupts
        for i in 0..num_regs {
            self.IPRIORITYR[i].set(0xA0);
        }
    }

    /// Configure interrupt targets for SPIs (Shared Peripheral Interrupts)
    pub fn configure_interrupt_targets(&self, max_interrupts: u32) {
        // SGIs (0-15) and PPIs (16-31) don't use ITARGETSR
        // Only SPIs (32+) need target configuration
        if max_interrupts <= 32 {
            return;
        }

        let spi_start = 32;
        let num_spis = max_interrupts - spi_start;
        let num_regs = num_spis.div_ceil(4) as usize;
        let target_reg_start = (spi_start / 4) as usize;
        let target_reg_end = target_reg_start + num_regs;
        let target_reg_end = target_reg_end.min(self.ITARGETSR.len());

        // Set all SPIs to target CPU 0 by default (0x01)
        for i in target_reg_start..target_reg_end {
            self.ITARGETSR[i].set(0x01);
        }
    }

    /// Configure interrupt configuration (edge/level triggered)
    pub fn configure_interrupt_config(&self, max_interrupts: u32) {
        // Calculate number of ICFGR registers needed (16 interrupts per register)
        let num_regs = max_interrupts.div_ceil(16) as usize;
        let num_regs = num_regs.min(self.ICFGR.len());

        // Configure all interrupts as level-sensitive (0x0) by default
        // SGIs are always edge-triggered, but we can set the bits anyway
        for i in 0..num_regs {
            self.ICFGR[i].set(0);
        }
    }

    /// Enable a specific interrupt
    pub fn enable_interrupt(&self, interrupt_id: u32) {
        set_vector32_bit(&self.ISENABLER, interrupt_id);
    }

    /// Disable a specific interrupt
    pub fn disable_interrupt(&self, interrupt_id: u32) {
        set_vector32_bit(&self.ICENABLER, interrupt_id);
    }

    /// Set interrupt priority
    pub fn set_interrupt_priority(&self, interrupt_id: u32, priority: u8) {
        if interrupt_id >= 1020 {
            return; // Invalid interrupt ID
        }

        let reg_idx = (interrupt_id / 4) as usize;

        if reg_idx < self.IPRIORITYR.len() {
            self.IPRIORITYR[reg_idx].set(priority);
        }
    }

    /// Set interrupt target CPU for SPIs
    pub fn set_interrupt_target(&self, interrupt_id: u32, target_cpu: u8) {
        if !(32..1020).contains(&interrupt_id) {
            return; // Invalid interrupt ID for target setting
        }

        let reg_idx = (interrupt_id / 4) as usize;

        if reg_idx < self.ITARGETSR.len() {
            self.ITARGETSR[reg_idx].set(target_cpu);
        }
    }

    /// Configure interrupt as Group 0 (Secure) or Group 1 (Non-secure)
    pub fn set_interrupt_group(&self, interrupt_id: u32, group1: bool) {
        if interrupt_id >= 1020 {
            return; // Invalid interrupt ID
        }

        let reg_idx = (interrupt_id / 32) as usize;
        let bit_idx = interrupt_id % 32;

        if reg_idx < self.IGROUPR.len() {
            if group1 {
                self.IGROUPR[reg_idx].set(self.IGROUPR[reg_idx].get() | (1 << bit_idx));
            } else {
                self.IGROUPR[reg_idx].set(self.IGROUPR[reg_idx].get() & !(1 << bit_idx));
            }
        }
    }

    pub fn max_spi_num(&self) -> u32 {
        let it_lines_number = self.TYPER.read(TYPER::ITLinesNumber); // ITLinesNumber field
        (it_lines_number + 1) * 32
    }
}

register_bitfields! [
    u32,
    /// Distributor Control Register (GICv2)
    pub CTLR [
        /// Enable Group 0 interrupts
        EnableGrp0 OFFSET(0) NUMBITS(1) [],
        /// Enable Group 1 interrupts
        EnableGrp1 OFFSET(1) NUMBITS(1) [],
    ],

    /// Interrupt Controller Type Register
    pub TYPER [
        /// Number of interrupt lines supported
        ITLinesNumber OFFSET(0) NUMBITS(5) [],
        /// Number of CPU interfaces implemented minus one
        CPUNumber OFFSET(5) NUMBITS(3) [],
        /// Indicates whether the GIC implements Security Extensions
        SecurityExtn OFFSET(10) NUMBITS(1) [
            SingleSecurity = 0,
            TwoSecurity = 1,
        ],
        /// Number of Lockable Shared Peripheral Interrupts
        LSPI OFFSET(11) NUMBITS(5) [],
    ],

    /// Distributor Implementer Identification Register
    pub IIDR [
        /// Implementer identification number
        Implementer OFFSET(0) NUMBITS(12) [],
        /// Revision number
        Revision OFFSET(12) NUMBITS(4) [],
        /// Variant number
        Variant OFFSET(16) NUMBITS(4) [],
        /// Product identification number
        ProductId OFFSET(24) NUMBITS(8) []
    ],

    /// Software Generated Interrupt Register
    pub SGIR [
        /// SGI interrupt ID
        SGIINTID OFFSET(0) NUMBITS(4) [],
        /// Non-secure access (only relevant when Security Extensions are implemented)
        NSATT OFFSET(15) NUMBITS(1) [],
        /// CPU target list
        CPUTargetList OFFSET(16) NUMBITS(8) [],
        /// Target list filter
        TargetListFilter OFFSET(24) NUMBITS(2) [
            /// Forward to CPUs listed in CPUTargetList
            TargetList = 0,
            /// Forward to all CPUs except the requesting CPU
            AllOther = 0b01,
            /// Forward only to the requesting CPU
            Current = 0b10,
        ],
    ],

    /// Peripheral ID2 Register
    pub PIDR2 [
        /// Architecture revision
        ArchRev OFFSET(4) NUMBITS(4) [],
    ],

    /// CPU Interface Control Register
    pub GICC_CTLR [
        /// Enable Group 0 interrupts
        EnableGrp0 OFFSET(0) NUMBITS(1) [],
        /// Enable Group 1 interrupts
        EnableGrp1 OFFSET(1) NUMBITS(1) [],
        /// Acknowledge control for Group 1 interrupts
        AckCtl OFFSET(2) NUMBITS(1) [],
        /// FIQ enable for Group 0 interrupts
        FIQEn OFFSET(3) NUMBITS(1) [],
        /// Common binary point register
        CBPR OFFSET(4) NUMBITS(1) [],
        /// FIQ bypass disable for Group 0
        FIQBypDisGrp0 OFFSET(5) NUMBITS(1) [],
        /// IRQ bypass disable for Group 0
        IRQBypDisGrp0 OFFSET(6) NUMBITS(1) [],
        /// FIQ bypass disable for Group 1
        FIQBypDisGrp1 OFFSET(7) NUMBITS(1) [],
        /// IRQ bypass disable for Group 1
        IRQBypDisGrp1 OFFSET(8) NUMBITS(1) [],
        /// EOI mode for Non-secure state
        EOImodeNS OFFSET(9) NUMBITS(1) [],
    ],

    /// Interrupt Acknowledge Register
    pub IAR [
        /// Interrupt ID
        InterruptID OFFSET(0) NUMBITS(10) [],
        /// CPU ID (for SGIs)
        CPUID OFFSET(10) NUMBITS(3) [],
    ],

    /// Priority Mask Register
    pub PMR [
        /// Priority
        Priority OFFSET(0) NUMBITS(8) [],
    ],

    /// Binary Point Register
    pub BPR [
        /// Binary point
        BinaryPoint OFFSET(0) NUMBITS(3) [],
    ],

    /// Running Priority Register
    pub RPR [
        /// Priority
        Priority OFFSET(0) NUMBITS(8) [],
    ],

    /// Highest Priority Pending Interrupt Register
    pub HPPIR [
        /// Pending interrupt ID
        PENDINTID OFFSET(0) NUMBITS(10) [],
        /// CPU ID (for SGIs)
        CPUID OFFSET(10) NUMBITS(3) [],
    ],

    /// Aliased Binary Point Register
    pub ABPR [
        /// Binary point
        BinaryPoint OFFSET(0) NUMBITS(3) [],
    ],

    /// Aliased Interrupt Acknowledge Register
    pub AIAR [
        /// Interrupt ID
        InterruptID OFFSET(0) NUMBITS(10) [],
        /// CPU ID (for SGIs)
        CPUID OFFSET(10) NUMBITS(3) [],
    ],

    /// Aliased End of Interrupt Register
    pub AEOIR [
        /// End of interrupt ID
        EOIINTID OFFSET(0) NUMBITS(10) [],
        /// CPU ID (for SGIs)
        CPUID OFFSET(10) NUMBITS(3) [],
    ],

    /// Aliased Highest Priority Pending Interrupt Register
    pub AHPPIR [
        /// Pending interrupt ID
        PENDINTID OFFSET(0) NUMBITS(10) [],
        /// CPU ID (for SGIs)
        CPUID OFFSET(10) NUMBITS(3) [],
    ],

    /// End of Interrupt Register
    pub EOIR [
        /// End of interrupt ID
        EOIINTID OFFSET(0) NUMBITS(10) [],
        /// CPU ID (for SGIs)
        CPUID OFFSET(10) NUMBITS(3) [],
    ],

    /// Deactivate Interrupt Register
    pub DIR [
        /// Interrupt ID
        InterruptID OFFSET(0) NUMBITS(10) [],
        /// CPU ID (for SGIs)
        CPUID OFFSET(10) NUMBITS(3) [],
    ],
];

register_structs! {
    /// GIC CPU Interface registers.
    #[allow(non_snake_case)]
    pub CpuInterfaceReg {
        /// CPU Interface Control Register.
        (0x0000 => pub CTLR: ReadWrite<u32, GICC_CTLR::Register>),
        /// Interrupt Priority Mask Register.
        (0x0004 => pub PMR: ReadWrite<u32, PMR::Register>),
        /// Binary Point Register.
        (0x0008 => pub BPR: ReadWrite<u32, BPR::Register>),
        /// Interrupt Acknowledge Register.
        (0x000c => pub IAR: ReadOnly<u32, IAR::Register>),
        /// End of Interrupt Register.
        (0x0010 => pub EOIR: WriteOnly<u32, EOIR::Register>),
        /// Running Priority Register.
        (0x0014 => pub RPR: ReadOnly<u32, RPR::Register>),
        /// Highest Priority Pending Interrupt Register.
        (0x0018 => pub HPPIR: ReadOnly<u32, HPPIR::Register>),
        /// Aliased Binary Point Register.
        (0x001c => pub ABPR: ReadWrite<u32, ABPR::Register>),
        /// Aliased Interrupt Acknowledge Register.
        (0x0020 => pub AIAR: ReadOnly<u32, AIAR::Register>),
        /// Aliased End of Interrupt Register.
        (0x0024 => pub AEOIR: WriteOnly<u32, AEOIR::Register>),
        /// Aliased Highest Priority Pending Interrupt Register.
        (0x0028 => pub AHPPIR: ReadOnly<u32, AHPPIR::Register>),
        (0x002c => _reserved_1),
        /// Active Priorities Registers.
        (0x00d0 => pub APR: [ReadWrite<u32>; 4]),
        /// Non-secure Active Priorities Registers.
        (0x00e0 => pub NSAPR: [ReadWrite<u32>; 4]),
        (0x00f0 => _reserved_2),
        /// CPU Interface Identification Register.
        (0x00fc => pub IIDR: ReadOnly<u32>),
        (0x0100 => _reserved_3),
        /// Deactivate Interrupt Register.
        (0x1000 => pub DIR: WriteOnly<u32, DIR::Register>),
        (0x1004 => @END),
    }
}
