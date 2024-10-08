# arm-gic-driver

## 介绍

arm gic 通用驱动，支持 v1-4

## 使用说明

```rust
let mut v2 = GicV2::new(gicd, gicc).unwrap();
v2.enable_irq(irq_num);
let mut v3 = GicV3::new(gicd, gicr).unwrap();
v3.enable_irq(irq_num);
```
