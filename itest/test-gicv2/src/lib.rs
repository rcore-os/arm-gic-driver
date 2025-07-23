#![no_std]
#![cfg(target_os = "none")]

use log::{debug, info};
use test_base::*;

#[somehal::entry]
fn main(_args: &somehal::BootInfo) -> ! {
    test_base::init_test();
    let binding = fdt();
    let gicv2_node = binding
        .find_compatible(&["arm,gic-400", "arm,gic-v2", "arm,cortex-a15-gic"])
        .next()
        .expect("GICv2 node not found in FDT");

    debug!("GICv2 node: {:?}", gicv2_node.name());

    info!("{TEST_SUCCESS}");
}
