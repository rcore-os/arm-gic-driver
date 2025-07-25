use core::sync::atomic::{AtomicBool, Ordering};

use arm_gic_driver::IntId;
use log::*;

use crate::test_suit::test_if;

static TIMER_INTERRUPT_FIRED: AtomicBool = AtomicBool::new(false);

const SYSTIMER_IRQ: IntId = IntId::ppi(14); // 定时器中断

pub fn test_irq() {
    info!("Starting PPI interrupt test...");

    // ARM Generic Timer的非安全物理定时器中断 PPI 30
    let timer_irq = SYSTIMER_IRQ; // PPI 30 - 16 = 14

    debug!("Testing system timer interrupt: {timer_irq:?}");

    // 重置全局标志
    TIMER_INTERRUPT_FIRED.store(false, Ordering::SeqCst);

    // 配置定时器中断
    {
        // 设置中断优先级 (较高优先级)
        test_if().set_priority(timer_irq, 0x80);
        debug!("Set timer interrupt priority to 0x80");

        // 启用定时器中断
        test_if().set_irq_enable(timer_irq, true);
        debug!("Enabled timer interrupt");

        // 检查中断是否已启用
        let enabled = test_if().is_irq_enable(timer_irq);
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
            test_if().set_irq_enable(timer_irq, false);

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
    test_if().set_irq_enable(timer_irq, false);
    debug!("Disabled timer interrupt");

    debug!("PPI test completed successfully");
}

pub fn handle(intid: IntId) -> Option<()> {
    if intid != SYSTIMER_IRQ {
        return Some(()); // 不是预期的PPI中断
    }

    // 处理PPI中断
    debug!("Handling PPI interrupt");

    // 设置标志表示中断已触发
    TIMER_INTERRUPT_FIRED.store(true, Ordering::SeqCst);

    // 这里可以添加更多的中断处理逻辑
    debug!("PPI interrupt handled successfully");

    unsafe {
        // 读取定时器频率
        let timer_freq: u64;
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) timer_freq);
        debug!("Timer frequency: {timer_freq} Hz");

        // 设置定时器值为1s后触发
        let timeout_ticks = timer_freq;
        core::arch::asm!("msr cntp_tval_el0, {}", in(reg) timeout_ticks);
    }
    None
}
