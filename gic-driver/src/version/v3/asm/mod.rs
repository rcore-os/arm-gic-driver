mod macros;

use core::arch::asm;

mod sre_el1 {
    use tock_registers::{interfaces::*, register_bitfields};
    register_bitfields! {u64,
        pub ICC_SRE_EL1 [
            SRE OFFSET(0) NUMBITS(1) [],
            DFB OFFSET(1) NUMBITS(1) [],
            DIB OFFSET(2) NUMBITS(1) [],
        ]
    }

    // ICC_SRE_EL2
    register_bitfields! {u64,
        pub ICC_SRE_EL2 [
            SRE OFFSET(0) NUMBITS(1) [],
            DFB OFFSET(1) NUMBITS(1) [],
            DIB OFFSET(2) NUMBITS(1) [],
            ENABLE OFFSET(3) NUMBITS(1) [],
            // Bits [63:4] Reserved, RES0
        ]
    }

    pub struct Reg;

    impl Readable for Reg {
        type T = u64;
        type R = ICC_SRE_EL1::Register;

        fn get(&self) -> Self::T {
            cpu_read!("ICC_SRE_EL1") as u64
        }
    }
    impl Writeable for Reg {
        type T = u64;
        type R = ICC_SRE_EL1::Register;
        #[inline(always)]
        fn set(&self, value: Self::T) {
            cpu_write!("ICC_SRE_EL1", value);
        }
    }

    pub const ICC_SRE_EL1: Reg = Reg {};
}
pub use sre_el1::ICC_SRE_EL1;

mod sre_el2 {
    use tock_registers::{interfaces::*, register_bitfields};

    // ICC_SRE_EL2
    register_bitfields! {u64,
        pub ICC_SRE_EL2 [
            SRE OFFSET(0) NUMBITS(1) [],
            DFB OFFSET(1) NUMBITS(1) [],
            DIB OFFSET(2) NUMBITS(1) [],
            ENABLE OFFSET(3) NUMBITS(1) [],
            // Bits [63:4] Reserved, RES0
        ]
    }

    pub struct Reg;

    impl Readable for Reg {
        type T = u64;
        type R = ICC_SRE_EL2::Register;

        fn get(&self) -> Self::T {
            cpu_read!("ICC_SRE_EL2") as u64
        }
    }
    impl Writeable for Reg {
        type T = u64;
        type R = ICC_SRE_EL2::Register;
        #[inline(always)]
        fn set(&self, value: Self::T) {
            cpu_write!("ICC_SRE_EL2", value);
        }
    }

    pub const ICC_SRE_EL2: Reg = Reg {};
}
pub use sre_el2::ICC_SRE_EL2;

// ICC_IGRPEN1_EL0
register_bitfields! {u64,
    pub ICC_IGRPEN0_EL1 [
        ENABLE OFFSET(0) NUMBITS(1) [],
        // Bits [63:1] Reserved, RES0
    ]
}

pub struct IccIgrpen0El1;
impl Readable for IccIgrpen0El1 {
    type T = u64;
    type R = ICC_IGRPEN0_EL1::Register;
    #[inline(always)]
    fn get(&self) -> u64 {
        let reg: u64;
        unsafe { asm!("mrs {0}, ICC_IGRPEN1_EL0", out(reg) reg) }
        reg
    }
}

pub const ICC_IGRPEN0_EL1: IccIgrpen0El1 = IccIgrpen0El1;
// ICC_IGRPEN1_EL1
register_bitfields! {u64,
    pub ICC_IGRPEN1_EL1 [
        ENABLE OFFSET(0) NUMBITS(1) [],
        // Bits [63:1] Reserved, RES0
    ]
}

pub struct IccIgrpen1El1;
impl Readable for IccIgrpen1El1 {
    type T = u64;
    type R = ICC_IGRPEN1_EL1::Register;
    #[inline(always)]
    fn get(&self) -> u64 {
        let reg: u64;
        unsafe { asm!("mrs {0}, ICC_IGRPEN1_EL1", out(reg) reg) }
        reg
    }
}
impl Writeable for IccIgrpen1El1 {
    type T = u64;
    type R = ICC_IGRPEN1_EL1::Register;
    #[inline(always)]
    fn set(&self, value: Self::T) {
        let reg: u64 = value;
        unsafe { asm!("msr ICC_IGRPEN1_EL1, {0:x}", in(reg) reg) }
    }
}

pub const ICC_IGRPEN1_EL1: IccIgrpen1El1 = IccIgrpen1El1;

// ICC_IAR1_EL1
register_bitfields! {u64,
    pub ICC_IAR1_EL1 [
        INTID OFFSET(0) NUMBITS(24) [],
        // Bits [63:24] Reserved, RES0
    ]
}

pub struct IccIar1El1;
impl Readable for IccIar1El1 {
    type T = u64;
    type R = ICC_IAR1_EL1::Register;
    #[inline(always)]
    fn get(&self) -> u64 {
        let reg: u64;
        unsafe { asm!("mrs {0}, ICC_IAR1_EL1", out(reg) reg) }
        reg
    }
}

