#![no_std]
#![cfg(target_os = "none")]

use arm_gic_driver::v3::*;
use log::{debug, info};
use spin::{Mutex, Once};
use test_base::{somehal::mem::iomap, *};
static GIC: Mutex<Gic> = Mutex::new(unsafe { Gic::new(VirtAddr::new(0), VirtAddr::new(0)) });
static CPU_IF: Mutex<Option<CpuInterface>> = Mutex::new(None);

static TRAP_OP: Once<TrapOp> = Once::new();

#[somehal::entry]
fn main(_args: &somehal::BootInfo) -> ! {
    test_base::init_test();
    test_base::test_suit::set_test_interface(&CpuImpl);
    init_gic();

    test_suit::ppi::test_irq();
    test_suit::sgi::test_to_current_cpu();

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

    let mut gic = unsafe { Gic::new(gicd_base.into(), gicc_base.into()) };

    gic.init();
    let mut cpu = gic.cpu_interface();
    cpu.init_current_cpu().unwrap();
    // cpu.set_eoi_mode_ns(false);
    TRAP_OP.call_once(|| cpu.trap_operations());
    *GIC.lock() = gic;
    CPU_IF.lock().replace(cpu);

    // 启用CPU全局中断
    unsafe {
        core::arch::asm!("msr daifclr, #2"); // 清除IRQ mask (bit 1)
    }
    debug!("Global interrupts enabled");
}

#[somehal::irq_handler]
fn irq_handler() {
    // debug!("IRQ handler invoked");
    let ack = trap().ack1();

    debug!("Handling IRQ: {ack:?}");

    if handle_list(ack).is_some() {
        panic!("Unhandled IRQ: {ack:?}");
    }

    if !ack.is_special() {
        trap().eoi1(ack);
        if trap().eoi_mode() {
            trap().dir(ack);
        }
    }
}

fn trap() -> &'static TrapOp {
    TRAP_OP.wait()
}

// 返回None表示中断已处理
fn handle_list(intid: IntId) -> Option<()> {
    test_suit::ppi::handle(intid)?;
    test_suit::sgi::handle(intid, None)?;
    Some(())
}

struct CpuImpl;

impl test_base::test_suit::TestIf for CpuImpl {
    fn set_irq_enable(&self, intid: IntId, enable: bool) {
        let cpu_if = CPU_IF.lock();
        if let Some(cpu) = cpu_if.as_ref() {
            cpu.set_irq_enable(intid, enable);
        } else {
            panic!("CPU interface not initialized");
        }
    }

    fn set_priority(&self, intid: IntId, priority: u8) {
        let cpu_if = CPU_IF.lock();
        if let Some(cpu) = cpu_if.as_ref() {
            cpu.set_priority(intid, priority);
        } else {
            panic!("CPU interface not initialized");
        }
    }

    fn is_irq_enable(&self, intid: IntId) -> bool {
        let cpu_if = CPU_IF.lock();
        if let Some(cpu) = cpu_if.as_ref() {
            cpu.is_irq_enable(intid)
        } else {
            panic!("CPU interface not initialized");
        }
    }

    fn sgi_to_current(&self, intid: IntId) {
        let c = CPU_IF.lock();
        c.as_ref().unwrap().send_sgi(intid, SGITarget::current());
    }
}
