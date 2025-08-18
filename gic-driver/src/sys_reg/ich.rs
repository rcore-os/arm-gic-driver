// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// ICH (Interrupt Controller Hypervisor) System registers

// Active Priority Group 0 寄存器 (EL2)
define_readwrite_register! {
    ICH_AP0R0_EL2 {
        ACTIVE OFFSET(0) NUMBITS(32) [],
    }
}

define_readwrite_register! {
    ICH_AP0R1_EL2 {
        ACTIVE OFFSET(0) NUMBITS(32) [],
    }
}

define_readwrite_register! {
    ICH_AP0R2_EL2 {
        ACTIVE OFFSET(0) NUMBITS(32) [],
    }
}

define_readwrite_register! {
    ICH_AP0R3_EL2 {
        ACTIVE OFFSET(0) NUMBITS(32) [],
    }
}

// Active Priority Group 1 寄存器 (EL2)
define_readwrite_register! {
    ICH_AP1R0_EL2 {
        NMI OFFSET(63) NUMBITS(1) [],
        ACTIVE OFFSET(0) NUMBITS(32) [],
    }
}

define_readwrite_register! {
    ICH_AP1R1_EL2 {
        ACTIVE OFFSET(0) NUMBITS(32) [],
    }
}

define_readwrite_register! {
    ICH_AP1R2_EL2 {
        ACTIVE OFFSET(0) NUMBITS(32) [],
    }
}

define_readwrite_register! {
    ICH_AP1R3_EL2 {
        ACTIVE OFFSET(0) NUMBITS(32) [],
    }
}

// Hypervisor Control Register
define_readwrite_register! {
    ICH_HCR_EL2 {
        EN OFFSET(0) NUMBITS(1) [],
        UIE OFFSET(1) NUMBITS(1) [],
        LRENPIE OFFSET(2) NUMBITS(1) [],
        NPIE OFFSET(3) NUMBITS(1) [],
        VGRP0EIE OFFSET(4) NUMBITS(1) [],
        VGRP0DIE OFFSET(5) NUMBITS(1) [],
        VGRP1EIE OFFSET(6) NUMBITS(1) [],
        VGRP1DIE OFFSET(7) NUMBITS(1) [],
        VSGIEOICOUNT OFFSET(8) NUMBITS(1) [],
        TC OFFSET(10) NUMBITS(1) [],
        TALL0 OFFSET(11) NUMBITS(1) [],
        TALL1 OFFSET(12) NUMBITS(1) [],
        TSEI OFFSET(13) NUMBITS(1) [],
        TDIR OFFSET(14) NUMBITS(1) [],
        DVIM OFFSET(15) NUMBITS(1) [],
        EOICOUNT OFFSET(27) NUMBITS(5) [],
    }
}

// VGIC Type Register
define_readonly_register! {
    ICH_VTR_EL2 {
        LISTREGS OFFSET(0) NUMBITS(4) [],
        TDS OFFSET(19) NUMBITS(1) [],
        NV4 OFFSET(20) NUMBITS(1) [],
        A3V OFFSET(21) NUMBITS(1) [],
        SEIS OFFSET(22) NUMBITS(1) [],
        IDBITS OFFSET(23) NUMBITS(4) [],
        PREBITS OFFSET(26) NUMBITS(3) [],
        PRIBITS OFFSET(29) NUMBITS(3) [],
    }
}

// Maintenance Interrupt Status Register
define_readonly_register! {
    ICH_MISR_EL2 {
        EOI OFFSET(0) NUMBITS(1) [],
        U OFFSET(1) NUMBITS(1) [],
        LRENP OFFSET(2) NUMBITS(1) [],
        NP OFFSET(3) NUMBITS(1) [],
        VGRP0E OFFSET(4) NUMBITS(1) [],
        VGRP0D OFFSET(5) NUMBITS(1) [],
        VGRP1E OFFSET(6) NUMBITS(1) [],
        VGRP1D OFFSET(7) NUMBITS(1) [],
    }
}

// End of Interrupt Status Register
define_readonly_register! {
    ICH_EISR_EL2 {
        STATUS OFFSET(0) NUMBITS(16) [],
    }
}

// Empty List Register Status Register
define_readonly_register! {
    ICH_ELRSR_EL2 {
        STATUS OFFSET(0) NUMBITS(16) [],
    }
}

