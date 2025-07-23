#![no_std]
#![cfg(target_os = "none")]

use core::ptr::null_mut;

use arm_gic_driver::v2;
use log::{debug, info};
use spin::Mutex;
use test_base::{somehal::mem::iomap, *};

static GIC: Mutex<v2::Gic> = Mutex::new(unsafe { v2::Gic::new(null_mut(), null_mut()) });
static CPU_IF: Mutex<Option<v2::CpuInterface>> = Mutex::new(None);

#[somehal::entry]
fn main(_args: &somehal::BootInfo) -> ! {
    test_base::init_test();
    let binding = fdt();
    let gicv2_node = binding
        .find_compatible(&["arm,gic-400", "arm,gic-v2", "arm,cortex-a15-gic"])
        .next()
        .expect("GICv2 node not found in FDT");

    let mut regs = gicv2_node.reg().unwrap();
    let gicd_base = regs.next().expect("GICD base address not found");
    let gicc_base = regs.next().expect("GICC base address not found");

    debug!("GICv2 node: {:?}", gicv2_node.name());
    debug!(
        "GICD base: {:#x}, GICC base: {:#x}",
        gicd_base.address, gicc_base.address
    );

    let gicd_base = iomap(gicd_base.address as _, gicd_base.size.unwrap_or_default())
        .expect("Failed to map GICD base address");
    let gicc_base = iomap(gicc_base.address as _, gicc_base.size.unwrap_or_default())
        .expect("Failed to map GICC base address");

    let mut gic = unsafe { v2::Gic::new(gicd_base.as_ptr(), gicc_base.as_ptr()) };

    gic.init();
    debug!("GICv2 initialized successfully");
    let cpu = gic.init_cpu_interface();
    cpu.set_eoi_mode_ns(false);
    {
        *GIC.lock() = gic;
        CPU_IF.lock().replace(cpu);
    }

    info!("{TEST_SUCCESS}");
}

#[somehal::irq_handler]
fn irq_handler() {
    debug!("IRQ handler invoked");
    let g = CPU_IF.lock();
    let cpu = g.as_ref().unwrap();
    let ack = cpu.ack();

    if let Some(irq) = ack {
        debug!("Handling IRQ: {irq:?}");
        cpu.eoi(irq);
        if cpu.eoi_mode_ns() {
            cpu.dir(irq);
        }
    } else {
        debug!("No IRQ to handle");
    }
}