// ICC_EOIR1_EL1
register_bitfields! {u64,
    pub ICC_EOIR1_EL1BF [
        INTID OFFSET(0) NUMBITS(24) [],
        // Bits [63:24] Reserved, RES0
    ]
}

pub struct IccEoir1El1;
impl Readable for IccEoir1El1 {
    type T = u64;
    type R = ICC_EOIR1_EL1BF::Register;
    #[inline(always)]
    fn get(&self) -> u64 {
        let reg: u64;
        unsafe { asm!("mrs {0}, ICC_EOIR1_EL1", out(reg) reg) }
        reg
    }
}

pub const ICC_IAR1_EL1: IccIar1El1 = IccIar1El1;
pub const ICC_EOIR1_EL1: IccEoir1El1 = IccEoir1El1;
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// 参考 mpidr_el1.rs 和 Arm GICv3 手册，定义 GICC 相关系统寄存器。

use aarch64_cpu::registers::Writeable;
use tock_registers::{interfaces::Readable, register_bitfields};

// 需要导入 aarch64-cpu crate 的 asm 宏和寄存器访问宏

// ICC_AP0R<n>_EL1, n = 0-3
register_bitfields! {u64,
    pub ICC_AP0R_EL1BF [
        IMPL_DEFINED OFFSET(0) NUMBITS(32) [],
    ]
}

macro_rules! define_icc_ap0r_el1 {
    ($name:ident, $regstr:expr) => {
        pub struct $name;
        impl Readable for $name {
            type T = u64;
            type R = ICC_AP0R_EL1BF::Register;
            #[inline(always)]
            fn get(&self) -> u64 {
                let reg: u64;
                unsafe { asm!(concat!("mrs {0}, ", $regstr), out(reg) reg) }
                reg
            }
        }
    };
}

define_icc_ap0r_el1!(IccAp0r0El1, "ICC_AP0R0_EL1");
define_icc_ap0r_el1!(IccAp0r1El1, "ICC_AP0R1_EL1");
define_icc_ap0r_el1!(IccAp0r2El1, "ICC_AP0R2_EL1");
define_icc_ap0r_el1!(IccAp0r3El1, "ICC_AP0R3_EL1");

// ICC_AP1R<n>_EL1, n = 0-3
register_bitfields! {u64,
    pub ICC_AP1R_EL1BF [
        NMI OFFSET(63) NUMBITS(1) [],
        IMPL_DEFINED OFFSET(0) NUMBITS(32) [],
    ]
}

macro_rules! define_icc_ap1r_el1 {
    ($name:ident, $regstr:expr) => {
        pub struct $name;
        impl Readable for $name {
            type T = u64;
            type R = ICC_AP1R_EL1BF::Register;
            #[inline(always)]
            fn get(&self) -> u64 {
                let reg: u64;
                unsafe { asm!(concat!("mrs {0}, ", $regstr), out(reg) reg) }
                reg
            }
        }
    };
}

define_icc_ap1r_el1!(IccAp1r0El1, "ICC_AP1R0_EL1");
define_icc_ap1r_el1!(IccAp1r1El1, "ICC_AP1R1_EL1");
define_icc_ap1r_el1!(IccAp1r2El1, "ICC_AP1R2_EL1");
define_icc_ap1r_el1!(IccAp1r3El1, "ICC_AP1R3_EL1");

// ICC_ASGI1R_EL1
register_bitfields! {u64,
    pub ICC_ASGI1R_EL1BF [
        Aff3 OFFSET(48) NUMBITS(8) [],
        RS OFFSET(44) NUMBITS(4) [],
        IRM OFFSET(40) NUMBITS(1) [],
        Aff2 OFFSET(32) NUMBITS(8) [],
        INTID OFFSET(24) NUMBITS(4) [],
        Aff1 OFFSET(16) NUMBITS(8) [],
        TargetList OFFSET(0) NUMBITS(16) [],
    ]
}

pub struct IccAsgi1rEl1;

impl Readable for IccAsgi1rEl1 {
    type T = u64;
    type R = ICC_ASGI1R_EL1BF::Register;
    #[inline(always)]
    fn get(&self) -> u64 {
        let reg: u64;
        unsafe { asm!("mrs {0}, ICC_ASGI1R_EL1", out(reg) reg) }
        reg
    }
}

// 常量实例，便于直接使用
pub const ICC_AP0R0_EL1: IccAp0r0El1 = IccAp0r0El1;
pub const ICC_AP0R1_EL1: IccAp0r1El1 = IccAp0r1El1;
pub const ICC_AP0R2_EL1: IccAp0r2El1 = IccAp0r2El1;
pub const ICC_AP0R3_EL1: IccAp0r3El1 = IccAp0r3El1;

pub const ICC_AP1R0_EL1: IccAp1r0El1 = IccAp1r0El1;
pub const ICC_AP1R1_EL1: IccAp1r1El1 = IccAp1r1El1;
pub const ICC_AP1R2_EL1: IccAp1r2El1 = IccAp1r2El1;
pub const ICC_AP1R3_EL1: IccAp1r3El1 = IccAp1r3El1;

pub const ICC_ASGI1R_EL1_REG: IccAsgi1rEl1 = IccAsgi1rEl1;
