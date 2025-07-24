use tock_registers::register_bitfields;

macro_rules! cpu_read {
    ($name: expr) => {{
        let x: usize;
        unsafe {
            core::arch::asm!(concat!("mrs {}, ", $name), out(reg) x);
        }
        x
    }};
}

macro_rules! cpu_write {
    ($name: expr, $value: expr) => {{
        let x = $value;
        unsafe {
            core::arch::asm!(concat!("msr ", $name, ", {0:x}"), in(reg) x);
        }
    }};
}

/// Set the EOI mode for non-secure interrupts
///
/// - `false` ICC_EOIR1_EL1 has both priority drop and deactivate interrupt functionality. Accesses to the ICC_DIR_EL1 are UNPREDICTABLE.
/// - `true`  ICC_EOIR1_EL1 has priority drop functionality only. ICC_DIR_EL1 has deactivate interrupt functionality.
pub fn set_eoi_mode_ns(is_two_step: bool) {
    let mut val = cpu_read!("ICC_CTLR_EL1");
    if is_two_step {
        val |= 1 << 10; // Set EOImodeNS bit
    } else {
        val &= !(1 << 10); // Clear EOImodeNS bit
    }
    cpu_write!("ICC_CTLR_EL1", val);
}

/// Set the EOI mode for secure interrupts (EL3 only)
///
/// - `false` ICC_EOIR0_EL1 has both priority drop and deactivate interrupt functionality.
/// - `true`  ICC_EOIR0_EL1 has priority drop functionality only. ICC_DIR_EL1 has deactivate interrupt functionality.
pub fn set_eoi_mode_s(is_two_step: bool) {
    let mut val = cpu_read!("ICC_CTLR_EL1");
    if is_two_step {
        val |= 1 << 9; // Set EOImodeS bit
    } else {
        val &= !(1 << 9); // Clear EOImodeS bit
    }
    cpu_write!("ICC_CTLR_EL1", val);
}

/// Enable System Register Interface
pub fn enable_sre() {
    cpu_write!("ICC_SRE_EL1", 1);
}

/// Set priority mask
pub fn set_priority_mask(priority: u8) {
    cpu_write!("ICC_PMR_EL1", priority as usize);
}

/// Enable Group 0 interrupts
pub fn enable_group0() {
    cpu_write!("ICC_IGRPEN0_EL1", 1);
}

/// Enable Group 1 interrupts
pub fn enable_group1() {
    cpu_write!("ICC_IGRPEN1_EL1", 1);
}

/// Acknowledge interrupt for Group 0
pub fn acknowledge_interrupt_group0() -> u32 {
    cpu_read!("ICC_IAR0_EL1") as u32
}

/// Acknowledge interrupt for Group 1
pub fn acknowledge_interrupt_group1() -> u32 {
    cpu_read!("ICC_IAR1_EL1") as u32
}

/// End of interrupt for Group 0
pub fn end_of_interrupt_group0(intid: u32) {
    cpu_write!("ICC_EOIR0_EL1", intid as usize);
}

/// End of interrupt for Group 1
pub fn end_of_interrupt_group1(intid: u32) {
    cpu_write!("ICC_EOIR1_EL1", intid as usize);
}

/// Deactivate interrupt
pub fn deactivate_interrupt(intid: u32) {
    cpu_write!("ICC_DIR_EL1", intid as usize);
}

/// Get highest priority pending interrupt for Group 0
pub fn get_highest_priority_pending_group0() -> u32 {
    cpu_read!("ICC_HPPIR0_EL1") as u32
}

/// Get highest priority pending interrupt for Group 1
pub fn get_highest_priority_pending_group1() -> u32 {
    cpu_read!("ICC_HPPIR1_EL1") as u32
}

/// Get running priority
pub fn get_running_priority() -> u8 {
    cpu_read!("ICC_RPR_EL1") as u8
}
