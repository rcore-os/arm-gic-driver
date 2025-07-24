#![no_std]
#![cfg(target_os = "none")]

use arm_gic_driver::{VirtAddr, v3};
use log::{debug, info};
use spin::Mutex;
use test_base::{somehal::mem::iomap, *};
static GIC: Mutex<v3::Gic> =
    Mutex::new(unsafe { v3::Gic::new(VirtAddr::new(0), VirtAddr::new(0)) });
static CPU_IF: Mutex<Option<v3::CpuInterface>> = Mutex::new(None);

#[somehal::entry]
fn main(_args: &somehal::BootInfo) -> ! {
    test_base::init_test();
    init_gic();

    info!("{TEST_SUCCESS}");
}

fn init_gic() {
    let binding = fdt();
    let node = binding
        .find_compatible(&["arm,gic-v3"])
        .next()
        .expect("GICv3 node not found in FDT");

    let mut regs = node.reg().unwrap();
    let gicd_base = regs.next().expect("GICD base address not found");
    let gicr_base = regs.next().expect("GICC base address not found");

    debug!("GICv3 node: {:?}", node.name());
    debug!(
        "GICD base: {:#x}, GICR base: {:#x}",
        gicd_base.address, gicr_base.address
    );

    let gicd_base = iomap(gicd_base.address as _, gicd_base.size.unwrap_or_default())
        .expect("Failed to map GICD base address");
    let gicc_base = iomap(gicr_base.address as _, gicr_base.size.unwrap_or_default())
        .expect("Failed to map GICC base address");

    let mut gic = unsafe { v3::Gic::new(gicd_base.into(), gicc_base.into()) };

    gic.init();
    let mut cpu = gic.cpu_interface();
    cpu.init_current_cpu().unwrap();
    // cpu.set_eoi_mode_ns(false);
    {
        *GIC.lock() = gic;
        CPU_IF.lock().replace(cpu);
    }

    // 启用CPU全局中断
    unsafe {
        core::arch::asm!("msr daifclr, #2"); // 清除IRQ mask (bit 1)
    }
    debug!("Global interrupts enabled");
}

#[somehal::irq_handler]
fn irq_handler() {}