// Virtual Machine Control Register
define_readwrite_register! {
    ICH_VMCR_EL2 {
        VENG0 OFFSET(0) NUMBITS(1) [],
        VENG1 OFFSET(1) NUMBITS(1) [],
        VACKCTL OFFSET(2) NUMBITS(1) [],
        VFIQEN OFFSET(3) NUMBITS(1) [],
        VCBPR OFFSET(4) NUMBITS(1) [],
        VEOIM OFFSET(9) NUMBITS(1) [],
        VBPR1 OFFSET(18) NUMBITS(3) [],
        VBPR0 OFFSET(21) NUMBITS(3) [],
        VPMR OFFSET(24) NUMBITS(8) [],
    }
}

// List Registers - 16 registers from LR0 to LR15
define_readwrite_register! {
    ICH_LR0_EL2 {
        VINTID OFFSET(0) NUMBITS(32) [],
        STATE OFFSET(62) NUMBITS(2) [],
        HW OFFSET(61) NUMBITS(1) [],
        GROUP OFFSET(60) NUMBITS(1) [],
        NMI OFFSET(59) NUMBITS(1) [],
        PRIORITY OFFSET(48) NUMBITS(8) [],
        PINTID OFFSET(32) NUMBITS(16) [],
    }
}

define_readwrite_register! {
    ICH_LR1_EL2 {
        VINTID OFFSET(0) NUMBITS(32) [],
        STATE OFFSET(62) NUMBITS(2) [],
        HW OFFSET(61) NUMBITS(1) [],
        GROUP OFFSET(60) NUMBITS(1) [],
        NMI OFFSET(59) NUMBITS(1) [],
        PRIORITY OFFSET(48) NUMBITS(8) [],
        PINTID OFFSET(32) NUMBITS(16) [],
    }
}

define_readwrite_register! {
    ICH_LR2_EL2 {
        VINTID OFFSET(0) NUMBITS(32) [],
        STATE OFFSET(62) NUMBITS(2) [],
        HW OFFSET(61) NUMBITS(1) [],
        GROUP OFFSET(60) NUMBITS(1) [],
        NMI OFFSET(59) NUMBITS(1) [],
        PRIORITY OFFSET(48) NUMBITS(8) [],
        PINTID OFFSET(32) NUMBITS(16) [],
    }
}

define_readwrite_register! {
    ICH_LR3_EL2 {
        VINTID OFFSET(0) NUMBITS(32) [],
        STATE OFFSET(62) NUMBITS(2) [],
        HW OFFSET(61) NUMBITS(1) [],
        GROUP OFFSET(60) NUMBITS(1) [],
        NMI OFFSET(59) NUMBITS(1) [],
        PRIORITY OFFSET(48) NUMBITS(8) [],
        PINTID OFFSET(32) NUMBITS(16) [],
    }
}

define_readwrite_register! {
    ICH_LR4_EL2 {
        VINTID OFFSET(0) NUMBITS(32) [],
        STATE OFFSET(62) NUMBITS(2) [],
        HW OFFSET(61) NUMBITS(1) [],
        GROUP OFFSET(60) NUMBITS(1) [],
        NMI OFFSET(59) NUMBITS(1) [],
        PRIORITY OFFSET(48) NUMBITS(8) [],
        PINTID OFFSET(32) NUMBITS(16) [],
    }
}

define_readwrite_register! {
    ICH_LR5_EL2 {
        VINTID OFFSET(0) NUMBITS(32) [],
        STATE OFFSET(62) NUMBITS(2) [],
        HW OFFSET(61) NUMBITS(1) [],
        GROUP OFFSET(60) NUMBITS(1) [],
        NMI OFFSET(59) NUMBITS(1) [],
        PRIORITY OFFSET(48) NUMBITS(8) [],
        PINTID OFFSET(32) NUMBITS(16) [],
    }
}

define_readwrite_register! {
    ICH_LR6_EL2 {
        VINTID OFFSET(0) NUMBITS(32) [],
        STATE OFFSET(62) NUMBITS(2) [],
        HW OFFSET(61) NUMBITS(1) [],
        GROUP OFFSET(60) NUMBITS(1) [],
        NMI OFFSET(59) NUMBITS(1) [],
        PRIORITY OFFSET(48) NUMBITS(8) [],
        PINTID OFFSET(32) NUMBITS(16) [],
    }
}

