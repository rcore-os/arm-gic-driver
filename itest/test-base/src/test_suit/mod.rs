use arm_gic_driver::IntId;

pub mod ppi;

pub trait ICpuIf: Send + Sync {
    fn set_irq_enable(&self, intid: IntId, enable: bool);
    fn set_priority(&self, intid: IntId, priority: u8);
    fn is_irq_enable(&self, intid: IntId) -> bool;
}

struct CpuInterfaceEmpty;
impl ICpuIf for CpuInterfaceEmpty {
    fn set_irq_enable(&self, _intid: IntId, _enable: bool) {
        unimplemented!()
    }
    fn set_priority(&self, _intid: IntId, _priority: u8) {
        unimplemented!()
    }
    fn is_irq_enable(&self, _intid: IntId) -> bool {
        unimplemented!()
    }
}

static mut CPU_IF: &dyn ICpuIf = &CpuInterfaceEmpty;

pub fn set_cpu_interface(iface: &'static dyn ICpuIf) {
    unsafe {
        CPU_IF = iface;
    }
}

fn cpu_interface() -> &'static dyn ICpuIf {
    unsafe { CPU_IF }
}
