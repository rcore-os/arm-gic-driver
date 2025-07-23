#![no_std]
#![cfg(target_os = "none")]

use core::ptr::null_mut;
use core::sync::atomic::{AtomicBool, Ordering};

use arm_gic_driver::v2;
use log::{debug, info};
use spin::Mutex;
use test_base::{somehal::mem::iomap, *};

static GIC: Mutex<v2::Gic> = Mutex::new(unsafe { v2::Gic::new(null_mut(), null_mut()) });
static CPU_IF: Mutex<Option<v2::CpuInterface>> = Mutex::new(None);
static TIMER_INTERRUPT_FIRED: AtomicBool = AtomicBool::new(false);
static SGI_INTERRUPT_FIRED: AtomicBool = AtomicBool::new(false);

#[somehal::entry]
fn main(_args: &somehal::BootInfo) -> ! {
    test_base::init_test();
    init_gic();
    info!("test_systice_irq");
    test_systice_irq();
    info!("test_systice_irq done");
    test_sgi_to_current_cpu_irq();
    info!("test_sgi_irq done");

    info!("{TEST_SUCCESS}");
}

fn init_gic() {
    let binding = fdt();
    let gicv2_node = binding
        .find_compatible(&["arm,gic-400", "arm,gic-v2", "arm,cortex-a15-gic"])
        .next()
        .expect("GICv2 node not found in FDT");

    let mut regs = gicv2_node.reg().unwrap();
    let gicd_base = regs.next().expect("GICD base address not found");
    let gicc_base = regs.next().expect("GICC base address not found");
    let gich_base = regs.next().expect("GICH base address not found");
    let gicv_base = regs.next().expect("GICV base address not found");

    debug!("GICv2 node: {:?}", gicv2_node.name());
    debug!(
        "GICD base: {:#x}, GICC base: {:#x}",
        gicd_base.address, gicc_base.address
    );

    let gicd_base = iomap(gicd_base.address as _, gicd_base.size.unwrap_or_default())
        .expect("Failed to map GICD base address");
    let gicc_base = iomap(gicc_base.address as _, gicc_base.size.unwrap_or_default())
        .expect("Failed to map GICC base address");
    let gich_base = iomap(gich_base.address as _, gich_base.size.unwrap_or_default())
        .expect("Failed to map GICH base address");
    let gicv_base = iomap(gicv_base.address as _, gicv_base.size.unwrap_or_default())
        .expect("Failed to map GICV base address");

    let mut gic = unsafe { v2::Gic::new(gicd_base.as_ptr(), gicc_base.as_ptr()) };

    gic.init();
    debug!("GICv2 initialized successfully");
    let cpu = gic.init_cpu_interface();
    cpu.set_eoi_mode_ns(false);
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

fn test_systice_irq() {
    // ARM Generic Timer的非安全物理定时器中断 PPI 30
    let timer_irq = arm_gic_driver::IntId::ppi(14); // PPI 30 - 16 = 14

    debug!("Testing system timer interrupt: {timer_irq:?}");

    // 重置全局标志
    TIMER_INTERRUPT_FIRED.store(false, Ordering::SeqCst);

    // 配置定时器中断
    {
        let cpu_if = CPU_IF.lock();
        let cpu = cpu_if.as_ref().unwrap();

        // 设置中断优先级 (较高优先级)
        cpu.set_priority(timer_irq, 0x80);
        debug!("Set timer interrupt priority to 0x80");

        // 启用定时器中断
        cpu.irq_enable(timer_irq);
        debug!("Enabled timer interrupt");

        // 检查中断是否已启用
        let enabled = cpu.irq_is_enabled(timer_irq);
        debug!("Timer interrupt enabled: {enabled}");
        assert!(enabled, "Timer interrupt should be enabled");
    }

    // 配置ARM Generic Timer - 设置1ms后触发中断
    unsafe {
        // 禁用定时器
        core::arch::asm!("msr cntp_ctl_el0, {}", in(reg) 0u64);

        // 读取定时器频率
        let timer_freq: u64;
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) timer_freq);
        debug!("Timer frequency: {timer_freq} Hz");

        // 设置定时器值为1ms后触发
        let timeout_ticks = timer_freq / 1000; // 1ms
        core::arch::asm!("msr cntp_tval_el0, {}", in(reg) timeout_ticks);

        // 启用定时器 (bit 0 = enable, bit 1 = interrupt mask, bit 2 = status)
        // 设置为 0x1 表示启用定时器且不屏蔽中断
        core::arch::asm!("msr cntp_ctl_el0, {}", in(reg) 1u64);

        debug!("Configured generic timer for 1ms timeout ({timeout_ticks} ticks)");
    }

    // 等待中断触发 - 循环等待2ms
    debug!("Waiting for timer interrupt (2ms timeout)...");

    let start_time = unsafe {
        let counter: u64;
        core::arch::asm!("mrs {}, cntpct_el0", out(reg) counter);
        counter
    };

    let timer_freq: u64 = unsafe {
        let freq: u64;
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) freq);
        freq
    };

    let timeout_duration = timer_freq / 500; // 2ms

    loop {
        let current_time = unsafe {
            let counter: u64;
            core::arch::asm!("mrs {}, cntpct_el0", out(reg) counter);
            counter
        };

        // 检查中断是否已触发
        if TIMER_INTERRUPT_FIRED.load(Ordering::SeqCst) {
            debug!("Timer interrupt successfully fired!");
            break;
        }

        // 检查是否超时 (2ms)
        if current_time.wrapping_sub(start_time) > timeout_duration {
            // 清理：禁用定时器
            unsafe {
                core::arch::asm!("msr cntp_ctl_el0, {}", in(reg) 0u64);
            }

            // 禁用中断
            {
                let cpu_if = CPU_IF.lock();
                let cpu = cpu_if.as_ref().unwrap();
                cpu.irq_disable(timer_irq);
            }

            panic!("Timer interrupt test failed: interrupt did not fire within 2ms");
        }

        // 短暂延迟
        core::hint::spin_loop();
    }

    // 清理：禁用定时器
    unsafe {
        core::arch::asm!("msr cntp_ctl_el0, {}", in(reg) 0u64);
        debug!("Disabled timer");
    }

    // 禁用中断
    {
        let cpu_if = CPU_IF.lock();
        let cpu = cpu_if.as_ref().unwrap();
        cpu.irq_disable(timer_irq);
        debug!("Disabled timer interrupt");
    }

    debug!("Timer interrupt test completed successfully");
}