define_readwrite_register! {
    ICH_LR7_EL2 {
        VINTID OFFSET(0) NUMBITS(32) [],
        STATE OFFSET(62) NUMBITS(2) [],
        HW OFFSET(61) NUMBITS(1) [],
        GROUP OFFSET(60) NUMBITS(1) [],
        NMI OFFSET(59) NUMBITS(1) [],
        PRIORITY OFFSET(48) NUMBITS(8) [],
        PINTID OFFSET(32) NUMBITS(16) [],
    }
}

define_readwrite_register! {
    ICH_LR8_EL2 {
        VINTID OFFSET(0) NUMBITS(32) [],
        STATE OFFSET(62) NUMBITS(2) [],
        HW OFFSET(61) NUMBITS(1) [],
        GROUP OFFSET(60) NUMBITS(1) [],
        NMI OFFSET(59) NUMBITS(1) [],
        PRIORITY OFFSET(48) NUMBITS(8) [],
        PINTID OFFSET(32) NUMBITS(16) [],
    }
}

define_readwrite_register! {
    ICH_LR9_EL2 {
        VINTID OFFSET(0) NUMBITS(32) [],
        STATE OFFSET(62) NUMBITS(2) [],
        HW OFFSET(61) NUMBITS(1) [],
        GROUP OFFSET(60) NUMBITS(1) [],
        NMI OFFSET(59) NUMBITS(1) [],
        PRIORITY OFFSET(48) NUMBITS(8) [],
        PINTID OFFSET(32) NUMBITS(16) [],
    }
}

define_readwrite_register! {
    ICH_LR10_EL2 {
        VINTID OFFSET(0) NUMBITS(32) [],
        STATE OFFSET(62) NUMBITS(2) [],
        HW OFFSET(61) NUMBITS(1) [],
        GROUP OFFSET(60) NUMBITS(1) [],
        NMI OFFSET(59) NUMBITS(1) [],
        PRIORITY OFFSET(48) NUMBITS(8) [],
        PINTID OFFSET(32) NUMBITS(16) [],
    }
}

define_readwrite_register! {
    ICH_LR11_EL2 {
        VINTID OFFSET(0) NUMBITS(32) [],
        STATE OFFSET(62) NUMBITS(2) [],
        HW OFFSET(61) NUMBITS(1) [],
        GROUP OFFSET(60) NUMBITS(1) [],
        NMI OFFSET(59) NUMBITS(1) [],
        PRIORITY OFFSET(48) NUMBITS(8) [],
        PINTID OFFSET(32) NUMBITS(16) [],
    }
}

define_readwrite_register! {
    ICH_LR12_EL2 {
        VINTID OFFSET(0) NUMBITS(32) [],
        STATE OFFSET(62) NUMBITS(2) [],
        HW OFFSET(61) NUMBITS(1) [],
        GROUP OFFSET(60) NUMBITS(1) [],
        NMI OFFSET(59) NUMBITS(1) [],
        PRIORITY OFFSET(48) NUMBITS(8) [],
        PINTID OFFSET(32) NUMBITS(16) [],
    }
}

define_readwrite_register! {
    ICH_LR13_EL2 {
        VINTID OFFSET(0) NUMBITS(32) [],
        STATE OFFSET(62) NUMBITS(2) [],
        HW OFFSET(61) NUMBITS(1) [],
        GROUP OFFSET(60) NUMBITS(1) [],
        NMI OFFSET(59) NUMBITS(1) [],
        PRIORITY OFFSET(48) NUMBITS(8) [],
        PINTID OFFSET(32) NUMBITS(16) [],
    }
}

define_readwrite_register! {
    ICH_LR14_EL2 {
        VINTID OFFSET(0) NUMBITS(32) [],
        STATE OFFSET(62) NUMBITS(2) [],
        HW OFFSET(61) NUMBITS(1) [],
        GROUP OFFSET(60) NUMBITS(1) [],
        NMI OFFSET(59) NUMBITS(1) [],
        PRIORITY OFFSET(48) NUMBITS(8) [],
        PINTID OFFSET(32) NUMBITS(16) [],
    }
}

define_readwrite_register! {
    ICH_LR15_EL2 {
        VINTID OFFSET(0) NUMBITS(32) [],
        STATE OFFSET(62) NUMBITS(2) [],
        HW OFFSET(61) NUMBITS(1) [],
        GROUP OFFSET(60) NUMBITS(1) [],
        NMI OFFSET(59) NUMBITS(1) [],
        PRIORITY OFFSET(48) NUMBITS(8) [],
        PINTID OFFSET(32) NUMBITS(16) [],
    }
}
