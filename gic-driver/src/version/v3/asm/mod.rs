#[macro_use]
mod macros;

// 使用宏定义读写寄存器
define_readwrite_register! {
    ICC_SRE_EL1 {
        SRE OFFSET(0) NUMBITS(1) [],
        DFB OFFSET(1) NUMBITS(1) [],
        DIB OFFSET(2) NUMBITS(1) [],
    }
}

define_readwrite_register! {
    ICC_SRE_EL2 {
        SRE OFFSET(0) NUMBITS(1) [],
        DFB OFFSET(1) NUMBITS(1) [],
        DIB OFFSET(2) NUMBITS(1) [],
        ENABLE OFFSET(3) NUMBITS(1) [],
    }
}

define_readwrite_register! {
    ICC_IGRPEN0_EL1 {
        ENABLE OFFSET(0) NUMBITS(1) [],
    }
}

define_readwrite_register! {
    ICC_IGRPEN1_EL1 {
        ENABLE OFFSET(0) NUMBITS(1) [],
    }
}

define_readonly_register! {
    ICC_IAR1_EL1 {
        INTID OFFSET(0) NUMBITS(24) [],
    }
}

define_readonly_register! {
    ICC_EOIR1_EL1 {
        INTID OFFSET(0) NUMBITS(24) [],
    }
}