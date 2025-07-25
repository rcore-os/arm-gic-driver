#!/bin/bash
cargo test --target aarch64-unknown-none-softfloat -p test-gicv3  --test test --features "hv" -- --show-output --uboot
