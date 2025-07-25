#![no_std]
#![cfg(target_os = "none")]

use core::ptr::NonNull;

pub use fdt_parser;
use fdt_parser::Fdt;
use log::info;
pub use somehal;
use somehal::{boot_info, mem::phys_to_virt};

use crate::logger::init_log;

pub mod lang_items;
mod logger;
mod mem;

pub const TEST_SUCCESS: &str = "All tests passed!";

pub fn init_test() {
    init_log();
    info!("boot_info: {:#?}", boot_info());
    mem::init_this();
    info!("begin test");
}

pub fn fdt() -> Fdt<'static> {
    let ptr = phys_to_virt(boot_info().fdt.unwrap().as_ptr() as usize);
    Fdt::from_ptr(NonNull::new(ptr).unwrap()).expect("Failed to parse FDT")
}
