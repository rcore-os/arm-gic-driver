# arm-gic-driver

## 介绍

arm gic 通用驱动，支持 v1-4

## 使用说明

```rust
let mut gic = GicV3::new(gicd, gicr).unwrap();
gic.enable_irq(irq_num);
```
