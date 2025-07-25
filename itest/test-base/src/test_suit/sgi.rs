use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use aarch64_cpu::registers::*;
use arm_gic_driver::IntId;
use log::debug;

use crate::test_suit::test_if;

static SGI_INTERRUPT_FIRED: AtomicBool = AtomicBool::new(false);
static SGI_SEND_CPU: AtomicU64 = AtomicU64::new(0);

const SGI_IRQ: IntId = IntId::sgi(1); // 使用SGI 1

pub fn test_to_current_cpu() {
    debug!("Testing SGI to current CPU: {SGI_IRQ:?}");

    // 重置全局标志
    SGI_INTERRUPT_FIRED.store(false, Ordering::SeqCst);

    // 配置SGI中断
    {
        // 设置中断优先级
        test_if().set_priority(SGI_IRQ, 0x80);
        debug!("Set SGI interrupt priority to 0x80");

        // 启用SGI中断
        test_if().set_irq_enable(SGI_IRQ, true);
        debug!("Enabled SGI interrupt");

        // 检查中断是否已启用
        let enabled = test_if().is_irq_enable(SGI_IRQ);
        debug!("SGI interrupt enabled: {enabled}");
        assert!(enabled, "SGI interrupt should be enabled");
    }

    // 发送SGI到当前CPU
    let cpuid = MPIDR_EL1.get();
    SGI_SEND_CPU.store(cpuid & 0xFFFFFF, Ordering::SeqCst);

    debug!("Sending SGI to current CPU...");
    test_if().sgi_to_current(SGI_IRQ);
    debug!("SGI sent successfully");

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
            test_if().set_irq_enable(SGI_IRQ, false);

            panic!("SGI interrupt test failed: interrupt did not fire within 2ms");
        }

        // 短暂延迟
        core::hint::spin_loop();
    }

    // 禁用SGI中断
    test_if().set_irq_enable(SGI_IRQ, false);
    debug!("Disabled SGI interrupt");

    debug!("SGI interrupt test completed successfully");
}

pub fn handle(intid: IntId, from_cpu: Option<usize>) -> Option<()> {
    if intid != SGI_IRQ {
        return Some(()); // 不是预期的PPI中断
    }

    if let Some(cpu_id) = from_cpu {
        let expected_cpu = SGI_SEND_CPU.load(Ordering::SeqCst);
        if cpu_id != expected_cpu as usize {
            panic!("Received SGI on CPU {cpu_id}, expected CPU {expected_cpu}");
        }
    }

    // 处理PPI中断
    debug!("Handling SGI interrupt");

    // 设置标志表示中断已触发
    SGI_INTERRUPT_FIRED.store(true, Ordering::SeqCst);
    None
}
