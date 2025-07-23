mod reg;

use core::ptr::NonNull;
use tock_registers::interfaces::*;

use reg::*;

/// GICv2 driver. (support GICv1)
pub struct Gic {
    gicd: NonNull<DistributorReg>,
    gicc: NonNull<CpuInterfaceReg>,
}

unsafe impl Send for Gic {}

impl Gic {
    /// `gicd`: Distributor register base address. `gicc`: CPU interface register base address.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided pointers are valid and point to the correct GICv2 registers.
    pub const unsafe fn new(gicd: *mut u8, gicc: *mut u8) -> Self {
        Self {
            gicd: unsafe { NonNull::new_unchecked(gicd as _) },
            gicc: unsafe { NonNull::new_unchecked(gicc as _) },
        }
    }

    fn gicd(&self) -> &DistributorReg {
        unsafe { self.gicd.as_ref() }
    }

    pub fn init_cpu_interface(&self) -> CpuInterface {
        let mut c = CpuInterface {
            gicc: self.gicc,
        };
        c.init();
        c
    }

    /// Initialize the GIC according to GICv2 specification
    /// This includes both Distributor and CPU Interface initialization
    pub fn init(&self) {
        // Initialize the Distributor
        self.gicd().init();
    }
    
    /// Enable a specific interrupt
    pub fn enable_interrupt(&self, interrupt_id: u32) {
        self.gicd().enable_interrupt(interrupt_id);
    }

    /// Disable a specific interrupt
    pub fn disable_interrupt(&self, interrupt_id: u32) {
        self.gicd().disable_interrupt(interrupt_id);
    }

    /// Set interrupt priority (0 = highest priority, 255 = lowest priority)
    pub fn set_interrupt_priority(&self, interrupt_id: u32, priority: u8) {
        self.gicd().set_interrupt_priority(interrupt_id, priority);
    }

    /// Set interrupt target CPU for SPIs (bit mask, bit 0 = CPU 0, etc.)
    pub fn set_interrupt_target(&self, interrupt_id: u32, target_cpu_mask: u8) {
        self.gicd().set_interrupt_target(interrupt_id, target_cpu_mask);
    }

    /// Configure interrupt as Group 0 (Secure) or Group 1 (Non-secure)
    pub fn set_interrupt_group(&self, interrupt_id: u32, group1: bool) {
        self.gicd().set_interrupt_group(interrupt_id, group1);
    }

    /// Send a Software Generated Interrupt (SGI) to target CPUs
    /// 
    /// # Arguments
    /// * `sgi_id` - SGI interrupt ID (0-15)
    /// * `target_list` - Target CPU list (bit mask)
    /// * `filter` - Target list filter:
    ///   - 0: Forward to CPUs listed in target_list
    ///   - 1: Forward to all CPUs except requesting CPU
    ///   - 2: Forward only to requesting CPU
    pub fn send_sgi(&self, sgi_id: u32, target_list: u8, filter: u32) {
        self.gicd().send_sgi(sgi_id, target_list, filter);
    }
}

pub struct CpuInterface{
    gicc: NonNull<CpuInterfaceReg>,
}

impl CpuInterface {
    fn gicc(&self) -> &CpuInterfaceReg {
        unsafe { self.gicc.as_ref() }
    }

    fn init(&mut self) {
        let gicc = self.gicc();
      
        // 1. Disable CPU interface first
        gicc.CTLR.set(0);
        
        // 2. Set priority mask to allow all interrupts (lowest priority)
        gicc.PMR.write(PMR::Priority.val(0xFF));
        
        // 3. Set binary point to default value (no preemption)
        gicc.BPR.write(BPR::BinaryPoint.val(0x2));
        
        // 4. Set aliased binary point for Group 1 interrupts
        gicc.ABPR.write(ABPR::BinaryPoint.val(0x3));
        
        // 5. Enable CPU interface for both Group 0 and Group 1 interrupts
        gicc.CTLR.write(
            GICC_CTLR::EnableGrp0::SET + 
            GICC_CTLR::EnableGrp1::SET +
            GICC_CTLR::FIQEn::CLEAR +      // Use IRQ for Group 0 interrupts
            GICC_CTLR::AckCtl::CLEAR       // Separate acknowledge for groups
        );
    }
    
    /// Acknowledge an interrupt and return the interrupt ID
    /// Returns the interrupt ID and source CPU ID (for SGIs)
    pub fn acknowledge_interrupt(&self) -> (u32, u32) {
        let iar = self.gicc().IAR.get();
        let interrupt_id = iar & 0x3FF;  // Bits [9:0]
        let cpu_id = (iar >> 10) & 0x7;  // Bits [12:10]
        (interrupt_id, cpu_id)
    }

    /// Signal end of interrupt processing
    pub fn end_of_interrupt(&self, interrupt_id: u32, cpu_id: u32) {
        let eoir_val = interrupt_id | (cpu_id << 10);
        self.gicc().EOIR.set(eoir_val);
    }

    /// Get the highest priority pending interrupt ID
    pub fn get_highest_priority_pending(&self) -> u32 {
        let hppir = self.gicc().HPPIR.get();
        hppir & 0x3FF  // Bits [9:0]
    }

    /// Get the current running priority
    pub fn get_running_priority(&self) -> u8 {
        (self.gicc().RPR.get() & 0xFF) as u8
    }

    /// Set the priority mask (interrupts with priority >= mask will be masked)
    pub fn set_priority_mask(&self, mask: u8) {
        self.gicc().PMR.write(PMR::Priority.val(mask as u32));
    }
}
