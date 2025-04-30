# arm-gic-driver

## 介绍

arm gic 通用驱动，支持 v1-4

## 使用说明

```rust
use arm_gic_driver::*;

let mut v2 = v2::Gic::new(gicd, gicc).unwrap();
v2.enable_irq(irq_num);

let mut v3 = v3::Gic::new(gicd, gicr).unwrap();
v3.enable_irq(irq_num);
let mut cpuif = v3.cpu_interface();
let intid = cpuif.ack();
cpuif.eoi(intid);
```
