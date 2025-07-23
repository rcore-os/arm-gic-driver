#![no_std]
#![cfg(target_os = "none")]

use arm_gic_driver::v2;
use log::{debug, info};
use test_base::{somehal::mem::iomap, *};

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

    let gicd_base = iomap(gicc_base.address as _, gicd_base.size.unwrap_or_default())
        .expect("Failed to map GICD base address");
    let gicc_base = iomap(gicc_base.address as _, gicc_base.size.unwrap_or_default())
        .expect("Failed to map GICC base address");

    let gic = unsafe { v2::Gic::new(gicd_base.as_ptr(), gicc_base.as_ptr()) };

    

    info!("{TEST_SUCCESS}");
}
