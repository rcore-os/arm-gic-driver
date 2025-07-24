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
/// - `false` GICC_EOIR has both priority drop and deactivate interrupt functionality. Accesses to the GICC_DIR are UNPREDICTABLE.
/// - `true`  GICC_EOIR has priority drop functionality only. GICC_DIR has deactivate interrupt functionality.
pub fn set_eoi_mode_ns(is_two_step: bool) {
   
}
