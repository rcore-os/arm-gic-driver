// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// 定义 GICv3 寄存器访问宏

// /// 定义 CPU 寄存器读取宏
// macro_rules! cpu_read {
//     ($reg:expr) => {{
//         let reg: u64;
//         unsafe { core::arch::asm!(concat!("mrs {0}, ", $reg), out(reg) reg) }
//         reg
//     }};
// }

// /// 定义 CPU 寄存器写入宏
// macro_rules! cpu_write {
//     ($reg:expr, $val:expr) => {
//         unsafe { core::arch::asm!(concat!("msr ", $reg, ", {0:x}"), in(reg) $val) }
//     };
// }

macro_rules! define_readonly_register {
    (
        $(#[$attr:meta])*
        $register:ident {
            $($field:ident OFFSET($offset:expr) NUMBITS($bits:expr) $values:tt,)*
        }

    ) => {
        paste::paste! {
        $(#[$attr])*
        pub mod [<$register:lower>] {
            use tock_registers::{interfaces::*, register_bitfields};
            use core::arch::asm;

            register_bitfields! {u64,
                pub $register [
                    $($field OFFSET($offset) NUMBITS($bits) $values,)*
                ]
            }

            pub struct Reg;

            impl Readable for Reg {
                type T = u64;
                type R = $register::Register;

                #[inline(always)]
                fn get(&self) -> Self::T {
                    let reg: u64;
                    unsafe { asm!(concat!("mrs {0}, ", stringify!($register)), out(reg) reg) }
                    reg
                }
            }

            pub const $register: Reg = Reg{};
        }
        pub use  [<$register:lower>] ::$register;
    }
    };
}

/// 定义读写寄存器的宏
macro_rules! define_readwrite_register {
    (
        $(#[$attr:meta])*
        $register:ident {
            $($field:ident OFFSET($offset:expr) NUMBITS($bits:expr) $values:tt,)*
        }
    ) => {
        paste::paste! {
        $(#[$attr])*
        pub mod [<$register:lower>] {
            use tock_registers::{interfaces::*, register_bitfields};
            use core::arch::asm;

            register_bitfields! {u64,
                pub $register [
                    $($field OFFSET($offset) NUMBITS($bits) $values,)*
                ]
            }

            pub struct Reg;

            impl Readable for Reg {
                type T = u64;
                type R = $register::Register;

                #[inline(always)]
                fn get(&self) -> Self::T {
                    let reg: u64;
                    unsafe { asm!(concat!("mrs {0}, ", stringify!($register)), out(reg) reg) }
                    reg
                }
            }

            impl Writeable for Reg {
                type T = u64;
                type R = $register::Register;

                #[inline(always)]
                fn set(&self, value: Self::T) {
                    unsafe { asm!(concat!("msr ", stringify!($register), ", {0}"), in(reg) value) }
                }
            }

            pub const $register: Reg = Reg{};
        }
        pub use  [<$register:lower>] ::$register;
    }
    };
}

/// 定义读写寄存器的宏
macro_rules! define_writeonly_register {
    (
        $(#[$attr:meta])*
        $register:ident {
            $($field:ident OFFSET($offset:expr) NUMBITS($bits:expr) $values:tt,)*
        }
    ) => {
        paste::paste! {
        $(#[$attr])*
        pub mod [<$register:lower>] {
            use tock_registers::{interfaces::*, register_bitfields};
            use core::arch::asm;

            register_bitfields! {u64,
                pub $register [
                    $($field OFFSET($offset) NUMBITS($bits) $values,)*
                ]
            }

            pub struct Reg;

            impl Writeable for Reg {
                type T = u64;
                type R = $register::Register;

                #[inline(always)]
                fn set(&self, value: Self::T) {
                    unsafe { asm!(concat!("msr ", stringify!($register), ", {0}"), in(reg) value) }
                }
            }

            pub const $register: Reg = Reg{};
        }
        pub use  [<$register:lower>] ::$register;
    }
    };
}
