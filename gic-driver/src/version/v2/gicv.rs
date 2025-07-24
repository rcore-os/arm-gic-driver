use tock_registers::{register_bitfields, register_structs, registers::*};

register_structs! {
    /// GIC Virtual CPU Interface registers.
    #[allow(non_snake_case)]
    pub VirtualCpuInterfaceReg {
        /// Virtual Machine Control Register
        (0x0000 => pub CTLR: ReadWrite<u32, CTLR::Register>),
        /// VM Priority Mask Register
        (0x0004 => pub PMR: ReadWrite<u32, PMR::Register>),
        /// VM Binary Point Register
        (0x0008 => pub BPR: ReadWrite<u32, BPR::Register>),
        /// VM Interrupt Acknowledge Register
        (0x000c => pub IAR: ReadOnly<u32, IAR::Register>),
        /// VM End of Interrupt Register
        (0x0010 => pub EOIR: WriteOnly<u32, EOIR::Register>),
        /// VM Running Priority Register
        (0x0014 => pub RPR: ReadOnly<u32, RPR::Register>),
        /// VM Highest Priority Pending Interrupt Register
        (0x0018 => pub HPPIR: ReadOnly<u32, HPPIR::Register>),
        /// VM Aliased Binary Point Register
        (0x001c => pub ABPR: ReadWrite<u32, ABPR::Register>),
        /// VM Aliased Interrupt Acknowledge Register
        (0x0020 => pub AIAR: ReadOnly<u32, AIAR::Register>),
        /// VM Aliased End of Interrupt Register
        (0x0024 => pub AEOIR: WriteOnly<u32, AEOIR::Register>),
        /// VM Aliased Highest Priority Pending Interrupt Register
        (0x0028 => pub AHPPIR: ReadOnly<u32, AHPPIR::Register>),
        (0x002c => _reserved_1),
        /// VM Active Priorities Registers
        (0x00d0 => pub APR: [ReadWrite<u32>; 4]),
        (0x00e0 => _reserved_2),
        /// VM CPU Interface Identification Register
        (0x00fc => pub IIDR: ReadOnly<u32>),
        (0x0100 => _reserved_3),
        /// VM Deactivate Interrupt Register
        (0x1000 => pub DIR: WriteOnly<u32, DIR::Register>),
        (0x1004 => @END),
    }
}

register_bitfields! [
    u32,
    /// Virtual Machine Control Register
    pub CTLR [
        /// Enable Group 0 virtual interrupts
        EnableGrp0 OFFSET(0) NUMBITS(1) [],
        /// Enable Group 1 virtual interrupts
        EnableGrp1 OFFSET(1) NUMBITS(1) [],
        /// Acknowledge control for Group 1 virtual interrupts
        AckCtl OFFSET(2) NUMBITS(1) [],
        /// FIQ enable for Group 0 virtual interrupts
        FIQEn OFFSET(3) NUMBITS(1) [],
        /// Common binary point register
        CBPR OFFSET(4) NUMBITS(1) [],
        /// EOI mode control
        EOImode OFFSET(9) NUMBITS(1) [],
    ],

    /// VM Priority Mask Register
    pub PMR [
        /// Priority mask (bits [7:3] only, bits [2:0] are reserved)
        Priority OFFSET(3) NUMBITS(5) [],
    ],

    /// VM Binary Point Register
    pub BPR [
        /// Binary point
        BinaryPoint OFFSET(0) NUMBITS(3) [],
    ],

    /// VM Interrupt Acknowledge Register
    pub IAR [
        /// Interrupt ID
        InterruptID OFFSET(0) NUMBITS(10) [],
        /// CPU ID (for SGIs)
        CPUID OFFSET(10) NUMBITS(3) [],
    ],

    /// VM End of Interrupt Register
    pub EOIR [
        /// End of interrupt ID
        EOIINTID OFFSET(0) NUMBITS(10) [],
        /// CPU ID (for SGIs)
        CPUID OFFSET(10) NUMBITS(3) [],
    ],

    /// VM Running Priority Register
    pub RPR [
        /// Priority
        Priority OFFSET(0) NUMBITS(8) [],
    ],

    /// VM Highest Priority Pending Interrupt Register
    pub HPPIR [
        /// Pending interrupt ID
        PENDINTID OFFSET(0) NUMBITS(10) [],
        /// CPU ID (for SGIs)
        CPUID OFFSET(10) NUMBITS(3) [],
    ],

    /// VM Aliased Binary Point Register
    pub ABPR [
        /// Binary point
        BinaryPoint OFFSET(0) NUMBITS(3) [],
    ],

    /// VM Aliased Interrupt Acknowledge Register
    pub AIAR [
        /// Interrupt ID
        InterruptID OFFSET(0) NUMBITS(10) [],
        /// CPU ID (for SGIs)
        CPUID OFFSET(10) NUMBITS(3) [],
    ],

    /// VM Aliased End of Interrupt Register
    pub AEOIR [
        /// End of interrupt ID
        EOIINTID OFFSET(0) NUMBITS(10) [],
        /// CPU ID (for SGIs)
        CPUID OFFSET(10) NUMBITS(3) [],
    ],

    /// VM Aliased Highest Priority Pending Interrupt Register
    pub AHPPIR [
        /// Pending interrupt ID
        PENDINTID OFFSET(0) NUMBITS(10) [],
        /// CPU ID (for SGIs)
        CPUID OFFSET(10) NUMBITS(3) [],
    ],

    /// VM Deactivate Interrupt Register
    pub DIR [
        /// Interrupt ID
        InterruptID OFFSET(0) NUMBITS(10) [],
        /// CPU ID (for SGIs)
        CPUID OFFSET(10) NUMBITS(3) [],
    ],
];
