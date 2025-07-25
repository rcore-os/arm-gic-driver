use arm_gic_driver::IntId;

pub mod ppi;
pub mod sgi;

pub trait TestIf: Send + Sync {
    fn set_irq_enable(&self, intid: IntId, enable: bool);
    fn set_priority(&self, intid: IntId, priority: u8);
    fn is_irq_enable(&self, intid: IntId) -> bool;

    fn sgi_to_current(&self, intid: IntId);
}

struct CpuInterfaceEmpty;
impl TestIf for CpuInterfaceEmpty {
    fn set_irq_enable(&self, _intid: IntId, _enable: bool) {
        unimplemented!()
    }
    fn set_priority(&self, _intid: IntId, _priority: u8) {
        unimplemented!()
    }
    fn is_irq_enable(&self, _intid: IntId) -> bool {
        unimplemented!()
    }

    fn sgi_to_current(&self, _intid: IntId) {
        todo!()
    }
}

static mut IF: &dyn TestIf = &CpuInterfaceEmpty;

pub fn set_test_interface(iface: &'static dyn TestIf) {
    unsafe {
        IF = iface;
    }
}

fn test_if() -> &'static dyn TestIf {
    unsafe { IF }
}
