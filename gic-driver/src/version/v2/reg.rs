use tock_registers::{register_bitfields, register_structs, registers::*};

register_structs! {
    #[allow(non_snake_case)]
    pub Distributor {
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
        /// Software Generated Interrupt Register.
        (0x0f00 => pub SGIR: WriteOnly<u32, SGIR::Register>),
        (0x0f04 => _rsv3),
        /// SGI Clear-Pending Registers.
        (0x0f10 => pub CPENDSGIR: [ReadWrite<u32>; 0x4]),
        /// SGI Set-Pending Registers.
        (0x0f20 => pub SPENDSGIR: [ReadWrite<u32>; 0x4]),
        (0x0f30 => _rsv4),
        /// Peripheral ID2 Register.
        (0x0fe8 => pub PIDR2: ReadOnly<u32, PIDR2::Register>),
        (0x0fec => _rsv5),
        (0xFFFC => @END),
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
        ],    /// Interrupt Controller Type Register
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
];

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
