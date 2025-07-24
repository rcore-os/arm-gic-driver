use tock_registers::{interfaces::*, registers::*};

pub mod v2;
pub mod v3;

use crate::define::*;

/// 通用 trait：为一组 ReadWrite<u32> 寄存器设置某一位
trait IrqVecWriteable {
    fn set_irq_bit(&self, intid: u32);
    fn clear_irq_bit(&self, intid: u32);
}
trait IrqVecReadable {
    fn get_irq_bit(&self, intid: u32) -> bool;
}

impl IrqVecWriteable for [ReadWrite<u32>] {
    fn set_irq_bit(&self, index: u32) {
        let reg_index = (index / 32) as usize;
        let bit = 1 << (index % 32);
        // For GIC ISENABLER/ISPENDR/ISACTIVER etc, writing 1 sets the bit
        // Writing 0 has no effect, so we can safely write only the target bit
        self[reg_index].set(bit);
    }
    fn clear_irq_bit(&self, intid: u32) {
        let reg_index = (intid / 32) as usize;
        let bit = 1 << (intid % 32);
        let old = self[reg_index].get();
        if old & bit == 0 {
            return; // Already cleared
        }
        self[reg_index].set(old & !bit);
    }
}

impl IrqVecReadable for [ReadWrite<u32>] {
    fn get_irq_bit(&self, index: u32) -> bool {
        let reg_index = (index / 32) as usize;
        let bit = 1 << (index % 32);
        self[reg_index].get() & bit != 0
    }
}

/// Parse interrupt configuration from device tree.
/// Based on Linux GIC driver's gic_irq_domain_translate function.
pub fn fdt_parse_irq_config(itr: &[u32]) -> Result<IrqConfig, &'static str> {
    // Handle single parameter case (SGI)
    if itr.len() == 1 && itr[0] < 16 {
        return Ok(IrqConfig {
            id: IntId::sgi(itr[0]),
            trigger: Trigger::Edge, // SGI is always edge-triggered
        });
    }

    // Need at least 3 parameters for full specification
    if itr.len() < 3 {
        return Err("Invalid IRQ configuration: need at least 3 parameters");
    }

    // Interrupt type constants (from Linux kernel)
    const SPI: u32 = 0; // Shared Peripheral Interrupt
    const PPI: u32 = 1; // Private Peripheral Interrupt
    const ESPI: u32 = 2; // Extended SPI
    const EPPI: u32 = 3; // Extended PPI
    const LPI: u32 = 4; // Locality-specific Peripheral Interrupt
    const PARTITION: u32 = 5; // Partitioned PPI

    // Base interrupt IDs for extended interrupts
    const ESPI_BASE_INTID: u32 = 4096;
    const EPPI_BASE_INTID: u32 = 1056;

    // IRQ type sense mask (from Linux include/linux/irq.h)
    const IRQ_TYPE_NONE: u32 = 0x00000000;
    const IRQ_TYPE_EDGE_RISING: u32 = 0x00000001;
    const IRQ_TYPE_EDGE_FALLING: u32 = 0x00000002;
    const IRQ_TYPE_EDGE_BOTH: u32 = IRQ_TYPE_EDGE_RISING | IRQ_TYPE_EDGE_FALLING;
    const IRQ_TYPE_LEVEL_HIGH: u32 = 0x00000004;
    const IRQ_TYPE_LEVEL_LOW: u32 = 0x00000008;
    const IRQ_TYPE_SENSE_MASK: u32 = 0x0000000f;

    let irq_type = itr[0];
    let irq_num = itr[1];
    let irq_flags = itr[2] & IRQ_TYPE_SENSE_MASK;

    // Calculate hardware interrupt ID based on type
    let hwirq = match irq_type {
        SPI => {
            // SPI: hwirq = param[1] + 32
            SPI_RANGE.start + irq_num
        }
        PPI => {
            // PPI: hwirq = param[1] + 16
            PPI_RANGE.start + irq_num
        }
        ESPI => {
            // ESPI: hwirq = param[1] + ESPI_BASE_INTID
            ESPI_BASE_INTID + irq_num
        }
        EPPI => {
            // EPPI: hwirq = param[1] + EPPI_BASE_INTID
            EPPI_BASE_INTID + irq_num
        }
        LPI => {
            // LPI: hwirq = param[1]
            irq_num
        }
        PARTITION => {
            // Partitioned PPI: special handling
            if irq_num >= 16 {
                EPPI_BASE_INTID + irq_num - 16
            } else {
                16 + irq_num
            }
        }
        _ => {
            return Err("Invalid IRQ type");
        }
    };

    // Create IntId from hardware interrupt ID
    let intid = unsafe { IntId::raw(hwirq) };

    // Determine trigger type from flags
    let trigger = match irq_flags {
        IRQ_TYPE_EDGE_RISING | IRQ_TYPE_EDGE_FALLING | IRQ_TYPE_EDGE_BOTH => Trigger::Edge,
        IRQ_TYPE_LEVEL_HIGH | IRQ_TYPE_LEVEL_LOW => Trigger::Level,
        IRQ_TYPE_NONE if irq_type == PARTITION => {
            // Partitioned PPIs can have IRQ_TYPE_NONE, default to level
            Trigger::Level
        }
        IRQ_TYPE_NONE => {
            return Err("IRQ_TYPE_NONE is not allowed for IRQ type");
        }
        _ => {
            return Err("Invalid IRQ trigger type");
        }
    };

    Ok(IrqConfig { id: intid, trigger })
}