fn test_sgi_to_current_cpu_irq() {
    // 使用 SGI 0 (Software Generated Interrupt 0)
    let sgi_irq = arm_gic_driver::IntId::sgi(0);

    debug!("Testing SGI to current CPU: {sgi_irq:?}");

    // 重置全局标志
    SGI_INTERRUPT_FIRED.store(false, Ordering::SeqCst);

    // 配置SGI中断
    {
        let cpu_if = CPU_IF.lock();
        let cpu = cpu_if.as_ref().unwrap();

        // 设置中断优先级
        cpu.set_priority(sgi_irq, 0x80);
        debug!("Set SGI interrupt priority to 0x80");

        // 启用SGI中断
        cpu.irq_enable(sgi_irq);
        debug!("Enabled SGI interrupt");

        // 检查中断是否已启用
        let enabled = cpu.irq_is_enabled(sgi_irq);
        debug!("SGI interrupt enabled: {enabled}");
        assert!(enabled, "SGI interrupt should be enabled");
    }

    // 发送SGI到当前CPU
    {
        let gic = GIC.lock();
        debug!("Sending SGI 1 to current CPU...");
        gic.send_sgi(1, v2::SGITarget::Current);
        debug!("SGI sent successfully");
    }

    // 等待SGI中断触发 - 循环等待2ms
    debug!("Waiting for SGI interrupt (2ms timeout)...");

    let start_time = unsafe {
        let counter: u64;
        core::arch::asm!("mrs {}, cntpct_el0", out(reg) counter);
        counter
    };

    let timer_freq: u64 = unsafe {
        let freq: u64;
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) freq);
        freq
    };

    let timeout_duration = timer_freq / 500; // 2ms

    loop {
        let current_time = unsafe {
            let counter: u64;
            core::arch::asm!("mrs {}, cntpct_el0", out(reg) counter);
            counter
        };

        // 检查SGI中断是否已触发
        if SGI_INTERRUPT_FIRED.load(Ordering::SeqCst) {
            debug!("SGI interrupt successfully fired!");
            break;
        }

        // 检查是否超时 (2ms)
        if current_time.wrapping_sub(start_time) > timeout_duration {
            // 禁用中断
            {
                let cpu_if = CPU_IF.lock();
                let cpu = cpu_if.as_ref().unwrap();
                cpu.irq_disable(sgi_irq);
            }

            panic!("SGI interrupt test failed: interrupt did not fire within 2ms");
        }

        // 短暂延迟
        core::hint::spin_loop();
    }

    // 禁用SGI中断
    {
        let cpu_if = CPU_IF.lock();
        let cpu = cpu_if.as_ref().unwrap();
        cpu.irq_disable(sgi_irq);
        debug!("Disabled SGI interrupt");
    }

    debug!("SGI interrupt test completed successfully");
}

#[somehal::irq_handler]
fn irq_handler() {
    // debug!("IRQ handler invoked");
    let g = CPU_IF.lock();
    let cpu = g.as_ref().unwrap();
    let ack = cpu.ack();

    if let Some(irq) = ack {
        debug!("Handling IRQ: {irq:?}");

        // 检查中断类型
        match irq {
            v2::Ack::Normal(intid) if intid == arm_gic_driver::IntId::ppi(14) => {
                debug!("Timer interrupt received!");
                TIMER_INTERRUPT_FIRED.store(true, Ordering::SeqCst);

                // 禁用定时器以防止重复中断
                unsafe {
                    core::arch::asm!("msr cntp_ctl_el0, {}", in(reg) 0u64);
                }
            }
            v2::Ack::SGI { intid, cpu_id } => {
                if intid != arm_gic_driver::IntId::sgi(1) {
                    panic!("Unexpected SGI interrupt: {intid:?}");
                }
                debug!("SGI interrupt received from CPU {cpu_id}!");
                SGI_INTERRUPT_FIRED.store(true, Ordering::SeqCst);
            }
            _ => {
                debug!("Other interrupt received: {irq:?}");
            }
        }

        cpu.eoi(irq);
        if cpu.eoi_mode_ns() {
            cpu.dir(irq);
        }
    } else {
        debug!("No IRQ to handle");
    }
}
